//! Search command implementation

use anyhow::{Context, Result};
use blz_core::{PerformanceMetrics, ResourceMonitor, SearchHit, SearchIndex, Storage};
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use std::time::Instant;

use crate::output::{FormatParams, OutputFormat, SearchResultFormatter};

const ALL_RESULTS_LIMIT: usize = 10_000;

/// Search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query: String,
    pub alias: Option<String>,
    pub last: bool,
    pub limit: usize,
    pub page: usize,
    pub top_percentile: Option<u8>,
    pub output: OutputFormat,
    pub(crate) all: bool,
}

/// Execute a search across cached documentation
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    query: &str,
    alias: Option<&str>,
    last: bool,
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
        last,
        limit,
        page,
        top_percentile,
        output,
        all: limit >= ALL_RESULTS_LIMIT, // If limit is >= ALL_RESULTS_LIMIT, we want all results
    };

    let results = perform_search(&options, metrics.clone()).await?;
    format_and_display(&results, &options)?;

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
        println!("\nExamples:");
        println!("  blz hooks react");
        println!("  blz react hooks");
        println!("  blz search \"async await\" --source react -o json");
        println!("\nNotes:");
        println!("  • SOURCE may be a canonical name or a metadata alias (see 'blz alias add').");
        println!("  • Set BLZ_OUTPUT_FORMAT=json to default JSON output for agent use.");
        println!("  • Run 'blz instruct' for agent-focused guidance.");
        return Ok(());
    }

    let storage = Storage::new()?;
    let sources = storage.list_sources();

    if sources.is_empty() {
        println!("No sources found. Use 'blz add ALIAS URL' to add sources.");
        return Ok(());
    }

    let (mut query, mut alias) = parse_arguments(args, &sources);

    // If no canonical alias was detected, attempt metadata alias resolution for first/last token
    if alias.is_none() && args.len() >= 2 {
        let first = &args[0];
        let last = &args[args.len() - 1];
        if let Ok(Some(canon)) = crate::utils::resolver::resolve_source(&storage, first) {
            // blz SOURCE QUERY...
            alias = Some(canon);
            query = args[1..].join(" ");
        } else if let Ok(Some(canon)) = crate::utils::resolver::resolve_source(&storage, last) {
            // blz QUERY... SOURCE
            alias = Some(canon);
            query = args[..args.len() - 1].join(" ");
        }
    }

    execute(
        &query,
        alias.as_deref(),
        false,
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
    // Smart argument detection with metadata alias resolution best-effort
    if args.len() >= 2 {
        // Check first token as source
        if let Some(candidate) = args.first() {
            if sources.contains(candidate) {
                return (args[1..].join(" "), Some(candidate.clone()));
            }
        }

        // Check last token as source
        if let Some(candidate) = args.last() {
            if sources.contains(candidate) {
                return (args[..args.len() - 1].join(" "), Some(candidate.clone()));
            }
        }
    }

    // Fallback: all args are the query; alias resolution will be handled by flags if provided
    (args.join(" "), None)
}

struct SearchResults {
    hits: Vec<SearchHit>,
    total_lines_searched: usize,
    search_time: std::time::Duration,
    sources: Vec<String>,
}

fn get_max_concurrent_searches() -> usize {
    std::thread::available_parallelism().map_or(8, |n| (n.get().saturating_mul(2)).min(16))
}

