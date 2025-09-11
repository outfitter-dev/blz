//! Update command implementation

use anyhow::{Result, anyhow};
use blz_core::{
    FetchResult, Fetcher, LineIndex, LlmsJson, MarkdownParser, ParseResult, PerformanceMetrics,
    SearchIndex, Source, Storage,
};
use chrono::Utc;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Instant;
use tracing::{debug, info};

/// Execute update for a specific source
pub async fn execute(alias: &str, metrics: PerformanceMetrics) -> Result<()> {
    let storage = Storage::new()?;

    if !storage.exists(alias) {
        return Err(anyhow!("Source '{}' not found", alias));
    }

    update_source(&storage, alias, metrics.clone()).await?;

    metrics.print_summary();
    Ok(())
}

/// Execute update for all sources
pub async fn execute_all(metrics: PerformanceMetrics) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources();

    if sources.is_empty() {
        println!("No sources to update");
        return Ok(());
    }

    println!("Updating {} source(s)...", sources.len());
    let mut updated_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    for alias in &sources {
        match update_source(&storage, alias, metrics.clone()).await {
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

#[allow(clippy::too_many_arguments)]
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
    };
    storage.save_llms_json(alias, &new_json)?;

    // Rebuild search index
    rebuild_index(storage, alias, &parse_result, metrics, pb)?;

    // Save metadata only after a successful index rebuild
    let metadata = Source {
        url: url.to_string(),
        etag: new_etag,
        last_modified: new_last_modified,
        fetched_at: Utc::now(),
        sha256,
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
