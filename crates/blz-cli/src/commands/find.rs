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
    ALL_RESULTS_LIMIT, DEFAULT_SCORE_PRECISION, SearchOptions, SearchResults, clamp_max_chars,
    copy_results_to_clipboard, default_search_limit, format_and_display, perform_search,
    resolve_show_components,
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
    prefs: Option<&mut CliPreferences>,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    match classify_inputs(inputs)? {
        FindMode::Retrieve(specs) => {
            // Retrieve mode: delegate to get command logic
            // Note: heading_level is ignored in retrieve mode
            get::execute_internal(&specs, context_mode, block, max_lines, format, copy).await
        },
        FindMode::Search(query) => {
            // Search mode: delegate to search command logic
            execute_search_mode(
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
                prefs,
                metrics,
                resource_monitor,
            )
            .await
        },
    }
}

/// Compute effective limit based on --all flag and environment/default
fn compute_effective_limit(limit: Option<usize>, all: bool) -> usize {
    if all {
        ALL_RESULTS_LIMIT
    } else {
        limit.unwrap_or_else(default_search_limit)
    }
}

/// Extract context parameters from `ContextMode`
const fn extract_context_params(
    context_mode: Option<&ContextMode>,
    block: bool,
) -> (usize, usize, bool) {
    match context_mode {
        Some(ContextMode::All) => (0, 0, true),
        Some(ContextMode::Symmetric(n)) => (*n, *n, false),
        Some(ContextMode::Asymmetric { before, after }) => (*before, *after, false),
        None => (0, 0, block),
    }
}

/// Build `SearchOptions` from decomposed parameters
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
fn build_search_options(
    query: &str,
    sources: &[String],
    limit: usize,
    page: usize,
    last: bool,
    top: Option<u8>,
    format: OutputFormat,
    show: &[ShowComponent],
    no_summary: bool,
    score_precision: Option<u8>,
    snippet_lines: u8,
    max_chars: usize,
    before_context: usize,
    after_context: usize,
    block: bool,
    max_lines: Option<usize>,
    no_history: bool,
    copy: bool,
    quiet: bool,
    headings_only: bool,
) -> SearchOptions {
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

/// Update preferences and save search history
fn save_preferences_and_history(
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

/// Execute search mode (query detected)
#[allow(clippy::too_many_arguments)]
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
    let limit = compute_effective_limit(limit, all);
    let max_chars = max_chars.map_or(200, clamp_max_chars);
    let (before_context, after_context, block) = extract_context_params(context_mode, block);

    let options = build_search_options(
        query,
        sources,
        limit,
        page,
        last,
        top,
        format,
        show,
        no_summary,
        score_precision,
        snippet_lines,
        max_chars,
        before_context,
        after_context,
        block,
        max_lines,
        no_history,
        copy,
        quiet,
        headings_only,
    );

    let mut results = perform_search(&options, metrics.clone()).await?;
    apply_heading_filter(&mut results, heading_level)?;

    let ((page, actual_limit, total_pages), total_results) =
        format_and_display(&results, &options)?;

    if options.copy && !results.hits.is_empty() {
        copy_results_to_clipboard(&results, page, actual_limit)?;
    }

    if let Some(prefs) = prefs {
        save_preferences_and_history(
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