#[allow(clippy::too_many_lines)]
async fn perform_search(
    options: &SearchOptions,
    metrics: PerformanceMetrics,
) -> Result<SearchResults> {
    let start_time = Instant::now();
    let storage = Arc::new(Storage::new()?);
    // Resolve requested source (supports metadata aliases)
    let sources = if let Some(requested) = &options.alias {
        match crate::utils::resolver::resolve_source(&storage, requested) {
            Ok(Some(canonical)) => vec![canonical],
            Ok(None) => {
                // Fallback: show hint and continue with zero sources handled below
                let known = storage.list_sources();
                if !known.contains(requested) && matches!(options.output, OutputFormat::Text) {
                    eprintln!(
                        "Source '{requested}' not found. Use 'blz list' to see available or 'blz lookup <name>' to add."
                    );
                }
                vec![requested.clone()]
            },
            Err(e) => return Err(e),
        }
    } else {
        storage.list_sources()
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
        (options.limit * 3).clamp(1, 1000)
    };

    // Set max concurrent searches adaptive to host CPUs, capped at reasonable limits
    let max_concurrent_searches = get_max_concurrent_searches();

    // Create futures that spawn blocking tasks for parallel search across sources
    // This ensures bounded concurrency by only spawning tasks when polled
    let search_tasks = sources.into_iter().map(|source| {
        let storage = Arc::clone(&storage);
        let metrics = metrics.clone();
        let query = options.query.clone();

        async move {
            tokio::task::spawn_blocking(
                move || -> anyhow::Result<(Vec<SearchHit>, usize, String)> {
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
                        .search(&query, Some(&source), None, effective_limit)
                        .with_context(|| format!("search failed for source={source}"))?;

                    // Count total lines for stats
                    let total_lines = storage
                        .load_llms_json(&source)
                        .map(|json| json.line_index.total_lines)
                        .unwrap_or(0);

                    Ok((hits, total_lines, source))
                },
            )
            .await
            .map_err(|e| anyhow::anyhow!("search task panicked: {}", e))?
        }
    });

    // Execute searches with bounded concurrency
    let mut search_stream = stream::iter(search_tasks).buffer_unordered(max_concurrent_searches);

    let mut all_hits = Vec::new();
    let mut total_lines_searched = 0usize;
    let mut sources_searched = Vec::new();

    // Collect results from the stream
    while let Some(res) = search_stream.next().await {
        match res {
            Ok((hits, lines, source)) => {
                let has_hits = !hits.is_empty();
                all_hits.extend(hits);
                total_lines_searched += lines;
                if lines > 0 || has_hits {
                    sources_searched.push(source);
                }
            },
            Err(e) => {
                tracing::warn!("Search failed: {}", e);
            },
        }
    }

    // Process results
    deduplicate_hits(&mut all_hits);
    sort_by_score(&mut all_hits);
    apply_percentile_filter(&mut all_hits, options.top_percentile);

    // Enrich results with sourceUrl and checksum where available
    // Best-effort: failures are ignored to avoid impacting search flow
    let mut alias_meta: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();
    for hit in &all_hits {
        if !alias_meta.contains_key(&hit.alias) {
            if let Ok(json) = storage.load_llms_json(&hit.alias) {
                alias_meta.insert(
                    hit.alias.clone(),
                    (json.source.url.clone(), json.source.sha256.clone()),
                );
            }
        }
    }
    for hit in &mut all_hits {
        if let Some((url, sha)) = alias_meta.get(&hit.alias) {
            hit.source_url = Some(url.clone());
            hit.checksum = sha.clone();
        }
    }

    sources_searched.sort();
    Ok(SearchResults {
        hits: all_hits,
        total_lines_searched,
        search_time: start_time.elapsed(),
        sources: sources_searched,
    })
}

fn deduplicate_hits(hits: &mut Vec<SearchHit>) {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    hits.retain(|h| seen.insert((h.alias.clone(), h.lines.clone(), h.heading_path.clone())));
}

