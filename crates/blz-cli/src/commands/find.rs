//! Find command implementation - unified search and retrieve with pattern-based dispatch
//!
//! This module provides the `blz find` command which intelligently dispatches to either
//! search or get functionality based on the input pattern:
//!
//! - Citation pattern (`alias:digits-digits`) → retrieve mode (like `get`)
//! - Query pattern (anything else) → search mode (like `search`)

use anyhow::Result;

use crate::cli::{ContextMode, ShowComponent};
use crate::commands::RequestSpec;
use crate::output::OutputFormat;
use crate::utils::preferences::CliPreferences;
use blz_core::{PerformanceMetrics, ResourceMonitor};

use super::get;
use super::search::{
    ALL_RESULTS_LIMIT, DEFAULT_SCORE_PRECISION, SearchOptions, clamp_max_chars,
    copy_results_to_clipboard, format_and_display, perform_search, resolve_show_components,
};

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

/// Execute the find command with smart pattern-based dispatch
///
/// # Pattern Detection
///
/// - If `input` matches `alias:digits-digits` → delegates to get (retrieve mode)
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
/// blz find bun:120-142,200-210 -C 5
/// ```
#[allow(clippy::too_many_arguments)]
#[allow(clippy::fn_params_excessive_bools)]
pub async fn execute(
    input: &str,
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
    prefs: Option<&mut CliPreferences>,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    if is_citation(input) {
        // Retrieve mode: delegate to get command logic
        // Note: heading_level is ignored in retrieve mode
        execute_retrieve_mode(input, context_mode, block, max_lines, format, copy).await
    } else {
        // Search mode: delegate to search command logic
        execute_search_mode(
            input,
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
            prefs,
            metrics,
            resource_monitor,
        )
        .await
    }
}

/// Execute retrieve mode (citation detected)
async fn execute_retrieve_mode(
    citation: &str,
    context_mode: Option<&ContextMode>,
    block: bool,
    max_lines: Option<usize>,
    format: OutputFormat,
    copy: bool,
) -> Result<()> {
    // Parse citation into RequestSpec
    // Format: alias:ranges (e.g., "bun:120-142,200-210")
    let (alias, line_expression) = citation
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("Invalid citation format: {citation}"))?;

    let specs = vec![RequestSpec {
        alias: alias.to_string(),
        line_expression: line_expression.to_string(),
    }];

    // Delegate to get::execute function
    get::execute(&specs, context_mode, block, max_lines, format, copy).await
}

/// Execute search mode (query detected)
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
#[allow(clippy::fn_params_excessive_bools)]
async fn execute_search_mode(
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
    prefs: Option<&mut CliPreferences>,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    use crate::utils::{heading_filter::HeadingLevelFilter, history_log, preferences};
    use anyhow::Context;
    use tracing::warn;

    // Calculate actual limit with proper default
    let limit = if all {
        ALL_RESULTS_LIMIT
    } else {
        limit.unwrap_or(50)
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
    let options = SearchOptions {
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
    };

    let mut results = perform_search(&options, metrics.clone()).await?;

    // Apply heading level filter if specified
    if let Some(filter_str) = heading_level {
        let filter = filter_str
            .parse::<HeadingLevelFilter>()
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("Invalid heading level filter")?;

        // Filter the hits based on the heading level
        results.hits.retain(|hit| filter.matches(hit.level));
    }

    let ((page, actual_limit, total_pages), total_results) =
        format_and_display(&results, &options)?;

    // Copy results to clipboard if --copy flag was set
    if options.copy && !results.hits.is_empty() {
        copy_results_to_clipboard(&results, page, actual_limit)?;
    }

    if let Some(prefs) = prefs {
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
            // For multiple sources, store as comma-separated list
            history_source_str = options.sources.join(",");
            Some(history_source_str.as_str())
        };
        let history_entry = preferences::HistoryEntryBuilder::new(
            &options.query,
            history_source,
            options.format,
            show,
        )
        .with_snippet_lines(options.snippet_lines)
        .with_score_precision(precision)
        .with_pagination(preferences::PaginationInfo {
            page: Some(page),
            limit: Some(actual_limit),
            total_pages: Some(total_pages),
            total_results: Some(total_results),
        })
        .build();
        if !options.no_history {
            if let Err(err) = history_log::append(&history_entry) {
                warn!("failed to persist search history: {err}");
            }
        }
    }

    if let Some(monitor) = resource_monitor {
        monitor.print_resource_usage();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
