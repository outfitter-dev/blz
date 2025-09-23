//! Update command implementation

use anyhow::{Result, anyhow};
use blz_core::{
    FetchResult, Fetcher, Flavor, LlmsJson, MarkdownParser, PerformanceMetrics, SearchIndex,
    Source, Storage, build_anchors_map, compute_anchor_mappings,
};
use chrono::Utc;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tracing::info;

use crate::utils::flavor::{
    BASE_FLAVOR, FULL_FLAVOR, build_llms_json, discover_flavor_candidates, set_preferred_flavor,
};
use crate::utils::settings;
use crate::utils::settings::PreferenceScope;

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum FlavorMode {
    /// Keep current URL/flavor
    Current,
    /// Prefer best available flavor (llms-full.txt > llms.txt > others)
    Auto,
    /// Force llms-full.txt if available
    Full,
    /// Force llms.txt if available
    Txt,
}

#[derive(Clone)]
struct FlavorPlan {
    flavor: Option<Flavor>,
    flavor_id: String,
    file_name: String,
    url: String,
    existing_json: Option<LlmsJson>,
    existing_metadata: Option<Source>,
}

struct FlavorSummary {
    flavor: String,
    headings: usize,
    lines: usize,
}

impl FlavorPlan {
    fn canonical_file_name(&self) -> &str {
        self.flavor
            .map_or_else(|| self.file_name.as_str(), |flavor| flavor.file_name())
    }

    fn canonical_flavor_id(&self) -> &str {
        self.flavor
            .map_or_else(|| self.flavor_id.as_str(), |flavor| flavor.as_str())
    }
}

/// Execute update for a specific source
pub async fn execute(
    alias: &str,
    metrics: PerformanceMetrics,
    quiet: bool,
    flavor: FlavorMode,
    yes: bool,
) -> Result<()> {
    let storage = Storage::new()?;

    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .unwrap_or_else(|| alias.to_string());

    if !storage.exists_any_flavor(&canonical) {
        return Err(anyhow!("Source '{}' not found", alias));
    }

    update_source(&storage, &canonical, metrics, flavor, yes, quiet)
        .await
        .map(|_| ())
}

