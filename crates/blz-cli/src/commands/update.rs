//! Update command implementation

use anyhow::{Result, anyhow};
use blz_core::{
    FetchResult, Fetcher, MarkdownParser, PerformanceMetrics, SearchIndex, Source, Storage,
};
use chrono::Utc;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Instant;

use crate::utils::count_headings;
use crate::utils::json_builder::build_llms_json;
use crate::utils::resolver;

/// Execute update for a specific source
pub async fn execute(alias: &str, metrics: PerformanceMetrics, quiet: bool) -> Result<()> {
    let storage = Storage::new()?;
    let canonical_alias =
        resolver::resolve_source(&storage, alias)?.unwrap_or_else(|| alias.to_string());

    if !storage.exists(&canonical_alias) {
        return Err(anyhow!("Source '{}' not found", alias));
    }

    update_source(&storage, &canonical_alias, metrics, quiet)
        .await
        .map(|_| ())
}

/// Execute update for all sources
pub async fn execute_all(metrics: PerformanceMetrics, quiet: bool) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources();

    if sources.is_empty() {
        anyhow::bail!("No sources configured. Use 'blz add' to add sources.");
    }

    let mut updated_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    for alias in sources {
        match update_source(&storage, &alias, metrics.clone(), quiet).await {
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

async fn update_source(
    storage: &Storage,
    alias: &str,
    metrics: PerformanceMetrics,
    quiet: bool,
) -> Result<bool> {
    let start = Instant::now();
    let pb = if quiet {
        ProgressBar::hidden()
    } else {
        create_spinner(format!("Checking {alias}...").as_str())
    };

    // Load existing metadata
    let existing_metadata = storage
        .load_source_metadata(alias)?
        .ok_or_else(|| anyhow!("Missing metadata for {}", alias))?;

    let current_url = existing_metadata.url.clone();
    let fetcher = Fetcher::new()?;

    pb.set_message(format!("Checking {alias}..."));

    // Conditional fetch using ETag
    let fetch_result = fetcher
        .fetch_with_cache(
            &current_url,
            existing_metadata.etag.as_deref(),
            existing_metadata.last_modified.as_deref(),
        )
        .await?;

    let (content, sha256, etag, last_modified) = match fetch_result {
        FetchResult::Modified {
            content,
            sha256,
            etag,
            last_modified,
        } => (content, sha256, etag, last_modified),
        FetchResult::NotModified { .. } => {
            pb.finish_and_clear();
            if !quiet {
                println!("{} {} (unchanged)", "✓".green(), alias.green());
            }
            return Ok(false);
        },
    };

    // Content changed - parse and reindex
    pb.set_message(format!("Parsing {alias}..."));
    let mut parser = MarkdownParser::new()?;
    let parse_result = parser.parse(&content)?;

    // Save updated content
    pb.set_message(format!("Saving {alias}..."));
    storage.save_llms_txt(alias, &content)?;

    // Build and save updated JSON (preserve existing aliases and tags)
    let mut llms_json = build_llms_json(
        alias,
        &current_url,
        "llms.txt",
        sha256.clone(),
        etag.clone(),
        last_modified.clone(),
        &parse_result,
    );
    llms_json.metadata.aliases = existing_metadata.aliases.clone();
    llms_json.metadata.tags = existing_metadata.tags.clone();
    storage.save_llms_json(alias, &llms_json)?;

    // Save updated metadata
    let metadata = Source {
        url: current_url,
        etag,
        last_modified,
        fetched_at: Utc::now(),
        sha256,
        aliases: existing_metadata.aliases,
        tags: existing_metadata.tags,
    };
    storage.save_source_metadata(alias, &metadata)?;

    // Reindex (recreate to clear old data)
    pb.set_message(format!("Indexing {alias}..."));
    let index_path = storage.index_dir(alias)?;
    let index = SearchIndex::create_or_open(&index_path)?.with_metrics(metrics);
    index.index_blocks(alias, &parse_result.heading_blocks)?;

    pb.finish_and_clear();

    let elapsed = start.elapsed();
    if !quiet {
        println!(
            "{} {} ({} headings, {} lines) in {:?}",
            "✓ Updated".green(),
            alias.green(),
            count_headings(&llms_json.toc),
            llms_json.line_index.total_lines,
            elapsed
        );
    }

    Ok(true)
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
