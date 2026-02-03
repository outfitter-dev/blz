//! Query command implementation - full-text search across sources
//!
//! This module provides the `blz query` command for searching documentation content.
//! Unlike `find`, the `query` command is explicitly for text searches and rejects
//! citation patterns with helpful error messages.
//!
//! # Examples
//!
//! ```bash
//! blz query "async patterns"
//! blz query "useEffect cleanup" --source react
//! blz query "error handling" -H 2,3 --json
//! ```

use std::io;

use anyhow::{Result, bail};
use blz_core::numeric::percent_to_u8;
use blz_core::{PerformanceMetrics, ResourceMonitor, SearchHit};
use clap::Args;

use crate::args::{ContextMode, ShowComponent};
use crate::config::{
    ContentConfig, DisplayConfig, QueryExecutionConfig, SearchConfig, SnippetConfig,
};
use crate::output::shapes::{ContextInfo, SearchHitOutput, SearchOutput};
use crate::output::{OutputFormat, SearchRenderOptions, render_search_with_options};
use crate::utils::cli_args::FormatArg;
use crate::utils::heading_filter::HeadingLevelFilter;
use crate::utils::preferences::CliPreferences;

/// Arguments for `blz query` (full-text search, rejects citations).
#[derive(Args, Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct QueryArgs {
    /// Search query terms (not citations - use `get` for retrieval).
    #[arg(value_name = "QUERY", required = true, num_args = 1..)]
    pub inputs: Vec<String>,

    /// Filter by source(s) - comma-separated or repeated (-s a -s b).
    #[arg(
        long = "source",
        short = 's',
        visible_alias = "alias",
        visible_alias = "sources",
        value_name = "SOURCE",
        value_delimiter = ','
    )]
    pub sources: Vec<String>,

    /// Maximum number of results per page.
    #[arg(short = 'n', long, value_name = "COUNT", conflicts_with = "all")]
    pub limit: Option<usize>,

    /// Show all results - no limit.
    #[arg(long, conflicts_with = "limit")]
    pub all: bool,

    /// Page number for pagination.
    #[arg(long, default_value = "1")]
    pub page: usize,

    /// Show only top N percentile of results (1-100).
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
    pub top: Option<u8>,

    /// Filter results by heading level.
    ///
    /// Supports comparison operators (<=2, >2, >=3, <4, =2), lists (1,2,3), and ranges (1-3).
    #[arg(short = 'H', long = "heading-level", value_name = "FILTER")]
    pub heading_level: Option<String>,

    /// Output format (text, json, jsonl).
    #[command(flatten)]
    pub format: FormatArg,

    /// Additional columns to include in text output.
    #[arg(long = "show", value_enum, value_delimiter = ',', env = "BLZ_SHOW")]
    pub show: Vec<ShowComponent>,

    /// Hide the summary/footer line.
    #[arg(long = "no-summary")]
    pub no_summary: bool,

    /// Number of decimal places to show for scores (0-4).
    #[arg(
        long = "score-precision",
        value_name = "PLACES",
        value_parser = clap::value_parser!(u8).range(0..=4),
        env = "BLZ_SCORE_PRECISION"
    )]
    pub score_precision: Option<u8>,

    /// Maximum snippet lines to display around a hit (1-10).
    #[arg(
        long = "snippet-lines",
        value_name = "LINES",
        value_parser = clap::value_parser!(u8).range(1..=10),
        env = "BLZ_SNIPPET_LINES",
        default_value_t = 3,
        hide = true
    )]
    pub snippet_lines: u8,

    /// Maximum total characters in snippet (range: 50-1000, default: 200).
    #[arg(
        long = "max-chars",
        value_name = "CHARS",
        env = "BLZ_MAX_CHARS",
        value_parser = clap::value_parser!(usize)
    )]
    pub max_chars: Option<usize>,

    /// Print LINES lines of context (both before and after match). Same as -C.
    ///
    /// Use "all" to expand to the full heading section containing the match.
    /// If no heading encompasses the match, returns only the matched lines.
    #[arg(
        short = 'C',
        long = "context",
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "5",
        allow_hyphen_values = false,
        conflicts_with_all = ["block", "context_deprecated"],
        display_order = 30
    )]
    pub context: Option<ContextMode>,

    /// Deprecated: use -C or --context instead.
    #[arg(
        short = 'c',
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "5",
        allow_hyphen_values = false,
        conflicts_with_all = ["block", "context"],
        hide = true,
        display_order = 100
    )]
    pub context_deprecated: Option<ContextMode>,

    /// Print LINES lines of context after each match.
    #[arg(
        short = 'A',
        long = "after-context",
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "5",
        allow_hyphen_values = false,
        conflicts_with = "block",
        display_order = 31
    )]
    pub after_context: Option<usize>,

    /// Print LINES lines of context before each match.
    #[arg(
        short = 'B',
        long = "before-context",
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "5",
        allow_hyphen_values = false,
        conflicts_with = "block",
        display_order = 32
    )]
    pub before_context: Option<usize>,

    /// Expand to the full heading section containing each hit.
    ///
    /// If no heading encompasses the range, returns only the requested lines.
    /// Legacy alias for --context all.
    #[arg(long, conflicts_with_all = ["context", "context_deprecated", "after_context", "before_context"], display_order = 33)]
    pub block: bool,

    /// Maximum number of lines to include when using block expansion.
    #[arg(
        long = "max-lines",
        value_name = "LINES",
        value_parser = clap::value_parser!(usize),
        display_order = 34
    )]
    pub max_lines: Option<usize>,

    /// Restrict matches to heading text only.
    #[arg(long = "headings-only", display_order = 35)]
    pub headings_only: bool,

    /// Don't save this search to history.
    #[arg(long = "no-history")]
    pub no_history: bool,

    /// Copy results to clipboard using OSC 52 escape sequence.
    #[arg(long)]
    pub copy: bool,

    /// Show detailed timing breakdown for performance analysis.
    #[arg(long)]
    pub timing: bool,
}

