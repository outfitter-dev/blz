//! Update command implementation

use anyhow::{anyhow, Result};
use blz_core::{
    FetchResult, Fetcher, LineIndex, LlmsJson, MarkdownParser, PerformanceMetrics, ResourceMonitor,
    SearchIndex, Source, Storage,
};
use chrono::Utc;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Instant;
use tracing::{debug, info};

/// Execute update for a specific source
pub async fn execute(
    alias: &str,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    let storage = Storage::new()?;

    if !storage.exists(alias) {
        return Err(anyhow!("Source '{}' not found", alias));
    }

    update_source(&storage, alias, metrics.clone()).await?;

    if let Some(monitor) = resource_monitor {
        monitor.print_resource_usage();
    }

    metrics.print_summary();
    Ok(())
}

/// Execute update for all sources
pub async fn execute_all(
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources()?;

    if sources.is_empty() {
        println!("No sources to update");
        return Ok(());
    }

    println!("Updating {} source(s)...", sources.len());
    let mut updated_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    for alias in sources {
        match update_source(&storage, &alias, metrics.clone()).await {
            Ok(updated) => {
                if updated {
                    updated_count += 1;
                } else {
                    skipped_count += 1;
                }
            },
            Err(e) => {
                eprintln!("Failed to update '{}': {}", alias.red(), e);
                error_count += 1;
            },
        }
    }

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

    if let Some(monitor) = resource_monitor {
        monitor.print_resource_usage();
    }

    metrics.print_summary();
    Ok(())
}

async fn update_source(
    storage: &Storage,
    alias: &str,
    metrics: PerformanceMetrics,
) -> Result<bool> {
    let start = Instant::now();

    // Load existing metadata
    let existing_metadata = storage.load_source_metadata(alias)?;
    let existing_json = storage.load_llms_json(alias)?;

    let url = existing_json.source.url.clone();
    let pb = create_spinner(format!("Checking {alias}...").as_str());

    // Create fetcher
    let fetcher = Fetcher::new()?;

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
        } => {
            pb.finish_with_message(format!("{}: Up-to-date", alias));
            info!("{} is up to date", alias);

            // Update metadata timestamp and any new validator values
            let current = existing_metadata
                .clone()
                .unwrap_or(existing_json.source.clone());

            let updated_metadata = Source {
                fetched_at: Utc::now(),
                etag: new_etag.or(current.etag),
                last_modified: new_last_modified.or(current.last_modified),
                ..current
            };

            storage.save_source_metadata(alias, &updated_metadata)?;

            Ok(false)
        },
        FetchResult::Modified {
            content,
            etag: new_etag,
            last_modified: new_last_modified,
            sha256,
        } => {
            pb.set_message(format!("Updating {alias}..."));

            // Check if content actually changed (SHA256 comparison)
            if existing_json.source.sha256 == sha256 {
                pb.finish_with_message(format!("{}: Content unchanged", alias));

                // Update metadata even if content hasn't changed (server headers might have)
                let new_metadata = Source {
                    url: url.clone(),
                    etag: new_etag,
                    last_modified: new_last_modified,
                    fetched_at: Utc::now(),
                    sha256,
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
            let parse_result = parser.parse(&content)?;

            // Save new content
            pb.set_message(format!("Saving {alias}..."));
            storage.save_llms_txt(alias, &content)?;

            // Create and save updated JSON
            let new_json = LlmsJson {
                alias: alias.to_string(),
                source: Source {
                    url: url.clone(),
                    etag: new_etag.clone(),
                    last_modified: new_last_modified.clone(),
                    fetched_at: Utc::now(),
                    sha256: sha256.clone(),
                },
                toc: parse_result.toc,
                files: vec![blz_core::FileInfo {
                    path: "llms.txt".to_string(),
                    sha256: sha256.clone(),
                }],
                line_index: LineIndex {
                    total_lines: parse_result.line_count,
                    byte_offsets: false,
                },
                diagnostics: parse_result.diagnostics,
            };
            storage.save_llms_json(alias, &new_json)?;

            // Save metadata separately for efficient checking
            let metadata = Source {
                url,
                etag: new_etag,
                last_modified: new_last_modified,
                fetched_at: Utc::now(),
                sha256,
            };
            storage.save_source_metadata(alias, &metadata)?;

            // Rebuild search index atomically
            pb.set_message(format!("Reindexing {alias}..."));
            let index_path = storage.index_dir(alias)?;

            // Build into temp path, then atomic swap
            let tmp_index = index_path.with_extension("new");
            if tmp_index.exists() {
                std::fs::remove_dir_all(&tmp_index)
                    .map_err(|e| anyhow!("Failed to clean temp index: {}", e))?;
            }
            
            let mut index = SearchIndex::create(&tmp_index)?.with_metrics(metrics);
            index.index_blocks(alias, "llms.txt", &parse_result.heading_blocks)?;
            
            // Swap in the new index
            if index_path.exists() {
                std::fs::remove_dir_all(&index_path)
                    .map_err(|e| anyhow!("Failed to remove old index: {}", e))?;
            }
            std::fs::rename(&tmp_index, &index_path)
                .map_err(|e| anyhow!("Failed to move new index into place: {}", e))?;

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
        },
    }
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