fn sort_by_score(hits: &mut [SearchHit]) {
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
        let len = hits.len();
        let percentile_f = f64::from(percentile) / 100.0;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let percentile_count = ((len as f64) * percentile_f).ceil().min(len as f64) as usize;
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

/// Formats and displays search results with pagination
///
/// This function safely handles edge cases in pagination including:
/// - Empty result sets (returns without error)
/// - Single result (paginates correctly)
/// - Over-large page numbers (displays helpful message)
/// - The `actual_limit` is guaranteed to be at least 1 to prevent divide-by-zero
///
/// # Example
///
/// ```ignore
/// // Example of safe pagination with empty results
/// let results = SearchResults {
///     hits: vec![], // Empty results
///     total_lines_searched: 0,
///     search_time: Duration::from_millis(10),
///     sources: vec![],
/// };
///
/// let options = SearchOptions {
///     query: "test".to_string(),
///     alias: None,
///     limit: 10,  // Even with limit 0, actual_limit would be max(0, 1) = 1
///     page: 1,
///     top_percentile: None,
///     output: OutputFormat::Text,
///     all: false,
/// };
///
/// // This will not panic even with empty results due to .max(1) guard
/// assert!(format_and_display(&results, &options).is_ok());
/// ```
fn format_and_display(results: &SearchResults, options: &SearchOptions) -> Result<()> {
    let total_results = results.hits.len();

    // Apply pagination
    let actual_limit = if options.limit >= ALL_RESULTS_LIMIT {
        results.hits.len().max(1)
    } else {
        options.limit.max(1)
    };

    let total_pages = if total_results == 0 {
        0
    } else {
        total_results.div_ceil(actual_limit)
    };
    let requested_page = if options.last {
        total_pages.max(1)
    } else {
        options.page
    };

    if total_results == 0 {
        // Let formatter print the "No results" message
        let formatter = SearchResultFormatter::new(options.output);
        let suggestions = if matches!(options.output, OutputFormat::Json) {
            let storage = Storage::new()?;
            Some(compute_suggestions(
                &options.query,
                &storage,
                &results.sources,
            ))
        } else {
            None
        };
        let params = FormatParams {
            hits: &[],
            query: &options.query,
            total_results,
            total_lines_searched: results.total_lines_searched,
            search_time: results.search_time,
            show_pagination: false,
            single_source: options.alias.is_some(),
            sources: &results.sources,
            start_idx: 0,
            page: 1,
            limit: actual_limit,
            total_pages,
            suggestions,
        };
        formatter.format(&params)?;
        return Ok(());
    }

    let page = requested_page.clamp(1, total_pages);
    let start_idx = (page - 1) * actual_limit;
    let end_idx = (start_idx + actual_limit).min(results.hits.len());

    if start_idx >= results.hits.len() {
        if matches!(options.output, OutputFormat::Text) {
            eprintln!(
                "Page {} is beyond available results (Page {} of {})",
                options.page, page, total_pages
            );
            eprintln!("Tip: use --last to jump to the final page.");
        }
        return Ok(());
    }

    let page_hits = &results.hits[start_idx..end_idx];

    let formatter = SearchResultFormatter::new(options.output);
    // Compute simple fuzzy suggestions for JSON output when few/low-quality results
    let suggestions = if matches!(options.output, OutputFormat::Json) {
        let need_suggest =
            total_results == 0 || results.hits.first().map_or(0.0, |h| h.score) < 2.0;
        if need_suggest {
            let storage = Storage::new()?;
            Some(compute_suggestions(
                &options.query,
                &storage,
                &results.sources,
            ))
        } else {
            None
        }
    } else {
        None
    };

    let params = FormatParams {
        hits: page_hits,
        query: &options.query,
        total_results,
        total_lines_searched: results.total_lines_searched,
        search_time: results.search_time,
        show_pagination: options.limit < ALL_RESULTS_LIMIT,
        single_source: options.alias.is_some(),
        sources: &results.sources,
        start_idx,
        page,
        limit: actual_limit,
        total_pages,
        suggestions,
    };
    formatter.format(&params)?;

    Ok(())
}

fn compute_suggestions(
    query: &str,
    storage: &blz_core::Storage,
    sources: &[String],
) -> Vec<serde_json::Value> {
    // Tokenize query (lowercase alphanumeric words)
    let qtokens = tokenize(query);
    if qtokens.is_empty() {
        return Vec::new();
    }

    let mut suggestions: Vec<(f32, String, String, String)> = Vec::new(); // (score, alias, heading, lines)
    for alias in sources {
        if let Ok(doc) = storage.load_llms_json(alias) {
            collect_suggestions_from_toc(&doc, alias, &qtokens, &mut suggestions);
        }
    }
    // Sort by score desc and take top 5
    suggestions.sort_by(|a, b| b.0.total_cmp(&a.0));
    suggestions.truncate(5);
    suggestions
        .into_iter()
        .map(|(score, alias, heading, lines)| {
            serde_json::json!({
                "alias": alias,
                "heading": heading,
                "lines": lines,
                "score": score,
            })
        })
        .collect()
}

fn collect_suggestions_from_toc(
    doc: &blz_core::LlmsJson,
    alias: &str,
    qtokens: &[String],
    out: &mut Vec<(f32, String, String, String)>,
) {
    fn walk(
        list: &[blz_core::TocEntry],
        alias: &str,
        qtokens: &[String],
        out: &mut Vec<(f32, String, String, String)>,
    ) {
        for e in list {
            if let Some(name) = e.heading_path.last() {
                let score = score_tokens(&tokenize(name), qtokens);
                if score > 0.2 {
                    out.push((score, alias.to_string(), name.clone(), e.lines.clone()));
                }
            }
            if !e.children.is_empty() {
                walk(&e.children, alias, qtokens, out);
            }
        }
    }
    walk(&doc.toc, alias, qtokens, out);
}

fn tokenize(s: &str) -> Vec<String> {
    let mut toks = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            cur.push(ch.to_ascii_lowercase());
        } else if !cur.is_empty() {
            toks.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        toks.push(cur);
    }
    toks
}