use super::search::{
    ALL_RESULTS_LIMIT, DEFAULT_SCORE_PRECISION, SearchOptions, SearchResults, clamp_max_chars,
    copy_results_to_clipboard, default_search_limit, perform_search, resolve_show_components,
};

/// Detect if input looks like a citation pattern: `alias:digits-digits`
fn looks_like_citation(input: &str) -> bool {
    let Some((alias, ranges)) = input.split_once(':') else {
        return false;
    };

    // Alias must be non-empty and contain only lowercase letters, digits, hyphens, underscores
    if alias.is_empty()
        || !alias
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return false;
    }

    // Ranges must be non-empty
    if ranges.is_empty() {
        return false;
    }

    // Check if first range looks like digits-digits
    let first_range = ranges.split(',').next().unwrap_or("");
    if let Some((start, end)) = first_range.split_once('-') {
        start.chars().all(|c| c.is_ascii_digit())
            && end.chars().all(|c| c.is_ascii_digit())
            && !start.is_empty()
            && !end.is_empty()
    } else {
        false
    }
}

/// Parse heading level filter from string.
fn parse_heading_filter(filter_str: Option<&str>) -> Result<Option<HeadingLevelFilter>> {
    filter_str
        .map(|s| {
            s.parse::<HeadingLevelFilter>()
                .map_err(|e| anyhow::anyhow!("Invalid heading level filter: {e}"))
        })
        .transpose()
}

