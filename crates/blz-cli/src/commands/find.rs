//! Find command implementation - unified search and retrieve with pattern-based dispatch
//!
//! This module provides the `blz find` command which intelligently dispatches to either
//! search or get functionality based on the input pattern:
//!
//! - Citation pattern (`alias:digits-digits`) → retrieve mode (like `get`)
//! - Query pattern (anything else) → search mode (like `search`)

use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::args::{ContextMode, ShowComponent};
use crate::cli::{Commands, merge_context_flags};
use crate::commands::RequestSpec;
use crate::config::{
    ContentConfig, DisplayConfig, QueryExecutionConfig, SearchConfig, SnippetConfig,
};
use crate::utils::cli_args::{FormatArg, deprecation_warnings_suppressed};
use crate::utils::heading_filter::HeadingLevelFilter;
use crate::utils::preferences::CliPreferences;
use blz_core::{PerformanceMetrics, ResourceMonitor};

use super::get;
use super::query::execute_internal as query_execute_internal;
use super::search::{ALL_RESULTS_LIMIT, clamp_max_chars, default_search_limit};

/// Arguments for the deprecated `blz find` command.
///
/// This command is deprecated in favor of `blz query` for search and `blz get` for retrieval.
#[derive(Args, Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct FindArgs {
    /// Query terms or citation(s) (e.g., "query" or "alias:123-456")
    #[arg(value_name = "INPUT", required = true, num_args = 1..)]
    pub inputs: Vec<String>,

    /// Filter by source(s) for search mode - comma-separated or repeated (-s a -s b)
    #[arg(
        long = "source",
        short = 's',
        visible_alias = "alias",
        visible_alias = "sources",
        value_name = "SOURCE",
        value_delimiter = ','
    )]
    pub sources: Vec<String>,

    /// Maximum number of results per page (search mode only)
    #[arg(short = 'n', long, value_name = "COUNT", conflicts_with = "all")]
    pub limit: Option<usize>,

    /// Show all results - no limit (search mode only)
    #[arg(long, conflicts_with = "limit")]
    pub all: bool,

    /// Page number for pagination (search mode only)
    #[arg(long, default_value = "1")]
    pub page: usize,

    /// Show only top N percentile of results (1-100, search mode only)
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
    pub top: Option<u8>,

    /// Filter results by heading level (search mode only)
    ///
    /// Supports comparison operators (<=2, >2, >=3, <4, =2), lists (1,2,3), and ranges (1-3).
    ///
    /// Examples:
    ///   -H <=2       # Level 1 and 2 headings only
    ///   -H >2        # Level 3+ headings only
    ///   -H 1,2,3     # Levels 1, 2, and 3 only
    ///   -H 2-4       # Levels 2, 3, and 4 only
    #[arg(short = 'H', long = "heading-level", value_name = "FILTER")]
    pub heading_level: Option<String>,

    /// Output format (text, json, jsonl)
    #[command(flatten)]
    pub format: FormatArg,

    /// Additional columns to include in text output (search mode only)
    #[arg(long = "show", value_enum, value_delimiter = ',', env = "BLZ_SHOW")]
    pub show: Vec<ShowComponent>,

    /// Hide the summary/footer line (search mode only)
    #[arg(long = "no-summary")]
    pub no_summary: bool,

    /// Number of decimal places to show for scores (0-4, search mode only)
    #[arg(
        long = "score-precision",
        value_name = "PLACES",
        value_parser = clap::value_parser!(u8).range(0..=4),
        env = "BLZ_SCORE_PRECISION"
    )]
    pub score_precision: Option<u8>,

    /// Maximum snippet lines to display around a hit (1-10, search mode only)
    #[arg(
        long = "snippet-lines",
        value_name = "LINES",
        value_parser = clap::value_parser!(u8).range(1..=10),
        env = "BLZ_SNIPPET_LINES",
        default_value_t = 3,
        hide = true
    )]
    pub snippet_lines: u8,

    /// Maximum total characters in snippet (search mode, range: 50-1000, default: 200)
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

    /// Deprecated: use -C or --context instead
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

    /// Print LINES lines of context after each match
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

    /// Print LINES lines of context before each match
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

    /// Maximum number of lines to include when using block expansion
    #[arg(
        long = "max-lines",
        value_name = "LINES",
        value_parser = clap::value_parser!(usize),
        display_order = 34
    )]
    pub max_lines: Option<usize>,

    /// Restrict matches to heading text only (search mode only)
    #[arg(long = "headings-only", display_order = 35)]
    pub headings_only: bool,

    /// Don't save this search to history (search mode only)
    #[arg(long = "no-history")]
    pub no_history: bool,

    /// Copy results to clipboard using OSC 52 escape sequence
    #[arg(long)]
    pub copy: bool,

    /// Show detailed timing breakdown for performance analysis
    #[arg(long)]
    pub timing: bool,
}

