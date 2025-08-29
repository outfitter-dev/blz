//! Add command implementation

use anyhow::Result;
use blz_core::{
    Fetcher, FileInfo, FlavorInfo, LineIndex, LlmsJson, MarkdownParser, PerformanceMetrics,
    ResourceMonitor, SearchIndex, Source, Storage,
};
use chrono::Utc;
use colored::Colorize;
use dialoguer::Select;
use indicatif::{ProgressBar, ProgressStyle};

use crate::utils::validation::validate_alias;

/// Add a new documentation source
///
/// # Arguments
///
/// * `alias` - Local alias for the source
/// * `url` - URL to fetch llms.txt from
/// * `auto_yes` - Auto-select the best flavor without prompts
/// * `metrics` - Performance metrics collector
/// * `resource_monitor` - Optional resource usage monitor
pub async fn execute(
    alias: &str,
    url: &str,
    auto_yes: bool,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    validate_alias(alias)?;

    let fetcher = Fetcher::new()?;
    let final_url = select_flavor(&fetcher, url, auto_yes).await?;

    fetch_and_index(alias, &final_url, fetcher, metrics, resource_monitor).await
}

async fn select_flavor(fetcher: &Fetcher, url: &str, auto_yes: bool) -> Result<String> {
    // Check if the user specified an exact llms.txt variant
    let is_exact_file = url
        .split('/')
        .next_back()
        .is_some_and(|filename| filename.starts_with("llms") && filename.ends_with(".txt"));

    if is_exact_file {
        return Ok(url.to_string());
    }

    // Smart detection: check for flavors
    let pb = create_spinner("Checking for available documentation flavors...");

    let flavors = match fetcher.check_flavors(url).await {
        Ok(flavors) if !flavors.is_empty() => flavors,
        Ok(_) => {
            pb.finish_with_message("No llms.txt variants found, using provided URL");
            vec![FlavorInfo {
                name: "llms.txt".to_string(),
                size: None,
                url: url.to_string(),
            }]
        },
        Err(e) => {
            pb.finish_with_message(format!("Failed to check flavors: {e}"));
            vec![FlavorInfo {
                name: "llms.txt".to_string(),
                size: None,
                url: url.to_string(),
            }]
        },
    };

    pb.finish();

    select_from_flavors(flavors, auto_yes)
}

fn select_from_flavors(flavors: Vec<FlavorInfo>, auto_yes: bool) -> Result<String> {
    if flavors.len() == 1 {
        return Ok(flavors[0].url.clone());
    }

    if auto_yes {
        println!("Auto-selecting: {}", flavors[0]);
        return Ok(flavors[0].url.clone());
    }

    // Interactive selection
    println!("Found {} versions:", flavors.len());

    let flavor_displays: Vec<String> = flavors
        .iter()
        .enumerate()
        .map(|(i, flavor)| {
            if i == 0 {
                format!("→ {flavor} [default]")
            } else {
                format!("  {flavor}")
            }
        })
        .collect();

    let selection = Select::new()
        .with_prompt("Select version")
        .items(&flavor_displays)
        .default(0)
        .interact()?;

    Ok(flavors[selection].url.clone())
}

async fn fetch_and_index(
    alias: &str,
    url: &str,
    fetcher: Fetcher,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    let pb = create_spinner(format!("Fetching {url}").as_str());

    let fetch_result = fetcher.fetch_with_cache(url, None, None).await?;

    let (content, sha256, etag, last_modified) = match fetch_result {
        blz_core::FetchResult::Modified {
            content,
            sha256,
            etag,
            last_modified,
        } => (content, sha256, etag, last_modified),
        blz_core::FetchResult::NotModified { .. } => {
            // Defensive: should not happen on initial fetch, but handle gracefully
            anyhow::bail!("Server returned 304 Not Modified on initial fetch for '{}'; please retry or verify the URL", url)
        },
    };
    pb.set_message("Parsing markdown");

    let mut parser = MarkdownParser::new()?;
    let parse_result = parser.parse(&content)?;

    pb.set_message("Building index");

    let storage = Storage::new()?;
    storage.save_llms_txt(alias, &content)?;

    let llms_json = create_llms_json(
        alias,
        url,
        sha256.clone(),
        etag.clone(),
        last_modified.clone(),
        parse_result.clone(),
    );
    storage.save_llms_json(alias, &llms_json)?;

    // Also save metadata for efficient update checking
    let metadata = Source {
        url: url.to_string(),
        etag,
        last_modified,
        fetched_at: Utc::now(),
        sha256,
    };
    storage.save_source_metadata(alias, &metadata)?;

    let index_path = storage.index_dir(alias)?;
    let index = SearchIndex::create(&index_path)?.with_metrics(metrics);
    index.index_blocks(alias, "llms.txt", &parse_result.heading_blocks)?;

    pb.finish_with_message(format!(
        "✓ Added {} ({} headings, {} lines)",
        alias.green(),
        llms_json.toc.len(),
        llms_json.line_index.total_lines
    ));

    if let Some(monitor) = resource_monitor {
        monitor.print_resource_usage();
    }

    Ok(())
}

fn create_llms_json(
    alias: &str,
    url: &str,
    sha256: String,
    etag: Option<String>,
    last_modified: Option<String>,
    parse_result: blz_core::ParseResult,
) -> LlmsJson {
    LlmsJson {
        alias: alias.to_string(),
        source: Source {
            url: url.to_string(),
            etag,
            last_modified,
            fetched_at: Utc::now(),
            sha256: sha256.clone(),
        },
        toc: parse_result.toc,
        files: vec![FileInfo {
            path: "llms.txt".to_string(),
            sha256,
        }],
        line_index: LineIndex {
            total_lines: parse_result.line_count,
            byte_offsets: false,
        },
        diagnostics: parse_result.diagnostics,
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