/// Execute the query command for full-text search
///
/// This command is specifically for text searches and will reject citation patterns
/// Dispatch a Query command.
pub async fn dispatch(
    args: QueryArgs,
    quiet: bool,
    prefs: &mut CliPreferences,
    metrics: PerformanceMetrics,
) -> Result<()> {
    let resolved_format = args.format.resolve(quiet);
    let merged_context = crate::args::merge_context_flags(
        args.context,
        args.context_deprecated,
        args.after_context,
        args.before_context,
    );

    // Parse heading filter
    let heading_filter = parse_heading_filter(args.heading_level.as_deref())?;

    // Calculate effective limit
    let effective_limit = if args.all {
        ALL_RESULTS_LIMIT
    } else {
        args.limit.unwrap_or_else(default_search_limit)
    };

    // Build config structs
    let search = SearchConfig::new()
        .with_limit(effective_limit)
        .with_page(args.page)
        .with_top_percentile(args.top)
        .with_heading_filter(heading_filter)
        .with_headings_only(args.headings_only)
        .with_last(false) // query command doesn't support --last flag
        .with_no_history(args.no_history);

    let display = DisplayConfig::new(resolved_format)
        .with_show(args.show.clone())
        .with_no_summary(args.no_summary)
        .with_timing(args.timing)
        .with_quiet(quiet);

    let snippet = SnippetConfig::new()
        .with_lines(args.snippet_lines)
        .with_max_chars(args.max_chars.map_or(200, clamp_max_chars))
        .with_score_precision(args.score_precision);

    let content = ContentConfig::new()
        .with_context(merged_context)
        .with_max_lines(args.max_lines)
        .with_copy(args.copy)
        .with_block(args.block);

    let config = QueryExecutionConfig::new(search, display, snippet, content);

    execute(
        &args.inputs,
        &args.sources,
        &config,
        Some(prefs),
        metrics,
        None,
    )
    .await
}

/// Execute the query command for full-text search.
///
/// This command rejects citation patterns with a helpful error message
/// suggesting `blz get` instead.
///
/// # Errors
///
/// Returns an error if:
/// - The input looks like a citation pattern (suggests using `blz get`)
/// - The query is empty
/// - The search fails
pub async fn execute(
    inputs: &[String],
    sources: &[String],
    config: &QueryExecutionConfig,
    prefs: Option<&mut CliPreferences>,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    if inputs.is_empty() {
        bail!("query requires at least one search term");
    }

    // Check if any input looks like a citation and reject with helpful message
    for input in inputs {
        if looks_like_citation(input) {
            bail!(
                "'{input}' looks like a citation pattern.\n\n\
                 For retrieving specific lines, use:\n  \
                 blz get {input}\n\n\
                 The `query` command is for text searches only."
            );
        }
    }

    // Join inputs into a single query
    let query = inputs.join(" ").trim().to_string();
    if query.is_empty() {
        bail!("Search query cannot be empty");
    }

    execute_internal(&query, sources, config, prefs, metrics, resource_monitor).await
}

/// Build search options from config structs.
fn build_search_options_from_config(
    query: &str,
    sources: &[String],
    config: &QueryExecutionConfig,
) -> SearchOptions {
    let (before_context, after_context, block) = config.content.resolve_context();
    let toggles = resolve_show_components(&config.display.show);

    SearchOptions {
        query: query.to_string(),
        sources: sources.to_vec(),
        last: config.search.last,
        limit: config.search.limit,
        page: config.search.page,
        top_percentile: config.search.top_percentile,
        format: config.display.format,
        show_url: toggles.url,
        show_lines: toggles.lines,
        show_anchor: toggles.anchor,
        show_raw_score: toggles.raw_score,
        no_summary: config.display.no_summary,
        score_precision: config.snippet.score_precision,
        snippet_lines: config.snippet.lines.max(1),
        all: config.search.limit >= ALL_RESULTS_LIMIT,
        no_history: config.search.no_history,
        copy: config.content.copy,
        before_context,
        after_context,
        block,
        max_block_lines: config.content.max_lines,
        max_chars: config.snippet.max_chars,
        quiet: config.display.quiet,
        headings_only: config.search.headings_only,
        timing: config.display.timing,
    }
}