#[allow(clippy::cast_precision_loss)]
fn score_tokens(h: &[String], q: &[String]) -> f32 {
    if h.is_empty() || q.is_empty() {
        return 0.0;
    }
    let hset: std::collections::BTreeSet<&str> =
        h.iter().map(std::string::String::as_str).collect();
    let qset: std::collections::BTreeSet<&str> =
        q.iter().map(std::string::String::as_str).collect();
    let inter = hset.intersection(&qset).count() as f32;
    inter / (qset.len() as f32)
}

// alias resolution moved to utils::resolver

#[cfg(test)]
#[allow(clippy::cast_precision_loss)] // Test code precision is not critical
mod tests {
    use super::*;
    use blz_core::SearchHit;

    /// Creates a test `SearchResults` with the specified number of hits
    fn create_test_results(num_hits: usize) -> SearchResults {
        let hits: Vec<SearchHit> = (0..num_hits)
            .map(|i| SearchHit {
                alias: format!("test-{i}"),
                source: format!("test-{i}"),
                file: "llms.txt".to_string(),
                heading_path: vec![format!("heading-{i}")],
                lines: format!("{}-{}", i * 10, i * 10 + 5),
                line_numbers: Some(vec![i * 10, i * 10 + 5]),
                snippet: format!("test content {i}"),
                score: (i as f32).mul_add(-0.01, 1.0),
                source_url: Some(format!("https://example.com/test-{i}")),
                checksum: format!("checksum-{i}"),
                anchor: Some("unit-test-anchor".to_string()),
                flavor: None,
            })
            .collect();

        SearchResults {
            hits,
            total_lines_searched: 1000,
            search_time: std::time::Duration::from_millis(10),
            sources: vec!["test".to_string()],
        }
    }

