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

use anyhow::{Result, bail};
use clap::Args;

use crate::args::{ContextMode, ShowComponent};
use crate::output::OutputFormat;
use crate::utils::cli_args::FormatArg;
use crate::utils::preferences::CliPreferences;
use blz_core::{PerformanceMetrics, ResourceMonitor};

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
    copy_results_to_clipboard, default_search_limit, format_and_display, perform_search,
    resolve_show_components,
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

    execute(
        &args.inputs,
        &args.sources,
        args.limit,
        args.all,
        args.page,
        false, // last - query command doesn't support --last flag
        args.top,
        args.heading_level.clone(),
        resolved_format,
        &args.show,
        args.no_summary,
        args.score_precision,
        args.snippet_lines,
        args.max_chars,
        merged_context.as_ref(),
        args.block,
        args.max_lines,
        args.no_history,
        args.copy,
        quiet,
        args.headings_only,
        args.timing,
        Some(prefs),
        metrics,
        None,
    )
    .await
}

/// with a helpful error message suggesting `blz get` instead.
///
/// # Errors
///
/// Returns an error if:
/// - The input looks like a citation pattern (suggests using `blz get`)
/// - The query is empty
/// - The search fails
#[allow(clippy::too_many_arguments)]
#[allow(clippy::fn_params_excessive_bools)]
pub async fn execute(
    inputs: &[String],
    sources: &[String],
    limit: Option<usize>,
    all: bool,
    page: usize,
    last: bool,
    top: Option<u8>,
    heading_level: Option<String>,
    format: OutputFormat,
    show: &[ShowComponent],
    no_summary: bool,
    score_precision: Option<u8>,
    snippet_lines: u8,
    max_chars: Option<usize>,
    context_mode: Option<&ContextMode>,
    block: bool,
    max_lines: Option<usize>,
    no_history: bool,
    copy: bool,
    quiet: bool,
    headings_only: bool,
    timing: bool,
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

    execute_internal(
        &query,
        sources,
        limit,
        all,
        page,
        last,
        top,
        heading_level,
        format,
        show,
        no_summary,
        score_precision,
        snippet_lines,
        max_chars,
        context_mode,
        block,
        max_lines,
        no_history,
        copy,
        quiet,
        headings_only,
        timing,
        prefs,
        metrics,
        resource_monitor,
    )
    .await
}

/// Build search options from CLI parameters
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
fn build_search_options(
    query: &str,
    sources: &[String],
    limit: Option<usize>,
    all: bool,
    page: usize,
    last: bool,
    top: Option<u8>,
    format: OutputFormat,
    show: &[ShowComponent],
    no_summary: bool,
    score_precision: Option<u8>,
    snippet_lines: u8,
    max_chars: Option<usize>,
    context_mode: Option<&ContextMode>,
    block: bool,
    max_lines: Option<usize>,
    no_history: bool,
    copy: bool,
    quiet: bool,
    headings_only: bool,
    timing: bool,
) -> SearchOptions {
    // Calculate actual limit with proper default
    let limit = if all {
        ALL_RESULTS_LIMIT
    } else {
        limit.unwrap_or_else(default_search_limit)
    };

    // Clamp max_chars to valid range
    let max_chars = max_chars.map_or(200, clamp_max_chars);

    // Convert ContextMode to before/after context and block flag
    let (before_context, after_context, block) = match context_mode {
        Some(ContextMode::All) => (0, 0, true),
        Some(ContextMode::Symmetric(n)) => (*n, *n, false),
        Some(ContextMode::Asymmetric { before, after }) => (*before, *after, false),
        None => (0, 0, block),
    };

    let toggles = resolve_show_components(show);
    SearchOptions {
        query: query.to_string(),
        sources: sources.to_vec(),
        last,
        limit,
        page,
        top_percentile: top,
        format,
        show_url: toggles.url,
        show_lines: toggles.lines,
        show_anchor: toggles.anchor,
        show_raw_score: toggles.raw_score,
        no_summary,
        score_precision,
        snippet_lines: snippet_lines.max(1),
        all: limit >= ALL_RESULTS_LIMIT,
        no_history,
        copy,
        before_context,
        after_context,
        block,
        max_block_lines: max_lines,
        max_chars,
        quiet,
        headings_only,
        timing,
    }
}

/// Apply heading level filter to search results
fn apply_heading_filter(results: &mut SearchResults, heading_level: Option<String>) -> Result<()> {
    use crate::utils::heading_filter::HeadingLevelFilter;
    use anyhow::Context;

    if let Some(filter_str) = heading_level {
        let filter = filter_str
            .parse::<HeadingLevelFilter>()
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("Invalid heading level filter")?;

        results.hits.retain(|hit| filter.matches(hit.level));
    }
    Ok(())
}

/// Record search in preferences and history
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

/// Internal search execution (no citation check)
///
/// This is the core search logic that can be called by both `query` and the
/// deprecated `find` command when operating in search mode.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::fn_params_excessive_bools)]
pub(super) async fn execute_internal(
    query: &str,
    sources: &[String],
    limit: Option<usize>,
    all: bool,
    page: usize,
    last: bool,
    top: Option<u8>,
    heading_level: Option<String>,
    format: OutputFormat,
    show: &[ShowComponent],
    no_summary: bool,
    score_precision: Option<u8>,
    snippet_lines: u8,
    max_chars: Option<usize>,
    context_mode: Option<&ContextMode>,
    block: bool,
    max_lines: Option<usize>,
    no_history: bool,
    copy: bool,
    quiet: bool,
    headings_only: bool,
    timing: bool,
    prefs: Option<&mut CliPreferences>,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    let options = build_search_options(
        query,
        sources,
        limit,
        all,
        page,
        last,
        top,
        format,
        show,
        no_summary,
        score_precision,
        snippet_lines,
        max_chars,
        context_mode,
        block,
        max_lines,
        no_history,
        copy,
        quiet,
        headings_only,
        timing,
    );

    let mut results = perform_search(&options, metrics.clone()).await?;

    apply_heading_filter(&mut results, heading_level)?;

    let ((page, actual_limit, total_pages), total_results) =
        format_and_display(&results, &options)?;

    if options.copy && !results.hits.is_empty() {
        copy_results_to_clipboard(&results, page, actual_limit)?;
    }

    if let Some(prefs) = prefs {
        record_search_history(
            prefs,
            &options,
            show,
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
}