/// Detect if input matches citation format: `alias:digits-digits[,digits-digits]*`
///
/// Examples:
/// - `bun:120-142` - single range
/// - `bun:120-142,200-210` - multiple ranges
/// - `react:1-50,100-150,200-250` - multiple ranges
fn is_citation(input: &str) -> bool {
    // Must contain a colon
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

    // Check each range (split by comma)
    for range in ranges.split(',') {
        // Each range must be in format "digits-digits"
        let Some((start, end)) = range.split_once('-') else {
            return false;
        };

        // Both start and end must be non-empty and all digits
        if start.is_empty()
            || end.is_empty()
            || !start.chars().all(|c| c.is_ascii_digit())
            || !end.chars().all(|c| c.is_ascii_digit())
        {
            return false;
        }
    }

    true
}

#[derive(Debug)]
enum FindMode {
    Retrieve(Vec<RequestSpec>),
    Search(String),
}

fn classify_inputs(inputs: &[String]) -> Result<FindMode> {
    if inputs.is_empty() {
        return Err(anyhow::anyhow!("find requires at least one input"));
    }

    let all_citations = inputs.iter().all(|input| is_citation(input));
    let any_citations = inputs.iter().any(|input| is_citation(input));

    if all_citations {
        let mut specs = Vec::with_capacity(inputs.len());
        for input in inputs {
            let (alias, line_expression) = input
                .split_once(':')
                .ok_or_else(|| anyhow::anyhow!("Invalid citation format: {input}"))?;
            specs.push(RequestSpec {
                alias: alias.to_string(),
                line_expression: line_expression.to_string(),
            });
        }
        return Ok(FindMode::Retrieve(specs));
    }

    if any_citations {
        return Err(anyhow::anyhow!(
            "Do not mix citations and search terms. Use `blz find \"query\"` or `blz find alias:1-2 alias:3-4`."
        ));
    }

    let query = inputs.join(" ").trim().to_string();
    if query.is_empty() {
        return Err(anyhow::anyhow!("Search query cannot be empty"));
    }
    Ok(FindMode::Search(query))
}