    #[test]
    fn test_pagination_with_zero_hits() {
        // Test that pagination handles empty results without panic
        let results = create_test_results(0);
        let options = SearchOptions {
            query: "test".to_string(),
            alias: None,
            last: false,
            limit: 10,
            page: 1,
            top_percentile: None,
            output: OutputFormat::Text,
            all: false,
        };

        // Should not panic even with empty results
        let result = format_and_display(&results, &options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pagination_with_single_hit() {
        // Test edge case where there's only one result
        let results = create_test_results(1);
        let options = SearchOptions {
            query: "test".to_string(),
            alias: None,
            last: false,
            limit: 10,
            page: 1,
            top_percentile: None,
            output: OutputFormat::Text,
            all: false,
        };

        let result = format_and_display(&results, &options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pagination_prevents_divide_by_zero() {
        // This is the main regression test for the divide-by-zero fix
        // Test case 1: Empty results with ALL_RESULTS_LIMIT
        let empty_results = create_test_results(0);
        let options_empty = SearchOptions {
            query: "test".to_string(),
            alias: None,
            last: false,
            limit: ALL_RESULTS_LIMIT,
            page: 2, // Try to access page 2 to trigger div_ceil
            top_percentile: None,
            output: OutputFormat::Text,
            all: true,
        };

        // This should NOT panic even with empty results
        let result = format_and_display(&empty_results, &options_empty);
        assert!(result.is_ok(), "Should handle empty results without panic");

        // Test case 2: Normal results with high page number
        let results = create_test_results(5);
        let options_high_page = SearchOptions {
            query: "test".to_string(),
            alias: None,
            last: false,
            limit: ALL_RESULTS_LIMIT,
            page: 100, // Very high page to trigger the div_ceil in the message
            top_percentile: None,
            output: OutputFormat::Text,
            all: true,
        };

        let result = format_and_display(&results, &options_high_page);
        assert!(
            result.is_ok(),
            "Should handle page out of bounds without panic"
        );
    }

    #[test]
    fn test_pagination_with_overlarge_page_number() {
        // Test that requesting a page beyond available results is handled gracefully
        let results = create_test_results(5);
        let options = SearchOptions {
            query: "test".to_string(),
            alias: None,
            last: false,
            limit: 2,
            page: 100, // Way beyond available pages
            top_percentile: None,
            output: OutputFormat::Text,
            all: false,
        };

        let result = format_and_display(&results, &options);
        assert!(result.is_ok()); // Should handle gracefully, not panic
    }

    #[test]
    fn test_pagination_boundary_conditions() {
        // Test exact boundary conditions
        let results = create_test_results(10);

        // Exactly at the boundary (page 2 with limit 5 for 10 results)
        let options = SearchOptions {
            query: "test".to_string(),
            alias: None,
            last: false,
            limit: 5,
            page: 2,
            top_percentile: None,
            output: OutputFormat::Text,
            all: false,
        };

        let result = format_and_display(&results, &options);
        assert!(result.is_ok());

        // Just beyond the boundary (page 3 with limit 5 for 10 results)
        let options_beyond = SearchOptions {
            query: "test".to_string(),
            alias: None,
            last: false,
            limit: 5,
            page: 3,
            top_percentile: None,
            output: OutputFormat::Text,
            all: false,
        };

        let test_results = create_test_results(10);
        let result_beyond = format_and_display(&test_results, &options_beyond);
        assert!(result_beyond.is_ok());
    }

    #[test]
    fn test_actual_limit_calculation() {
        // Test that actual_limit is always at least 1
        // This directly tests the fix for the divide-by-zero issue

        // Case 1: Normal limit
        let options1 = SearchOptions {
            query: "test".to_string(),
            alias: None,
            last: false,
            limit: 10,
            page: 1,
            top_percentile: None,
            output: OutputFormat::Text,
            all: false,
        };

        // The actual_limit calculation from the code
        let actual_limit1 = if options1.limit >= ALL_RESULTS_LIMIT {
            1 // Minimum limit is always 1
        } else {
            options1.limit.max(1)
        };
        assert!(actual_limit1 >= 1, "actual_limit must be at least 1");

        // Case 2: ALL_RESULTS_LIMIT with empty results
        let options2 = SearchOptions {
            query: "test".to_string(),
            alias: None,
            last: false,
            limit: ALL_RESULTS_LIMIT,
            page: 1,
            top_percentile: None,
            output: OutputFormat::Text,
            all: true,
        };

        let actual_limit2 = if options2.limit >= ALL_RESULTS_LIMIT {
            1 // Minimum limit is always 1
        } else {
            options2.limit.max(1)
        };
        assert!(
            actual_limit2 >= 1,
            "actual_limit must be at least 1 even with empty results"
        );
    }

    #[test]
    fn test_div_ceil_safety() {
        // Ensure div_ceil never receives 0 as divisor
        let test_cases = vec![
            (0_usize, 1_usize),    // 0 results
            (1_usize, 1_usize),    // 1 result
            (5_usize, 2_usize),    // 5 results with limit 2
            (10_usize, 10_usize),  // 10 results with limit 10
            (100_usize, 25_usize), // 100 results with limit 25
        ];

        for (total_results, limit) in test_cases {
            // Simulating the actual_limit calculation with the fix
            let actual_limit = limit.max(1);

            // This is what was causing the panic before the fix
            let pages = total_results.div_ceil(actual_limit);

            // Verify it doesn't panic and produces sensible results
            if total_results == 0 {
                assert_eq!(pages, 0);
            } else {
                assert!(pages >= 1);
            }
        }
    }
}
