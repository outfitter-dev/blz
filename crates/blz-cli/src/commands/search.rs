//! Search command implementation

use anyhow::Result;
use blz_core::{PerformanceMetrics, ResourceMonitor, SearchHit, SearchIndex, Storage};
use std::time::Instant;

use crate::output::{OutputFormat, SearchResultFormatter};

/// Search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query: String,
    pub alias: Option<String>,
    pub limit: usize,
    pub page: usize,
    pub top_percentile: Option<u8>,
    pub output: OutputFormat,
}

/// Execute a search across cached documentation
pub async fn execute(
    query: &str,
    alias: Option<&str>,
    limit: usize,
    page: usize,
    top_percentile: Option<u8>,
    output: OutputFormat,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    let options = SearchOptions {
        query: query.to_string(),
        alias: alias.map(String::from),
        limit,
        page,
        top_percentile,
        output,
    };

    let results = perform_search(&options, metrics.clone()).await?;
    format_and_display(results, &options)?;

    if let Some(monitor) = resource_monitor {
        monitor.print_resource_usage();
    }

    Ok(())
}

/// Handle default search from command line arguments
pub async fn handle_default(
    args: &[String],
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    if args.is_empty() {
        println!("Usage: blz [QUERY] [SOURCE] or blz [SOURCE] [QUERY]");
        println!("       blz search [OPTIONS] QUERY");
        return Ok(());
    }

    let storage = Storage::new()?;
    let sources = storage.list_sources()?;

    if sources.is_empty() {
        println!("No sources found. Use 'blz add ALIAS URL' to add sources.");
        return Ok(());
    }

    let (query, alias) = parse_arguments(args, &sources);

    execute(
        &query,
        alias.as_deref(),
        50,
        1,
        None,
        OutputFormat::Text,
        metrics,
        resource_monitor,
    )
    .await
}

fn parse_arguments(args: &[String], sources: &[String]) -> (String, Option<String>) {
    // Smart argument detection
    if args.len() >= 2 && sources.contains(&args[0]) {
        // Format: blz SOURCE QUERY...
        (args[1..].join(" "), Some(args[0].clone()))
    } else if args.len() >= 2 && sources.contains(&args[args.len() - 1]) {
        // Format: blz QUERY... SOURCE
        (
            args[..args.len() - 1].join(" "),
            Some(args[args.len() - 1].clone()),
        )
    } else {
        // Single query or query without known source
        (args.join(" "), None)
    }
}

struct SearchResults {
    hits: Vec<SearchHit>,
    total_lines_searched: usize,
    search_time: std::time::Duration,
    sources: Vec<String>,
}

async fn perform_search(
    options: &SearchOptions,
    metrics: PerformanceMetrics,
) -> Result<SearchResults> {
    let start_time = Instant::now();
    let storage = Storage::new()?;

    let sources = if let Some(ref alias) = options.alias {
        vec![alias.clone()]
    } else {
        storage.list_sources()?
    };

    if sources.is_empty() {
        return Err(anyhow::anyhow!(
            "No sources found. Use 'blz add' to add sources."
        ));
    }

    let mut all_hits = Vec::new();
    let mut total_lines_searched = 0usize;

    // Collect all hits from all sources
    for source in &sources {
        let index_path = storage.index_dir(source)?;
        if index_path.exists() {
            let index = SearchIndex::open(&index_path)?.with_metrics(metrics.clone());
            let hits = index.search(&options.query, Some(source), 10000)?;
            all_hits.extend(hits);

            // Count total lines for stats
            if let Ok(llms_json) = storage.load_llms_json(source) {
                total_lines_searched += llms_json.line_index.total_lines;
            }
        }
    }

    // Process results
    deduplicate_hits(&mut all_hits);
    sort_by_score(&mut all_hits);
    apply_percentile_filter(&mut all_hits, options.top_percentile);

    Ok(SearchResults {
        hits: all_hits,
        total_lines_searched,
        search_time: start_time.elapsed(),
        sources,
    })
}

fn deduplicate_hits(hits: &mut Vec<SearchHit>) {
    hits.sort_by(|a, b| {
        let cmp = a
            .alias
            .cmp(&b.alias)
            .then(a.lines.cmp(&b.lines))
            .then(a.heading_path.cmp(&b.heading_path));
        if cmp == std::cmp::Ordering::Equal {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            cmp
        }
    });

    hits.dedup_by(|a, b| {
        a.alias == b.alias && a.lines == b.lines && a.heading_path == b.heading_path
    });
}

fn sort_by_score(hits: &mut Vec<SearchHit>) {
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn apply_percentile_filter(hits: &mut Vec<SearchHit>, top_percentile: Option<u8>) {
    if let Some(percentile) = top_percentile {
        let percentile_count =
            (hits.len() as f32 * (f32::from(percentile) / 100.0)).ceil() as usize;
        hits.truncate(percentile_count.max(1));

        if hits.len() < 10 {
            eprintln!(
                "Tip: Only {} results in top {}%. Try a lower percentile or remove --top flag.",
                hits.len(),
                percentile
            );
        }
    }
}

fn format_and_display(results: SearchResults, options: &SearchOptions) -> Result<()> {
    let total_results = results.hits.len();

    // Apply pagination
    let actual_limit = if options.limit >= 10000 {
        results.hits.len()
    } else {
        options.limit
    };

    let start_idx = (options.page - 1) * actual_limit;
    let end_idx = (start_idx + actual_limit).min(results.hits.len());

    if start_idx >= results.hits.len() {
        println!(
            "Page {} is beyond available results (only {} pages available)",
            options.page,
            total_results.div_ceil(actual_limit)
        );
        return Ok(());
    }

    let page_hits = &results.hits[start_idx..end_idx];

    let formatter = SearchResultFormatter::new(options.output);
    formatter.format(
        page_hits,
        &options.query,
        total_results,
        results.total_lines_searched,
        results.search_time,
        options.limit < 10000,
        options.alias.is_some(),
        &results.sources,
        start_idx,
    )?;

    Ok(())
}
