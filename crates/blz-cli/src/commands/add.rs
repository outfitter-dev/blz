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
/// * `auto_yes` - Auto-select the best flavor without prompts
/// * `metrics` - Performance metrics collector
pub async fn execute(
    alias: &str,
    url: &str,
    auto_yes: bool,
    metrics: PerformanceMetrics,
) -> Result<()> {
    let _ = auto_yes;
    // Normalize the alias to kebab-case lowercase
    let normalized_alias = normalize_alias(alias);

    // Show normalization if it changed
    if normalized_alias != alias {
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
                println!(
                    "Warning: URL scheme '{other}' may not be supported for fetching ({url}). \n \
                     If this is a local file, consider hosting llms.txt or using a supported HTTP URL."
                );
            },
        }
    } else {
        println!("Warning: URL appears invalid: {url}");
    }

    fetch_and_index_variants(&normalized_alias, url, fetcher, metrics).await
}

async fn fetch_and_index_variants(
    alias: &str,
    url: &str,
    fetcher: Fetcher,
    metrics: PerformanceMetrics,
) -> Result<()> {
    let mut candidates = discover_flavor_candidates(&fetcher, url).await?;
    if candidates.is_empty() {
        anyhow::bail!("No reachable llms.txt variants discovered for {url}");
    }

    let storage = Storage::new()?;
    let index_path = storage.index_dir(alias)?;
    let index = SearchIndex::create(&index_path)?.with_metrics(metrics);

    let spinner = create_spinner("Fetching documentation variants...");
    let mut summaries = Vec::new();

    for candidate in &mut candidates {
        spinner.set_message(format!("Fetching {}", candidate.file_name));

        match crate::utils::http::head_with_retries(&fetcher, &candidate.url, 3, 200).await {
            Ok(meta) => {
                let status = meta.status;
                if !(200..=399).contains(&status) {
                    anyhow::bail!(
                        "Preflight failed (HTTP {status}) for {}. Verify the URL or try 'blz lookup' to find a valid source.",
                        candidate.url
                    );
                }
            },
            Err(err) => {
                println!(
                    "Warning: Preflight HEAD request failed for {}: {err}",
                    candidate.url
                );
            },
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
                anyhow::bail!(
                    "Server returned 304 Not Modified on initial fetch for '{}'; please retry or verify the URL",
                    candidate.url
                );
            },
        };

        spinner.set_message(format!("Parsing {}", candidate.file_name));
        let mut parser = MarkdownParser::new()?;
        let parse_result = parser.parse(&content)?;

        let (file_name, flavor_id, summary_label) =
            match Flavor::from_identifier(&candidate.flavor_id) {
                Some(flavor) => {
                    let id = flavor.as_str();
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

    let summary_text = summaries
        .iter()
        .map(|(flavor, headings, lines)| format!("{flavor}: {headings} headings, {lines} lines"))
        .collect::<Vec<_>>()
        .join(", ");

    println!("{} {} ({})", "✓ Added".green(), alias.green(), summary_text);

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
