//! Add command implementation

use anyhow::Result;
use blz_core::{Fetcher, MarkdownParser, PerformanceMetrics, SearchIndex, Source, Storage};
use chrono::Utc;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::utils::count_headings;
use crate::utils::json_builder::build_llms_json;
use crate::utils::url_resolver;
use crate::utils::validation::{normalize_alias, validate_alias};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceAnalysis {
    #[serde(alias = "alias")]
    name: String,
    url: String,
    final_url: String,
    analysis: ContentAnalysis,
    would_index: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentAnalysis {
    line_count: usize,
    char_count: usize,
    header_count: usize,
    sections: usize,
    file_size: String,
    content_type: String,
}

/// Add a new documentation source
///
/// # Arguments
///
/// * `alias` - Local alias for the source (will be normalized to kebab-case)
/// * `url` - URL to fetch llms.txt from
/// * `aliases` - Additional aliases to associate with this source
/// * `auto_yes` - Skip confirmation prompts (non-interactive mode)
/// * `dry_run` - Analyze source without adding it (outputs JSON analysis)
/// * `metrics` - Performance metrics collector
pub async fn execute(
    alias: &str,
    url: &str,
    aliases: &[String],
    auto_yes: bool,
    dry_run: bool,
    quiet: bool,
    metrics: PerformanceMetrics,
) -> Result<()> {
    let _ = auto_yes;
    // Normalize the alias to kebab-case lowercase
    let normalized_alias = normalize_alias(alias);

    // Show normalization if it changed
    if normalized_alias != alias && !quiet && !dry_run {
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
                if !quiet && !dry_run {
                    eprintln!(
                        "Warning: URL scheme '{other}' may not be supported for fetching ({url}).\n \
                         If this is a local file, consider hosting llms.txt or using a supported HTTP URL."
                    );
                }
            },
        }
    } else if !quiet && !dry_run {
        eprintln!("Warning: URL appears invalid: {url}");
    }

    fetch_and_index(
        &normalized_alias,
        url,
        aliases,
        dry_run,
        quiet,
        fetcher,
        metrics,
    )
    .await
}

#[allow(clippy::too_many_lines)]
async fn fetch_and_index(
    alias: &str,
    url: &str,
    aliases: &[String],
    dry_run: bool,
    quiet: bool,
    fetcher: Fetcher,
    metrics: PerformanceMetrics,
) -> Result<()> {
    // Check if source already exists (validate even in dry-run mode)
    let storage = Storage::new()?;
    if storage.exists(alias) {
        anyhow::bail!(
            "Source '{}' already exists. Use 'blz update {}' or choose a different alias.",
            alias,
            alias
        );
    }

    let spinner = if quiet {
        ProgressBar::hidden()
    } else {
        create_spinner("Resolving URL...")
    };

    // Resolve the best URL variant (llms-full.txt vs llms.txt)
    spinner.set_message("Resolving URL variant...");
    let resolved = url_resolver::resolve_best_url(&fetcher, url).await?;

    // Show warning if index file
    if resolved.should_warn && !quiet && !dry_run {
        spinner.finish_and_clear();
        eprintln!(
            "{} This appears to be a navigation index only ({} lines).\n\
             BLZ works best with full documentation files (llms-full.txt).",
            "⚠".yellow(),
            resolved.line_count
        );
    }

    // Fetch from resolved URL
    spinner.set_message("Fetching documentation...");
    let fetch_result = fetcher
        .fetch_with_cache(&resolved.final_url, None, None)
        .await?;

    let (content, sha256, etag, last_modified) = match fetch_result {
        blz_core::FetchResult::Modified {
            content,
            sha256,
            etag,
            last_modified,
        } => (content, sha256, etag, last_modified),
        blz_core::FetchResult::NotModified { .. } => {
            anyhow::bail!(
                "Server returned 304 Not Modified on initial fetch. This should not happen for new sources."
            );
        },
    };

    // Parse the content
    spinner.set_message("Parsing markdown...");
    let mut parser = MarkdownParser::new()?;
    let parse_result = parser.parse(&content)?;

    // In dry-run mode, analyze content and output JSON instead of indexing
    if dry_run {
        let char_count = content.len();
        let header_count = parse_result.heading_blocks.len();
        let sections = parse_result.toc.len();
        let file_size = format_size(content.len());

        let content_type = match resolved.content_type {
            blz_core::ContentType::Full => "full",
            blz_core::ContentType::Index => "index",
            blz_core::ContentType::Mixed => "mixed",
        };

        let analysis = SourceAnalysis {
            name: alias.to_string(),
            url: url.to_string(),
            final_url: resolved.final_url.clone(),
            analysis: ContentAnalysis {
                line_count: resolved.line_count,
                char_count,
                header_count,
                sections,
                file_size,
                content_type: content_type.to_string(),
            },
            would_index: true,
        };

        let json = serde_json::to_string_pretty(&analysis)?;
        println!("{json}");
        spinner.finish_and_clear();
        return Ok(());
    }

    // Save content and metadata
    let storage = Storage::new()?;
    spinner.set_message("Saving content...");
    storage.save_llms_txt(alias, &content)?;

    // Build and save JSON metadata
    let mut llms_json = build_llms_json(
        alias,
        &resolved.final_url,
        "llms.txt",
        sha256.clone(),
        etag.clone(),
        last_modified.clone(),
        &parse_result,
    );
    llms_json.metadata.variant = resolved.variant.clone();
    llms_json.metadata.aliases = aliases.to_vec();
    storage.save_llms_json(alias, &llms_json)?;

    // Save source metadata with resolved variant
    let metadata = Source {
        url: resolved.final_url.clone(),
        etag,
        last_modified,
        fetched_at: Utc::now(),
        sha256,
        variant: resolved.variant,
        aliases: aliases.to_vec(),
        tags: Vec::new(),
    };
    storage.save_source_metadata(alias, &metadata)?;

    // Create and populate index
    spinner.set_message("Indexing content...");
    let index_path = storage.index_dir(alias)?;
    let index = SearchIndex::create(&index_path)?.with_metrics(metrics);
    index.index_blocks(alias, &parse_result.heading_blocks)?;

    spinner.finish_and_clear();

    if !quiet {
        println!(
            "{} {} ({} headings, {} lines)",
            "✓ Added".green(),
            alias.green(),
            count_headings(&llms_json.toc),
            llms_json.line_index.total_lines
        );
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

fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;

    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        let whole = bytes / KB;
        let tenths = ((bytes % KB) * 10) / KB;
        format!("{whole}.{tenths} KB")
    } else {
        let whole = bytes / MB;
        let tenths = ((bytes % MB) * 10) / MB;
        format!("{whole}.{tenths} MB")
    }
}
