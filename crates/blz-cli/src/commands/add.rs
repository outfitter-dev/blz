//! Add command implementation

use anyhow::Result;
use blz_core::{Fetcher, Flavor, MarkdownParser, PerformanceMetrics, SearchIndex, Source, Storage};
use chrono::Utc;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use url::Url;

use crate::utils::flavor::{build_llms_json, discover_flavor_candidates};
use crate::utils::validation::{normalize_alias, validate_alias};

/// Add a new documentation source
///
/// # Arguments
///
/// * `alias` - Local alias for the source (will be normalized to kebab-case)
/// * `url` - URL to fetch llms.txt from
/// * `auto_yes` - Skip confirmation prompts (non-interactive mode)
/// * `metrics` - Performance metrics collector
pub async fn execute(
    alias: &str,
    url: &str,
    auto_yes: bool,
    quiet: bool,
    metrics: PerformanceMetrics,
) -> Result<()> {
    let _ = auto_yes;
    // Normalize the alias to kebab-case lowercase
    let normalized_alias = normalize_alias(alias);

    // Show normalization if it changed
    if normalized_alias != alias && !quiet {
        println!(
            "Normalizing alias: '{}' → '{}'",
            alias,
            normalized_alias.green()
        );
    }

    // Validate the normalized alias
    validate_alias(&normalized_alias)?;

    let fetcher = Fetcher::new()?;

    if let Ok(parsed) = Url::parse(url) {
        match parsed.scheme() {
            "http" | "https" => {},
            other => {
                if !quiet {
                    eprintln!(
                        "Warning: URL scheme '{other}' may not be supported for fetching ({url}).\n \
                         If this is a local file, consider hosting llms.txt or using a supported HTTP URL."
                    );
                }
            },
        }
    } else if !quiet {
        eprintln!("Warning: URL appears invalid: {url}");
    }

    fetch_and_index_variants(&normalized_alias, url, quiet, fetcher, metrics).await
}

#[allow(clippy::too_many_lines)]
async fn fetch_and_index_variants(
    alias: &str,
    url: &str,
    quiet: bool,
    fetcher: Fetcher,
    metrics: PerformanceMetrics,
) -> Result<()> {
    let mut candidates = discover_flavor_candidates(&fetcher, url).await?;
    if candidates.is_empty() {
        anyhow::bail!("No reachable llms.txt variants discovered for {url}");
    }

    let storage = Storage::new()?;
    if storage.exists_any_flavor(alias) {
        anyhow::bail!(
            "Source '{}' already exists. Use 'blz update {}' or choose a different alias.",
            alias,
            alias
        );
    }
    let index_path = storage.index_dir(alias)?;
    let index = SearchIndex::create(&index_path)?.with_metrics(metrics);

    let spinner = if quiet {
        ProgressBar::hidden()
    } else {
        create_spinner("Fetching documentation variants...")
    };
    let mut summaries = Vec::new();
    let mut skipped = 0usize;

    for candidate in &mut candidates {
        if !quiet {
            spinner.set_message(format!("Fetching {}", candidate.file_name));
        }

        let head_ok = match crate::utils::http::head_with_retries(&fetcher, &candidate.url, 3, 200)
            .await
        {
            Ok(meta) => {
                let status = meta.status;
                if status == 405 || status == 501 {
                    if !quiet {
                        eprintln!(
                            "Warning: Server does not support HEAD (HTTP {status}) for {}. Proceeding with GET.",
                            candidate.url
                        );
                    }
                    true
                } else if (200..=399).contains(&status) {
                    true
                } else {
                    if !quiet {
                        eprintln!(
                            "Skipping {} due to preflight failure (HTTP {status}).",
                            candidate.url
                        );
                    }
                    skipped += 1;
                    false
                }
            },
            Err(err) => {
                if !quiet {
                    eprintln!(
                        "Skipping {}: Preflight HEAD request failed: {err}",
                        candidate.url
                    );
                }
                skipped += 1;
                false
            },
        };

        if !head_ok {
            continue;
        }

        let fetch_result = fetcher.fetch_with_cache(&candidate.url, None, None).await?;

        let (content, sha256, etag, last_modified) = match fetch_result {
            blz_core::FetchResult::Modified {
                content,
                sha256,
                etag,
                last_modified,
            } => (content, sha256, etag, last_modified),
            blz_core::FetchResult::NotModified { .. } => {
                if !quiet {
                    eprintln!(
                        "Skipping {}: server returned 304 Not Modified on initial fetch.",
                        candidate.url
                    );
                }
                skipped += 1;
                continue;
            },
        };

        if !quiet {
            spinner.set_message(format!("Parsing {}", candidate.file_name));
        }
        let mut parser = MarkdownParser::new()?;
        let parse_result = parser.parse(&content)?;

        let (file_name, flavor_id, summary_label) =
            match Flavor::from_identifier(&candidate.flavor_id) {
                Some(flavor) => {
                    let id = flavor.as_str();

                    // Log when using llms-full due to FORCE_PREFER_FULL
                    if crate::utils::flavor::FORCE_PREFER_FULL
                        && id == crate::utils::flavor::FULL_FLAVOR
                    {
                        tracing::info!(
                            alias = alias,
                            flavor = id,
                            "Using llms-full.txt (preferred flavor)"
                        );
                    }

                    (flavor.file_name(), id, id.to_string())
                },
                None => (
                    candidate.file_name.as_str(),
                    candidate.flavor_id.as_str(),
                    candidate.flavor_id.clone(),
                ),
            };

        storage.save_flavor_content(alias, file_name, &content)?;

        let llms_json = build_llms_json(
            alias,
            &candidate.url,
            file_name,
            sha256.clone(),
            etag.clone(),
            last_modified.clone(),
            &parse_result,
        );
        storage.save_flavor_json(alias, flavor_id, &llms_json)?;

        let metadata = Source {
            url: candidate.url.clone(),
            etag,
            last_modified,
            fetched_at: Utc::now(),
            sha256,
            aliases: Vec::new(),
        };
        storage.save_source_metadata_for_flavor(alias, flavor_id, &metadata)?;

        index.index_blocks(alias, file_name, &parse_result.heading_blocks, flavor_id)?;

        summaries.push((
            summary_label,
            llms_json.toc.len(),
            llms_json.line_index.total_lines,
        ));
    }

    spinner.finish_and_clear();

    if summaries.is_empty() {
        anyhow::bail!(
            "No reachable llms.txt variants for {}. Nothing was added ({} skipped).",
            url,
            skipped
        );
    }

    let summary_text = summaries
        .iter()
        .map(|(flavor, headings, lines)| format!("{flavor}: {headings} headings, {lines} lines"))
        .collect::<Vec<_>>()
        .join(", ");

    if !quiet {
        if skipped > 0 {
            println!(
                "{} {} ({}) — {skipped} variant(s) skipped",
                "✓ Added".green(),
                alias.green(),
                summary_text
            );
        } else {
            println!("{} {} ({})", "✓ Added".green(), alias.green(), summary_text);
        }
    }

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
