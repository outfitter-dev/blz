//! Search command implementation

use anyhow::{Context, Result};
use blz_core::{PerformanceMetrics, ResourceMonitor, SearchHit, SearchIndex, Storage};
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use std::time::Instant;

use crate::output::{OutputFormat, SearchResultFormatter};

const ALL_RESULTS_LIMIT: usize = 10_000;

/// Search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query: String,
    pub alias: Option<String>,
    pub limit: usize,
    pub page: usize,
    pub top_percentile: Option<u8>,
    pub output: OutputFormat,
    pub(crate) all: bool,
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
        all: limit >= ALL_RESULTS_LIMIT, // If limit is >= ALL_RESULTS_LIMIT, we want all results
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
    let storage = Arc::new(Storage::new()?);

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

    // Calculate effective limit to prevent over-fetching
    // If we want all results, use 10k limit. Otherwise, use (limit * 3) capped at 1000
    // The 3x multiplier provides buffer for good results after deduplication/sorting
    let effective_limit = if options.all {
        ALL_RESULTS_LIMIT
    } else {
        ((options.limit * 3).min(1000)).max(1)
    };

    // Set max concurrent searches to prevent resource exhaustion
    const MAX_CONCURRENT_SEARCHES: usize = 8;

    // Create blocking tasks for parallel search across sources (avoid blocking the async runtime)
    let search_tasks = sources.into_iter().map(|source| {
        let storage = Arc::clone(&storage);
        let metrics = metrics.clone();
        let query = options.query.clone();

        tokio::task::spawn_blocking(move || -> anyhow::Result<(Vec<SearchHit>, usize, String)> {
            let index_path = storage.index_dir(&source)?;
            if !index_path.exists() {
                return Ok((Vec::new(), 0, source));
            }

            let index = SearchIndex::open(&index_path)
                .with_context(|| {
                    format!(
                        "open index for source={} at {}",
                        source,
                        index_path.display()
                    )
                })?
                .with_metrics(metrics);
            let hits = index
                .search(&query, Some(&source), effective_limit)
                .with_context(|| format!("search failed for source={}", source))?;

            // Count total lines for stats
            let total_lines = storage
                .load_llms_json(&source)
                .map(|json| json.line_index.total_lines)
                .unwrap_or(0);

            Ok((hits, total_lines, source))
        })
    });

    // Execute searches with bounded concurrency
    let mut search_stream = stream::iter(search_tasks).buffer_unordered(MAX_CONCURRENT_SEARCHES);

    let mut all_hits = Vec::new();
    let mut total_lines_searched = 0usize;
    let mut sources_searched = Vec::new();

    // Collect results from the stream
    while let Some(join_res) = search_stream.next().await {
        match join_res {
            Ok(Ok((hits, lines, source))) => {
                let has_hits = !hits.is_empty();
                all_hits.extend(hits);
                total_lines_searched += lines;
                if lines > 0 || has_hits {
                    sources_searched.push(source);
                }
            },
            Ok(Err(e)) => {
                tracing::warn!("Search failed: {}", e);
            },
            Err(join_err) => {
                tracing::warn!("Search task panicked: {}", join_err);
            },
        }
    }

    // Process results
    deduplicate_hits(&mut all_hits);
    sort_by_score(&mut all_hits);
    apply_percentile_filter(&mut all_hits, options.top_percentile);

    sources_searched.sort();
    Ok(SearchResults {
        hits: all_hits,
        total_lines_searched,
        search_time: start_time.elapsed(),
        sources: sources_searched,
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
    // Sort by score with deterministic tie-breakers
    hits.sort_by(|a, b| {
        // First by score (descending) with total ordering on floats
        match b.score.total_cmp(&a.score) {
            std::cmp::Ordering::Equal => {
                // Then by alias (ascending)
                a.alias.cmp(&b.alias)
                    // Then by line number (ascending)
                    .then(a.lines.cmp(&b.lines))
                    // Finally by heading path (ascending)
                    .then(a.heading_path.cmp(&b.heading_path))
            },
            ordering => ordering,
        }
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
    let actual_limit = if options.limit >= ALL_RESULTS_LIMIT {
        results.hits.len().max(1)
    } else {
        options.limit.max(1)
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
        options.limit < ALL_RESULTS_LIMIT,
        options.alias.is_some(),
        &results.sources,
        start_idx,
    )?;

    Ok(())
}