/// Execute update for all sources
pub async fn execute_all(
    metrics: PerformanceMetrics,
    quiet: bool,
    flavor: FlavorMode,
    yes: bool,
) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources();

    if sources.is_empty() {
        anyhow::bail!("No sources configured. Use 'blz add' to add sources.");
    }

    let mut updated_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    for alias in sources {
        match update_source(&storage, &alias, metrics.clone(), flavor, yes, quiet).await {
            Ok(true) => updated_count += 1,
            Ok(false) => skipped_count += 1,
            Err(e) => {
                if !quiet {
                    eprintln!("{}: {}", alias.red(), e);
                }
                error_count += 1;
            },
        }
    }

    if !quiet {
        println!(
            "\nSummary: {} updated, {} unchanged, {} errors",
            updated_count.to_string().green(),
            skipped_count,
            if error_count > 0 {
                error_count.to_string().red()
            } else {
                error_count.to_string().normal()
            }
        );
        metrics.print_summary();
    }
    Ok(())
}

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
async fn update_source(
    storage: &Storage,
    alias: &str,
    metrics: PerformanceMetrics,
    flavor: FlavorMode,
    yes: bool,
    quiet: bool,
) -> Result<bool> {
    let start = Instant::now();
    let pb = if quiet {
        ProgressBar::hidden()
    } else {
        create_spinner(format!("Checking {alias}...").as_str())
    };

    let mut available_flavors = storage.available_flavors(alias)?;
    if available_flavors.is_empty() {
        if !quiet {
            pb.finish_with_message(format!("{alias}: no cached flavors"));
        }
        return Ok(false);
    }
    available_flavors.sort();

    let llms_json = storage.load_flavor_json(alias, BASE_FLAVOR)?;
    let llms_metadata = storage.load_source_metadata_for_flavor(alias, BASE_FLAVOR)?;
    let llms_full_json = storage.load_flavor_json(alias, FULL_FLAVOR)?;
    let llms_full_metadata = storage.load_source_metadata_for_flavor(alias, FULL_FLAVOR)?;

    let mut primary: Option<(String, Option<Flavor>, LlmsJson, Option<Source>)> = None;

    if let Some(json) = llms_json.clone() {
        primary = Some((
            BASE_FLAVOR.to_string(),
            Some(Flavor::Llms),
            json,
            llms_metadata.clone(),
        ));
    } else if let Some(json) = llms_full_json.clone() {
        primary = Some((
            FULL_FLAVOR.to_string(),
            Some(Flavor::LlmsFull),
            json,
            llms_full_metadata.clone(),
        ));
    } else {
        for flavor_id in &available_flavors {
            if let Some(json) = storage.load_flavor_json(alias, flavor_id)? {
                let metadata = storage.load_source_metadata_for_flavor(alias, flavor_id)?;
                primary = Some((
                    flavor_id.clone(),
                    Flavor::from_identifier(flavor_id),
                    json,
                    metadata,
                ));
                break;
            }
        }
    }

    let Some((primary_id, _primary_flavor, primary_json, primary_metadata)) = primary else {
        if !quiet {
            pb.finish_with_message(format!("{alias}: no cached flavors"));
        }
        return Ok(false);
    };

    let current_url = primary_metadata
        .as_ref()
        .map_or_else(|| primary_json.source.url.clone(), |meta| meta.url.clone());

    let fetcher = Fetcher::new()?;

    let effective_flavor = if matches!(flavor, FlavorMode::Current) {
        if let Ok(cfg) = blz_core::Config::load() {
            if cfg.defaults.prefer_llms_full {
                FlavorMode::Full
            } else {
                FlavorMode::Current
            }
        } else {
            FlavorMode::Current
        }
    } else {
        flavor
    };

    if let Ok(meta) = crate::utils::http::head_with_retries(&fetcher, &current_url, 3, 200).await {
        let status = meta.status;
        let is_success = (200..=299).contains(&status);
        let is_redirect = (300..=399).contains(&status);
        let size_text = meta
            .content_length
            .map_or_else(|| "unknown size".to_string(), |n| format!("{n} bytes"));
        if is_success || is_redirect {
            if let Some(len) = meta.content_length {
                let denom: u128 = 5u128 * 1024 * 1024;
                let eta_ms_u128 = (u128::from(len) * 1000).div_ceil(denom);
                let eta_ms = u64::try_from(eta_ms_u128).unwrap_or(u64::MAX);
                if !quiet {
                    pb.set_message(format!(
                        "Checking {alias}... • Preflight: [{} • {size_text}] (est ~{eta_ms}ms @5MB/s)",
                        if is_redirect { "REDIRECT" } else { "OK" }
                    ));
                }
            } else if !quiet {
                pb.set_message(format!(
                    "Checking {alias}... • Preflight: [{} • {size_text}]",
                    if is_redirect { "REDIRECT" } else { "OK" }
                ));
            }
        } else {
            return Err(anyhow!(
                "Preflight failed (HTTP {status}) for {current_url}. Verify the URL or update the source."
            ));
        }
    }
    let mut plan_map: HashMap<String, FlavorPlan> = HashMap::new();

    for flavor_id in &available_flavors {
        let flavor_enum = Flavor::from_identifier(flavor_id);
        let (json, metadata) = if flavor_id.as_str() == BASE_FLAVOR {
            let json = llms_json
                .clone()
                .ok_or_else(|| anyhow!("Missing cached {BASE_FLAVOR}.json for {alias}"))?;
            let metadata = llms_metadata.clone().or_else(|| Some(json.source.clone()));
            (json, metadata)
        } else if flavor_id.as_str() == FULL_FLAVOR {
            let json = llms_full_json
                .clone()
                .ok_or_else(|| anyhow!("Missing cached {FULL_FLAVOR}.json for {alias}"))?;
            let metadata = llms_full_metadata
                .clone()
                .or_else(|| Some(json.source.clone()));
            (json, metadata)
        } else if *flavor_id == primary_id {
            let metadata = primary_metadata
                .clone()
                .or_else(|| Some(primary_json.source.clone()));
            (primary_json.clone(), metadata)
        } else {
            let json = storage
                .load_flavor_json(alias, flavor_id)?
                .ok_or_else(|| anyhow!("Missing cached {flavor_id}.json for {alias}"))?;
            let metadata = storage
                .load_source_metadata_for_flavor(alias, flavor_id)?
                .or_else(|| Some(json.source.clone()));
            (json, metadata)
        };

        let url = metadata
            .as_ref()
            .map_or_else(|| json.source.url.clone(), |meta| meta.url.clone());

        let file_name = json
            .files
            .first()
            .map_or_else(|| format!("{flavor_id}.txt"), |f| f.path.clone());

        plan_map.insert(
            flavor_id.clone(),
            FlavorPlan {
                flavor: flavor_enum,
                flavor_id: flavor_id.clone(),
                file_name,
                url,
                existing_json: Some(json),
                existing_metadata: metadata,
            },
        );
    }

    let prefer_full = settings::effective_prefer_llms_full();
    let allow_auto_full = matches!(effective_flavor, FlavorMode::Auto) && (yes || prefer_full);
    let include_new_full = plan_map.contains_key(FULL_FLAVOR)
        || matches!(effective_flavor, FlavorMode::Full)
        || allow_auto_full;

    let candidates = discover_flavor_candidates(&fetcher, &current_url).await?;
    for candidate in candidates {
        let flavor_id = candidate.flavor_id.clone();
        let flavor_enum = Flavor::from_identifier(&flavor_id);
        if plan_map.contains_key(&flavor_id) {
            if matches!(flavor_enum, Some(Flavor::Llms)) {
                if let Some(plan) = plan_map.get_mut(&flavor_id) {
                    plan.url.clone_from(&candidate.url);
                    plan.file_name.clone_from(&candidate.file_name);
                }
            }
            continue;
        }

        if matches!(flavor_enum, Some(Flavor::LlmsFull)) {
            if include_new_full {
                plan_map.insert(
                    flavor_id.clone(),
                    FlavorPlan {
                        flavor: flavor_enum,
                        flavor_id,
                        file_name: candidate.file_name.clone(),
                        url: candidate.url.clone(),
                        existing_json: None,
                        existing_metadata: None,
                    },
                );
            }
        } else if matches!(flavor_enum, Some(Flavor::Llms)) {
            plan_map.insert(
                flavor_id.clone(),
                FlavorPlan {
                    flavor: flavor_enum,
                    flavor_id,
                    file_name: candidate.file_name.clone(),
                    url: candidate.url.clone(),
                    existing_json: llms_json.clone(),
                    existing_metadata: llms_metadata.clone(),
                },
            );
        }
    }

    if plan_map.is_empty() {
        if !quiet {
            pb.finish_with_message(format!("{alias}: no cached flavors"));
        }
        return Ok(false);
    }

    let mut plans: Vec<FlavorPlan> = plan_map.into_values().collect();
    plans.sort_by(|a, b| flavor_sort_key(&a.flavor_id).cmp(&flavor_sort_key(&b.flavor_id)));

    let index_dir = storage.index_dir(alias)?;
    let index = SearchIndex::create_or_open(&index_dir)?.with_metrics(metrics.clone());
    let mut summaries: Vec<FlavorSummary> = Vec::new();
    let mut any_modified = false;
    let mut archived = false;

    for plan in &mut plans {
        if !quiet {
            let label = if matches!(plan.flavor, Some(Flavor::Llms)) {
                alias.to_string()
            } else {
                format!("{alias} [{}]", plan.canonical_flavor_id())
            };
            pb.set_message(format!("Fetching {label}..."));
        }

        let (etag_hint, last_modified_hint) = plan
            .existing_metadata
            .as_ref()
            .map_or((None, None), |meta| {
                (meta.etag.as_deref(), meta.last_modified.as_deref())
            });

        let fetch = fetcher
            .fetch_with_cache(&plan.url, etag_hint, last_modified_hint)
            .await?;

        match fetch {
            FetchResult::NotModified {
                etag,
                last_modified,
            } => {
                let flavor_key = plan.canonical_flavor_id().to_string();
                let json = plan.existing_json.as_ref().ok_or_else(|| {
                    anyhow!(
                        "Server reported 304 Not Modified for new flavor {}",
                        flavor_key
                    )
                })?;

                if let Some(mut metadata) = plan.existing_metadata.clone() {
                    metadata.etag = etag.or(metadata.etag);
                    metadata.last_modified = last_modified.or(metadata.last_modified);
                    metadata.fetched_at = Utc::now();
                    storage.save_source_metadata_for_flavor(alias, &flavor_key, &metadata)?;
                    plan.existing_metadata = Some(metadata);
                }

                summaries.push(FlavorSummary {
                    flavor: flavor_key,
                    headings: json.toc.len(),
                    lines: json.line_index.total_lines,
                });
            },
            FetchResult::Modified {
                content,
                etag,
                last_modified,
                sha256,
            } => {
                if !archived {
                    storage.archive(alias)?;
                    archived = true;
                }

                let mut parser = MarkdownParser::new()?;
                let parse_result = parser.parse(&content)?;

                let file_name = plan.canonical_file_name().to_string();
                let flavor_id = plan.canonical_flavor_id().to_string();

                storage.save_flavor_content(alias, &file_name, &content)?;

                let mut llms_json = build_llms_json(
                    alias,
                    &plan.url,
                    &file_name,
                    sha256.clone(),
                    etag.clone(),
                    last_modified.clone(),
                    &parse_result,
                );
                if let Some(existing) = plan.existing_json.as_ref() {
                    llms_json
                        .source
                        .aliases
                        .clone_from(&existing.source.aliases);
                }
                storage.save_flavor_json(alias, &flavor_id, &llms_json)?;

                let aliases = plan
                    .existing_metadata
                    .as_ref()
                    .map_or_else(Vec::new, |meta| meta.aliases.clone());

                let metadata = Source {
                    url: plan.url.clone(),
                    etag,
                    last_modified,
                    fetched_at: Utc::now(),
                    sha256,
                    aliases,
                };
                storage.save_source_metadata_for_flavor(alias, &flavor_id, &metadata)?;

                let previous_json = plan.existing_json.clone();
                plan.existing_metadata = Some(metadata);
                plan.existing_json = Some(llms_json.clone());

                index.index_blocks(alias, &file_name, &parse_result.heading_blocks, &flavor_id)?;

                if matches!(plan.flavor, Some(Flavor::Llms)) {
                    if let Some(old_json) = previous_json {
                        if !old_json.toc.is_empty() && !llms_json.toc.is_empty() {
                            let mappings = compute_anchor_mappings(&old_json.toc, &llms_json.toc);
                            if !mappings.is_empty() {
                                let anchors_map = build_anchors_map(mappings, Utc::now());
                                let _ = storage.save_anchors_map(alias, &anchors_map);
                            }
                        }
                    }
                }

                summaries.push(FlavorSummary {
                    flavor: flavor_id,
                    headings: llms_json.toc.len(),
                    lines: llms_json.line_index.total_lines,
                });

                any_modified = true;
            },
        }
    }

    let summary_text = format_summary(&summaries);
    let elapsed = start.elapsed();

    let available_flavors: HashSet<&str> = summaries.iter().map(|s| s.flavor.as_str()).collect();
    match flavor {
        FlavorMode::Full => {
            if available_flavors.contains(FULL_FLAVOR) {
                set_preferred_flavor(PreferenceScope::Local, alias, Some(FULL_FLAVOR))?;
            } else if !quiet {
                eprintln!(
                    "{} Full flavor not available for {}; keeping existing preference",
                    "Warning:".yellow(),
                    alias
                );
            }
        },
        FlavorMode::Txt => {
            if available_flavors.contains(BASE_FLAVOR) {
                set_preferred_flavor(PreferenceScope::Local, alias, Some(BASE_FLAVOR))?;
            }
        },
        FlavorMode::Auto => {
            // Allow global/project defaults to drive resolution by clearing local override.
            set_preferred_flavor(PreferenceScope::Local, alias, None)?;
        },
        FlavorMode::Current => {
            // Leave per-source override untouched.
        },
    }

    if quiet {
        pb.finish_and_clear();
    } else if any_modified {
        pb.finish_with_message(format!(
            "✓ Updated {} ({summary_text}) in {:.1}s",
            alias.green(),
            elapsed.as_secs_f32()
        ));
    } else {
        pb.finish_with_message(format!("{alias}: Up-to-date ({summary_text})"));
    }

    info!(
        "Updated {} with {} flavor(s) in {:.2?}",
        alias,
        summaries.len(),
        elapsed
    );

    Ok(any_modified)
}

fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message(message.to_string());
    pb
}

fn flavor_sort_key(flavor: &str) -> u8 {
    match flavor {
        BASE_FLAVOR => 0,
        FULL_FLAVOR => 1,
        _ => 2,
    }
}

fn format_summary(summaries: &[FlavorSummary]) -> String {
    if summaries.is_empty() {
        return "no flavors".to_string();
    }
    summaries
        .iter()
        .map(|s| format!("{}: {} headings, {} lines", s.flavor, s.headings, s.lines))
        .collect::<Vec<_>>()
        .join(", ")
}