/// Apply heading level filter to search results.
fn apply_heading_filter(results: &mut SearchResults, heading_filter: Option<&HeadingLevelFilter>) {
    if let Some(filter) = heading_filter {
        results.hits.retain(|hit| filter.matches(hit.level));
    }
}

/// Record search in preferences and history.
fn record_search_history(
    prefs: &mut CliPreferences,
    options: &SearchOptions,
    show: &[ShowComponent],
    page: usize,
    actual_limit: usize,
    total_pages: usize,
    total_results: usize,
) {
    use crate::utils::{history_log, preferences};
    use tracing::warn;

    let precision = options.score_precision.unwrap_or(DEFAULT_SCORE_PRECISION);
    let show_components = preferences::collect_show_components_extended(
        options.show_url,
        options.show_lines,
        options.show_anchor,
        options.show_raw_score,
    );
    prefs.set_default_show(&show_components);
    prefs.set_default_score_precision(precision);
    prefs.set_default_snippet_lines(options.snippet_lines);

    let history_source_str;
    let history_source = if options.sources.is_empty() {
        None
    } else if options.sources.len() == 1 {
        Some(options.sources[0].as_str())
    } else {
        history_source_str = options.sources.join(",");
        Some(history_source_str.as_str())
    };

    let history_entry =
        preferences::HistoryEntryBuilder::new(&options.query, history_source, options.format, show)
            .with_snippet_lines(options.snippet_lines)
            .with_score_precision(precision)
            .with_pagination(preferences::PaginationInfo {
                page: Some(page),
                limit: Some(actual_limit),
                total_pages: Some(total_pages),
                total_results: Some(total_results),
            })
            .with_headings_only(options.headings_only)
            .build();

    if !options.no_history {
        if let Err(err) = history_log::append(&history_entry) {
            warn!("failed to persist search history: {err}");
        }
    }
}

// -----------------------------------------------------------------------------
// Shape-Based Output Conversion
// -----------------------------------------------------------------------------

/// Convert a `SearchHit` to `SearchHitOutput` with percentage scoring.
fn convert_hit_to_output(
    hit: &SearchHit,
    max_score: f32,
    context_applied: usize,
) -> SearchHitOutput {
    // Calculate percentage score
    let percent = if max_score > 0.0 {
        f64::from(hit.score) / f64::from(max_score) * 100.0
    } else {
        0.0
    };
    let score_percentage = percent_to_u8(percent);

    // Convert context if present
    let context = hit.context.as_ref().map(|ctx| {
        let mut info = ContextInfo::new(context_applied, &ctx.lines)
            .with_line_numbers(ctx.line_numbers.clone());
        if !ctx.content.is_empty() {
            info = info.with_content(&ctx.content);
        }
        if let Some(truncated) = ctx.truncated {
            info = info.with_truncated(truncated);
        }
        info
    });

    SearchHitOutput {
        alias: hit.source.clone(),
        lines: hit.lines.clone(),
        snippet: hit.snippet.clone(),
        score: score_percentage,
        raw_score: Some(hit.score),
        heading_path: hit.heading_path.clone(),
        level: hit.level,
        anchor: hit.anchor.clone(),
        source_url: hit.source_url.clone(),
        fetched_at: hit.fetched_at,
        is_stale: hit.is_stale,
        checksum: hit.checksum.clone(),
        context,
    }
}

/// Build `SearchOutput` from `SearchResults` with pagination.
fn build_search_output(
    results: &SearchResults,
    options: &SearchOptions,
    page: usize,
    page_size: usize,
    total_pages: usize,
    page_hits: &[SearchHit],
) -> SearchOutput {
    // Get max score for percentage calculation
    let max_score = results.hits.first().map_or(0.0, |h| h.score);

    // Calculate context_applied from options (use max of before/after for asymmetric context)
    let context_applied = options.before_context.max(options.after_context);

    // Convert hits to output format
    let hit_outputs: Vec<SearchHitOutput> = page_hits
        .iter()
        .map(|hit| convert_hit_to_output(hit, max_score, context_applied))
        .collect();

    SearchOutput::builder(&options.query, hit_outputs)
        .total_results(results.hits.len())
        .total_lines_searched(results.total_lines_searched)
        .search_time(results.search_time)
        .sources(results.sources.clone())
        .page(page)
        .page_size(page_size)
        .total_pages(total_pages)
        .build()
}