/// Execute the find command with smart pattern-based dispatch
///
/// # Pattern Detection
///
/// - If all inputs match `alias:digits-digits` → delegates to get (retrieve mode)
/// - Otherwise → delegates to search (search mode)
///
/// # Examples
///
/// ```ignore
/// // Search mode
/// blz find "async patterns"
/// blz find "useEffect cleanup" --source react
///
/// // Retrieve mode
/// blz find bun:120-142
/// blz find bun:120-142 deno:5-10
/// blz find bun:120-142,200-210 -C 5
/// ```
pub async fn execute(
    inputs: &[String],
    sources: &[String],
    config: &QueryExecutionConfig,
    prefs: Option<&mut CliPreferences>,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    match classify_inputs(inputs)? {
        FindMode::Retrieve(specs) => {
            // Retrieve mode: delegate to get command logic
            // Note: heading_filter is ignored in retrieve mode
            let (_, _, block) = config.content.resolve_context();
            get::execute_internal(
                &specs,
                config.content.context.as_ref(),
                block,
                config.content.max_lines,
                config.display.format,
                config.content.copy,
            )
            .await
        },
        FindMode::Search(query) => {
            // Search mode: delegate to query command's internal execution
            query_execute_internal(&query, sources, config, prefs, metrics, resource_monitor).await
        },
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

// ============================================================================
// Dispatch and Handler Functions (moved from lib.rs)
// ============================================================================

/// Dispatch a Find command variant, handling destructuring internally.
#[allow(clippy::too_many_lines, deprecated)]
pub async fn dispatch(
    cmd: Commands,
    quiet: bool,
    prefs: &mut CliPreferences,
    metrics: PerformanceMetrics,
) -> Result<()> {
    let Commands::Find(args) = cmd else {
        unreachable!("dispatch called with non-Find command");
    };

    if !deprecation_warnings_suppressed() {
        eprintln!(
            "{}",
            "Warning: 'find' is deprecated, use 'query' or 'get' instead".yellow()
        );
    }

    let resolved_format = args.format.resolve(quiet);
    let merged_context = merge_context_flags(
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
        .with_last(false) // find command doesn't support --last flag
        .with_no_history(args.no_history);

    let display = DisplayConfig::new(resolved_format)
        .with_show(args.show)
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn classify_inputs_retrieve_multiple() {
        let inputs = vec!["bun:1-2".to_string(), "deno:3-4".to_string()];
        let mode = classify_inputs(&inputs).expect("should classify as retrieve mode");
        assert!(matches!(mode, FindMode::Retrieve(specs) if specs.len() == 2));
    }

    #[test]
    fn classify_inputs_search_multiword() {
        let inputs = vec!["async".to_string(), "patterns".to_string()];
        let mode = classify_inputs(&inputs).expect("should classify as search mode");
        assert!(matches!(mode, FindMode::Search(ref query) if query == "async patterns"));
    }

    #[test]
    fn classify_inputs_rejects_mixed() {
        let inputs = vec!["bun:1-2".to_string(), "async".to_string()];
        let err = classify_inputs(&inputs).unwrap_err();
        assert!(
            err.to_string().contains("Do not mix citations"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn classify_inputs_rejects_empty() {
        let inputs: Vec<String> = vec![];
        let err = classify_inputs(&inputs).unwrap_err();
        assert!(
            err.to_string().contains("at least one input"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_citation_detection() {
        // Valid citations
        assert!(is_citation("bun:120-142"));
        assert!(is_citation("react:1-50"));
        assert!(is_citation("vue-router:100-200"));
        assert!(is_citation("bun:120-142,200-210"));
        assert!(is_citation("react:1-50,100-150,200-250"));

        // Invalid citations (should be treated as queries)
        assert!(!is_citation("async patterns"));
        assert!(!is_citation("useEffect cleanup"));
        assert!(!is_citation("bun"));
        assert!(!is_citation("bun:"));
        assert!(!is_citation("bun:120"));
        assert!(!is_citation("bun:120-"));
        assert!(!is_citation("bun:-142"));
        assert!(!is_citation(":120-142"));
        assert!(!is_citation("Invalid:120-142")); // uppercase not allowed
        assert!(!is_citation("bun:abc-def")); // non-numeric ranges
    }

    #[test]
    fn test_citation_patterns() {
        // Test valid patterns
        assert!(is_citation("test:1-10"));
        assert!(is_citation("my-source:100-200"));
        assert!(is_citation("source_name:50-75"));
        assert!(is_citation("a:1-2,3-4"));
        assert!(is_citation("source:10-20,30-40,50-60"));

        // Test invalid patterns
        assert!(!is_citation(""));
        assert!(!is_citation("test"));
        assert!(!is_citation("test:"));
        assert!(!is_citation("test:1"));
        assert!(!is_citation("test:1-"));
        assert!(!is_citation("test:-10"));
        assert!(!is_citation(":1-10"));
        assert!(!is_citation("TEST:1-10")); // uppercase
        assert!(!is_citation("test:1-10-20")); // invalid range format
    }

    #[test]
    fn test_citation_with_special_chars() {
        // Allowed special characters in alias
        assert!(is_citation("my-source:1-10"));
        assert!(is_citation("my_source:1-10"));
        assert!(is_citation("source-123:1-10"));
        assert!(is_citation("source_456:1-10"));

        // Disallowed characters
        assert!(!is_citation("my.source:1-10")); // dot not allowed
        assert!(!is_citation("my@source:1-10")); // @ not allowed
        assert!(!is_citation("my source:1-10")); // space not allowed
        assert!(!is_citation("my/source:1-10")); // slash not allowed
    }
}
