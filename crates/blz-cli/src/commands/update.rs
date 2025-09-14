//! Update command implementation

use anyhow::{Result, anyhow};
use blz_core::{
    FetchResult, Fetcher, LineIndex, LlmsJson, MarkdownParser, ParseResult, PerformanceMetrics,
    SearchIndex, Source, Storage, build_anchors_map, compute_anchor_mappings,
};
use chrono::Utc;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Instant;
use tracing::{debug, info};

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

/// Execute update for a specific source
pub async fn execute(
    alias: &str,
    metrics: PerformanceMetrics,
    quiet: bool,
    flavor: FlavorMode,
    yes: bool,
) -> Result<()> {
    let storage = Storage::new()?;

    // Resolve metadata alias to canonical if needed
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .unwrap_or_else(|| alias.to_string());

    if !storage.exists(&canonical) {
        return Err(anyhow!("Source '{}' not found", alias));
    }

    update_source(&storage, &canonical, metrics.clone(), flavor, yes, quiet).await?;

    if !quiet {
        metrics.print_summary();
    }
    Ok(())
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
        if !quiet {
            println!("No sources to update");
        }
        return Ok(());
    }

    if !quiet {
        println!("Updating {} source(s)...", sources.len());
    }
    let mut updated_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    for alias in &sources {
        match update_source(&storage, alias, metrics.clone(), flavor, yes, quiet).await {
            Ok(updated) => {
                if updated {
                    updated_count += 1;
                } else {
                    skipped_count += 1;
                }
            },
            Err(e) => {
                if !quiet {
                    eprintln!("Failed to update '{}': {}", alias.red(), e);
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

async fn update_source(
    storage: &Storage,
    alias: &str,
    metrics: PerformanceMetrics,
    flavor: FlavorMode,
    yes: bool,
    quiet: bool,
) -> Result<bool> {
    let start = Instant::now();

    // Load existing metadata
    let existing_metadata = storage.load_source_metadata(alias)?;
    let existing_json = storage.load_llms_json(alias)?;

    let mut url = existing_json.source.url.clone();
    let pb = create_spinner(format!("Checking {alias}...").as_str());

    // Create fetcher
    let fetcher = Fetcher::new()?;

    // Optional flavor upgrade/selection (consider global config default)
    let effective_flavor = if matches!(flavor, FlavorMode::Current) {
        // Respect config default if set
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

    if let Some(new_url) =
        select_update_flavor(&fetcher, &url, effective_flavor, yes, quiet).await?
    {
        url = new_url;
    }

    // Preflight HEAD summary (size/ETA) and early failure on non-2xx
    if let Ok(meta) = crate::utils::http::head_with_retries(&fetcher, &url, 3, 200).await {
        // Treat 2xx as OK; accept 3xx as soft-OK (redirects) to avoid false negatives
        let status_u16 = meta.status;
        let is_success = (200..=299).contains(&status_u16);
        let is_redirect = (300..=399).contains(&status_u16);
        let size_text = meta
            .content_length
            .map_or_else(|| "unknown size".to_string(), |n| format!("{n} bytes"));

        if is_success || is_redirect {
            if let Some(n) = meta.content_length {
                // Show rough ETA assuming ~5 MB/s when size is known
                let denom: u128 = 5u128 * 1024 * 1024; // bytes per second
                let eta_ms_u128 = (u128::from(n) * 1000).div_ceil(denom);
                let eta_ms = u64::try_from(eta_ms_u128).unwrap_or(u64::MAX);
                if is_redirect {
                    pb.set_message(format!(
                        "Checking {alias}... • Preflight: [REDIRECT • {size_text}] (est ~{eta_ms}ms @5MB/s)"
                    ));
                } else {
                    pb.set_message(format!(
                        "Checking {alias}... • Preflight: [OK • {size_text}] (est ~{eta_ms}ms @5MB/s)"
                    ));
                }
            } else if is_redirect {
                pb.set_message(format!(
                    "Checking {alias}... • Preflight: [REDIRECT • {size_text}]"
                ));
            } else {
                pb.set_message(format!(
                    "Checking {alias}... • Preflight: [OK • {size_text}]"
                ));
            }
        } else {
            // Fail fast for clearer errors before attempting fetch
            return Err(anyhow!(
                "Preflight failed (HTTP {status}) for {url}. Verify the URL or update the source.",
                status = meta.status
            ));
        }
    }

    // Try conditional fetch with ETag/Last-Modified
    let (etag, last_modified) = if let Some(ref metadata) = existing_metadata {
        (metadata.etag.as_deref(), metadata.last_modified.as_deref())
    } else {
        // Fall back to existing json source info
        (
            existing_json.source.etag.as_deref(),
            existing_json.source.last_modified.as_deref(),
        )
    };

    debug!(
        "Fetching {} with ETag: {:?}, Last-Modified: {:?}",
        url, etag, last_modified
    );
    let fetch_result = fetcher.fetch_with_cache(&url, etag, last_modified).await?;

    match fetch_result {
        FetchResult::NotModified {
            etag: new_etag,
            last_modified: new_last_modified,
        } => handle_not_modified(
            storage,
            alias,
            &pb,
            existing_metadata,
            existing_json.source,
            new_etag,
            new_last_modified,
        ),
        FetchResult::Modified {
            content,
            etag: new_etag,
            last_modified: new_last_modified,
            sha256,
        } => handle_modified(
            storage,
            alias,
            &pb,
            &url,
            &existing_json,
            &content,
            new_etag,
            new_last_modified,
            sha256,
            metrics,
            start,
        ),
    }
}

/// Decide whether to change the URL flavor during update
async fn select_update_flavor(
    fetcher: &Fetcher,
    current_url: &str,
    flavor: FlavorMode,
    yes: bool,
    quiet: bool,
) -> Result<Option<String>> {
    match flavor {
        FlavorMode::Current => Ok(None),
        FlavorMode::Auto | FlavorMode::Full | FlavorMode::Txt => {
            let flavors = fetcher.check_flavors(current_url).await.unwrap_or_default();
            if flavors.is_empty() {
                return Ok(None);
            }
            let want = match flavor {
                FlavorMode::Auto => flavors.get(0).map(|f| f.url.clone()),
                FlavorMode::Full => flavors
                    .iter()
                    .find(|f| f.name == "llms-full.txt")
                    .map(|f| f.url.clone()),
                FlavorMode::Txt => flavors
                    .iter()
                    .find(|f| f.name == "llms.txt")
                    .map(|f| f.url.clone()),
                FlavorMode::Current => None,
            };
            if let Some(candidate) = want {
                if candidate != current_url {
                    if yes {
                        if !quiet {
                            eprintln!("Upgrading flavor: {} -> {}", current_url, candidate);
                        }
                        return Ok(Some(candidate));
                    } else if !quiet {
                        eprintln!(
                            "Flavor upgrade available (use --yes to apply): {} -> {}",
                            current_url, candidate
                        );
                    }
                }
            }
            Ok(None)
        },
    }
}

fn handle_not_modified(
    storage: &Storage,
    alias: &str,
    pb: &ProgressBar,
    existing_metadata: Option<Source>,
    existing_source: Source,
    new_etag: Option<String>,
    new_last_modified: Option<String>,
) -> Result<bool> {
    pb.finish_with_message(format!("{alias}: Up-to-date"));
    info!("{} is up to date", alias);

    // Update metadata timestamp and any new validator values
    let current = existing_metadata.unwrap_or(existing_source);

    let updated_metadata = Source {
        fetched_at: Utc::now(),
        etag: new_etag.or(current.etag),
        last_modified: new_last_modified.or(current.last_modified),
        ..current
    };

    storage.save_source_metadata(alias, &updated_metadata)?;
    Ok(false)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn handle_modified(
    storage: &Storage,
    alias: &str,
    pb: &ProgressBar,
    url: &str,
    existing_json: &LlmsJson,
    content: &str,
    new_etag: Option<String>,
    new_last_modified: Option<String>,
    sha256: String,
    metrics: PerformanceMetrics,
    start: Instant,
) -> Result<bool> {
    pb.set_message(format!("Updating {alias}..."));

    // Check if content actually changed (SHA256 comparison)
    if existing_json.source.sha256 == sha256 {
        pb.finish_with_message(format!("{alias}: Content unchanged"));

        // Update metadata even if content hasn't changed (server headers might have)
        let new_metadata = Source {
            url: url.to_string(),
            etag: new_etag,
            last_modified: new_last_modified,
            fetched_at: Utc::now(),
            sha256,
            aliases: existing_json.source.aliases.clone(),
        };
        storage.save_source_metadata(alias, &new_metadata)?;

        return Ok(false);
    }

    // Archive existing content before updating
    pb.set_message(format!("Archiving {alias}..."));
    storage.archive(alias)?;

    // Parse new content
    pb.set_message(format!("Parsing {alias}..."));
    let mut parser = MarkdownParser::new()?;
    let parse_result = parser.parse(content)?;

    // Save new content
    pb.set_message(format!("Saving {alias}..."));
    storage.save_llms_txt(alias, content)?;

    // Create and save updated JSON
    let new_json = LlmsJson {
        alias: alias.to_string(),
        source: Source {
            url: url.to_string(),
            etag: new_etag.clone(),
            last_modified: new_last_modified.clone(),
            fetched_at: Utc::now(),
            sha256: sha256.clone(),
            aliases: existing_json.source.aliases.clone(),
        },
        toc: parse_result.toc.clone(),
        files: vec![blz_core::FileInfo {
            path: "llms.txt".to_string(),
            sha256: sha256.clone(),
        }],
        line_index: LineIndex {
            total_lines: parse_result.line_count,
            byte_offsets: false,
        },
        diagnostics: parse_result.diagnostics.clone(),
        parse_meta: Some(blz_core::ParseMeta {
            parser_version: 1,
            segmentation: "structured".to_string(),
        }),
    };
    storage.save_llms_json(alias, &new_json)?;

    // Build anchors remap from old -> new using core helper
    if !existing_json.toc.is_empty() && !new_json.toc.is_empty() {
        let mappings = compute_anchor_mappings(&existing_json.toc, &new_json.toc);
        if !mappings.is_empty() {
            let anchors_map = build_anchors_map(mappings, Utc::now());
            let _ = storage.save_anchors_map(alias, &anchors_map);
        }
    }

    // Rebuild search index
    rebuild_index(storage, alias, &parse_result, metrics, pb)?;

    // Save metadata only after a successful index rebuild
    let metadata = Source {
        url: url.to_string(),
        etag: new_etag,
        last_modified: new_last_modified,
        fetched_at: Utc::now(),
        sha256,
        aliases: new_json.source.aliases.clone(),
    };
    storage.save_source_metadata(alias, &metadata)?;

    let elapsed = start.elapsed();
    pb.finish_with_message(format!(
        "✓ Updated {} ({} headings, {} lines) in {:.1}s",
        alias.green(),
        new_json.toc.len(),
        new_json.line_index.total_lines,
        elapsed.as_secs_f32()
    ));

    info!(
        "Updated {} - {} headings, {} lines",
        alias,
        new_json.toc.len(),
        new_json.line_index.total_lines
    );

    Ok(true)
}

fn rebuild_index(
    storage: &Storage,
    alias: &str,
    parse_result: &ParseResult,
    metrics: PerformanceMetrics,
    pb: &ProgressBar,
) -> Result<()> {
    pb.set_message(format!("Reindexing {alias}..."));
    let index_path = storage.index_dir(alias)?;

    // Build into temp path, then atomic swap
    let tmp_index = index_path.with_extension("new");
    if tmp_index.exists() {
        std::fs::remove_dir_all(&tmp_index)
            .map_err(|e| anyhow!("Failed to clean temp index: {}", e))?;
    }

    let index = SearchIndex::create(&tmp_index)?.with_metrics(metrics);
    index.index_blocks(alias, "llms.txt", &parse_result.heading_blocks)?;

    // Ensure no open handles before swapping on Windows
    drop(index);

    // Swap in the new index
    if index_path.exists() {
        std::fs::remove_dir_all(&index_path)
            .map_err(|e| anyhow!("Failed to remove old index: {}", e))?;
    }
    std::fs::rename(&tmp_index, &index_path)
        .map_err(|e| anyhow!("Failed to move new index into place: {}", e))?;

    Ok(())
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