/// Build `SearchRenderOptions` from `SearchOptions`.
fn build_render_options(
    options: &SearchOptions,
    page: usize,
    total_pages: usize,
    per_page: usize,
) -> SearchRenderOptions {
    SearchRenderOptions {
        query: options.query.clone(),
        show_url: options.show_url,
        show_lines: options.show_lines,
        show_anchor: options.show_anchor,
        show_raw_score: options.show_raw_score,
        no_summary: options.no_summary,
        score_precision: options.score_precision.unwrap_or(DEFAULT_SCORE_PRECISION),
        snippet_lines: usize::from(options.snippet_lines.max(1)),
        page,
        total_pages,
        per_page,
    }
}

/// Calculate pagination values from search results.
fn calculate_pagination(results: &SearchResults, options: &SearchOptions) -> (usize, usize, usize) {
    let total_results = results.hits.len();

    // Calculate effective limit
    let actual_limit = if options.limit >= ALL_RESULTS_LIMIT {
        results.hits.len().max(1)
    } else {
        options.limit.max(1)
    };

    // Calculate total pages
    let total_pages = if total_results == 0 {
        0
    } else {
        total_results.div_ceil(actual_limit)
    };

    // Determine page number
    let page = if options.last {
        total_pages.max(1)
    } else {
        options.page.clamp(1, total_pages.max(1))
    };

    (page, actual_limit, total_pages)
}

/// Render search results using shape-based output.
fn render_search_results(
    results: &SearchResults,
    options: &SearchOptions,
) -> Result<(usize, usize, usize, usize)> {
    let (page, actual_limit, total_pages) = calculate_pagination(results, options);
    let total_results = results.hits.len();

    // Handle empty results
    if total_results == 0 {
        let output = SearchOutput::builder(&options.query, vec![])
            .total_results(0)
            .total_lines_searched(results.total_lines_searched)
            .search_time(results.search_time)
            .sources(results.sources.clone())
            .page(0)
            .page_size(actual_limit)
            .total_pages(0)
            .build();

        let render_options = build_render_options(options, 0, 0, actual_limit);
        let mut stdout = io::stdout();
        render_search_with_options(&output, options.format, &render_options, &mut stdout)?;
        return Ok((0, actual_limit, total_pages, total_results));
    }

    // Calculate slice for current page
    let start_idx = (page - 1) * actual_limit;
    let end_idx = (start_idx + actual_limit).min(total_results);

    // Handle out-of-range page
    if start_idx >= total_results {
        if matches!(options.format, OutputFormat::Text) {
            eprintln!(
                "Page {} is beyond available results (Page {} of {})",
                options.page, page, total_pages
            );
            eprintln!("Tip: use --last to jump to the final page.");
        }
        // Show empty output for out-of-range
        let output = SearchOutput::builder(&options.query, vec![])
            .total_results(total_results)
            .total_lines_searched(results.total_lines_searched)
            .search_time(results.search_time)
            .sources(results.sources.clone())
            .page(page)
            .page_size(actual_limit)
            .total_pages(total_pages)
            .build();

        let render_options = build_render_options(options, page, total_pages, actual_limit);
        let mut stdout = io::stdout();
        render_search_with_options(&output, options.format, &render_options, &mut stdout)?;
        return Ok((page, actual_limit, total_pages, total_results));
    }

    // Normal case: render current page
    let page_hits = &results.hits[start_idx..end_idx];
    let output = build_search_output(results, options, page, actual_limit, total_pages, page_hits);

    let render_options = build_render_options(options, page, total_pages, actual_limit);
    let mut stdout = io::stdout();
    render_search_with_options(&output, options.format, &render_options, &mut stdout)?;

    Ok((page, actual_limit, total_pages, total_results))
}

/// Internal search execution (no citation check)
///
/// This is the core search logic that can be called by both `query` and the
/// deprecated `find` command when operating in search mode.
pub(super) async fn execute_internal(
    query: &str,
    sources: &[String],
    config: &QueryExecutionConfig,
    prefs: Option<&mut CliPreferences>,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    let options = build_search_options_from_config(query, sources, config);

    let mut results = perform_search(&options, metrics.clone()).await?;

    apply_heading_filter(&mut results, config.search.heading_filter.as_ref());

    // Use shape-based output rendering
    let (page, actual_limit, total_pages, total_results) =
        render_search_results(&results, &options)?;

    if options.copy && !results.hits.is_empty() {
        copy_results_to_clipboard(&results, page, actual_limit)?;
    }

    if let Some(prefs) = prefs {
        record_search_history(
            prefs,
            &options,
            &config.display.show,
            page,
            actual_limit,
            total_pages,
            total_results,
        );
    }

    if let Some(monitor) = resource_monitor {
        monitor.print_resource_usage();
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;

    #[test]
    fn test_citation_detection() {
        // Valid citations that should be rejected
        assert!(looks_like_citation("bun:120-142"));
        assert!(looks_like_citation("react:1-50"));
        assert!(looks_like_citation("vue-router:100-200"));
        assert!(looks_like_citation("bun:120-142,200-210"));

        // Invalid citations (should be treated as queries)
        assert!(!looks_like_citation("async patterns"));
        assert!(!looks_like_citation("useEffect cleanup"));
        assert!(!looks_like_citation("bun"));
        assert!(!looks_like_citation("bun:"));
        assert!(!looks_like_citation("bun:120"));
        assert!(!looks_like_citation("bun:120-"));
        assert!(!looks_like_citation("bun:-142"));
        assert!(!looks_like_citation(":120-142"));
        assert!(!looks_like_citation("Invalid:120-142")); // uppercase not allowed
        assert!(!looks_like_citation("bun:abc-def")); // non-numeric ranges
    }

    #[test]
    fn test_parse_heading_filter() {
        // Valid filters
        assert!(parse_heading_filter(Some("<=2")).is_ok());
        assert!(parse_heading_filter(Some("1,2,3")).is_ok());
        assert!(parse_heading_filter(Some("1-3")).is_ok());
        assert!(parse_heading_filter(None).unwrap().is_none());

        // Invalid filters
        assert!(parse_heading_filter(Some("invalid")).is_err());
    }

    #[test]
    fn test_build_search_options_from_config() {
        let search = SearchConfig::new()
            .with_limit(20)
            .with_page(2)
            .with_headings_only(true)
            .with_no_history(true);
        let display = DisplayConfig::new(OutputFormat::Json)
            .with_no_summary(true)
            .with_quiet(true);
        let snippet = SnippetConfig::new().with_lines(5).with_max_chars(300);
        let content = ContentConfig::new().with_context(Some(ContextMode::Symmetric(10)));

        let config = QueryExecutionConfig::new(search, display, snippet, content);

        let options = build_search_options_from_config("test query", &["bun".to_string()], &config);

        assert_eq!(options.query, "test query");
        assert_eq!(options.sources, vec!["bun".to_string()]);
        assert_eq!(options.limit, 20);
        assert_eq!(options.page, 2);
        assert!(options.headings_only);
        assert_eq!(options.format, OutputFormat::Json);
        assert!(options.no_summary);
        assert!(options.quiet);
        assert_eq!(options.snippet_lines, 5);
        assert_eq!(options.max_chars, 300);
        assert_eq!(options.before_context, 10);
        assert_eq!(options.after_context, 10);
        assert!(!options.block);
        assert!(options.no_history);
    }
}
