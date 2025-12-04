//! Find tool implementation for searching and retrieving documentation snippets

use blz_core::{SearchIndex, Storage, index::DEFAULT_SNIPPET_CHAR_LIMIT};
use serde::{Deserialize, Serialize};

use crate::{
    cache,
    error::McpResult,
    types::{IndexCache, ResponseFormat},
};

/// Default maximum number of search results
const DEFAULT_MAX_RESULTS: usize = 10;
/// Maximum line padding allowed
const MAX_LINE_PADDING: u32 = 50;
/// Maximum allowed search results
const MAX_ALLOWED_RESULTS: usize = 1000;
/// Maximum number of characters to include in search snippets when using concise format
const CONCISE_SEARCH_SNIPPET_CHARS: usize = 160;
/// Maximum number of characters to include in snippet content when using concise format
const CONCISE_SNIPPET_CONTENT_CHARS: usize = 800;

/// Source filter for search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SourceFilter {
    /// Search multiple specific sources
    Multiple(Vec<String>),
    /// Search a single source (including "all")
    Single(String),
}

impl SourceFilter {
    /// Check if this filter represents "all sources"
    #[cfg(test)]
    fn is_all(&self) -> bool {
        matches!(self, Self::Single(s) if s == "all")
    }

    /// Get the list of specific sources to search, or None for "all"
    fn sources(&self) -> Option<Vec<String>> {
        match self {
            Self::Single(s) if s == "all" => None,
            Self::Single(s) => Some(vec![s.clone()]),
            Self::Multiple(sources) => Some(sources.clone()),
        }
    }
}

/// Parameters for the find tool
///
/// # Performance Notes
/// - Cross-source queries scale linearly with the number of sources searched
/// - Results are merged and re-ranked globally by relevance across sources
/// - Failed sources emit warnings and are skipped to keep responses resilient
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindParams {
    /// Search query (optional if only retrieving snippets)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// Citation strings for snippet retrieval (e.g., "bun:10-20,30-40")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippets: Option<Vec<String>>,

    /// Context mode: "none", "symmetric", or "all"
    #[serde(default = "default_context_mode")]
    pub context_mode: String,

    /// Lines of padding (0-50)
    #[serde(default)]
    pub line_padding: u32,

    /// Maximum search results (default 10)
    #[serde(default = "default_max_results")]
    pub max_results: usize,

    /// Optional source filter - can be:
    /// - None: search all sources
    /// - "all": search all sources explicitly
    /// - Single string: search one source
    /// - Array of strings: search multiple specific sources
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceFilter>,

    /// Response format: "concise" (default) or "detailed"
    ///
    /// Concise returns minimal data, detailed includes all metadata.
    /// Based on Anthropic research showing 30-65% token savings with concise mode.
    #[serde(default)]
    pub format: ResponseFormat,

    /// Restrict matches to heading text only
    #[serde(default)]
    pub headings_only: bool,
}

fn default_context_mode() -> String {
    "none".to_string()
}

const fn default_max_results() -> usize {
    DEFAULT_MAX_RESULTS
}

/// Output from find tool
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindOutput {
    /// Search results (if query provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_results: Option<Vec<SearchHitResult>>,

    /// Snippet results (if snippets requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet_results: Option<Vec<SnippetResult>>,

    /// Execution metadata
    pub executed: FindExecuted,
}

/// Individual search hit
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHitResult {
    /// Source identifier where this hit was found
    pub source: String,
    /// Line range in format "start-end"
    pub lines: String,
    /// BM25 relevance score
    pub score: f32,
    /// Text snippet preview
    pub snippet: String,
    /// Hierarchical heading path (e.g., "Section > Subsection")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_path: Option<String>,
}

/// Individual snippet result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetResult {
    /// Source identifier
    pub source: String,
    /// Retrieved content
    pub content: String,
    /// Starting line number (1-based)
    pub line_start: usize,
    /// Ending line number (1-based, inclusive)
    pub line_end: usize,
}

/// Execution metadata
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindExecuted {
    /// Whether search was executed
    pub search_executed: bool,
    /// Whether snippet retrieval was executed
    pub snippets_executed: bool,
}

/// Truncate a string to the specified number of characters, appending ellipsis when shortened.
fn truncate_with_ellipsis(text: &mut String, max_chars: usize) {
    if text.chars().count() <= max_chars {
        return;
    }

    let mut truncated = String::with_capacity(max_chars + 3);
    for (idx, ch) in text.chars().enumerate() {
        if idx >= max_chars {
            truncated.push_str("...");
            *text = truncated;
            return;
        }
        truncated.push(ch);
    }
}

/// Apply the concise response format by trimming verbose fields.
fn apply_concise_format(
    search_results: &mut Option<Vec<SearchHitResult>>,
    snippet_results: &mut Option<Vec<SnippetResult>>,
) {
    if let Some(hits) = search_results {
        for hit in hits {
            hit.heading_path = None;
            truncate_with_ellipsis(&mut hit.snippet, CONCISE_SEARCH_SNIPPET_CHARS);
        }
    }

    if let Some(snippets) = snippet_results {
        for snippet in snippets {
            truncate_with_ellipsis(&mut snippet.content, CONCISE_SNIPPET_CONTENT_CHARS);
        }
    }
}

/// Parse citation string into source and line ranges
///
/// Format: "source:range1,range2" where range is "start-end"
/// Examples: "bun:10-20", "react:10-20,30-40"
#[tracing::instrument]
fn parse_citation(citation: &str) -> Result<(String, Vec<(usize, usize)>), String> {
    let parts: Vec<&str> = citation.splitn(2, ':').collect();

    if parts.len() != 2 {
        return Err(format!(
            "Invalid citation format: {citation}. Expected 'source:lines'"
        ));
    }

    let source = parts[0].trim();
    if source.is_empty() {
        return Err("Source cannot be empty".to_string());
    }

    let source = source.to_string();
    let ranges_text = parts[1];

    let mut ranges = Vec::new();

    for range_part in ranges_text.split(',') {
        let range_str = range_part.trim();
        let range_parts: Vec<&str> = range_str.split('-').collect();

        if range_parts.len() != 2 {
            return Err(format!(
                "Invalid range format: {range_str}. Expected 'start-end'"
            ));
        }

        let start_str = range_parts[0].trim();
        let end_str = range_parts[1].trim();

        let start = start_str
            .parse::<usize>()
            .map_err(|_| format!("Invalid line number: {start_str}"))?;

        let end = end_str
            .parse::<usize>()
            .map_err(|_| format!("Invalid line number: {end_str}"))?;

        if start == 0 || end == 0 {
            return Err("Line numbers must be >= 1".to_string());
        }

        if start > end {
            return Err(format!("Invalid range {start}-{end}: start must be <= end"));
        }

        ranges.push((start, end));
    }

    if ranges.is_empty() {
        return Err("No valid ranges found in citation".to_string());
    }

    Ok((source, ranges))
}

/// Execute search query
#[tracing::instrument(skip(index))]
async fn execute_search(
    index: &SearchIndex,
    query: &str,
    source_filter: Option<&str>,
    max_results: usize,
    headings_only: bool,
) -> McpResult<Vec<SearchHitResult>> {
    let hits = if headings_only {
        index.search_headings_only(
            query,
            source_filter,
            max_results,
            DEFAULT_SNIPPET_CHAR_LIMIT,
        )?
    } else {
        index.search(query, source_filter, max_results)?
    };

    let results = hits
        .into_iter()
        .map(|hit| SearchHitResult {
            source: hit.source,
            lines: hit.lines,
            score: hit.score,
            snippet: hit.snippet,
            heading_path: if hit.heading_path.is_empty() {
                None
            } else {
                Some(hit.heading_path.join(" > "))
            },
        })
        .collect();

    Ok(results)
}

/// Find section boundary for context "all" mode based on heading hierarchy.
///
/// Locates the TOC entry containing the requested lines and expands to the
/// logical section boundary by finding the next heading of equal or higher level.
///
/// ## Heading Hierarchy
///
/// The heading level is determined by `heading_path.len()`:
/// - H1: `heading_path.len() == 1` (e.g., `["Introduction"]`)
/// - H2: `heading_path.len() == 2` (e.g., `["Introduction", "Setup"]`)
/// - H3: `heading_path.len() == 3` (e.g., `["Introduction", "Setup", "Prerequisites"]`)
///
/// Expansion stops at the next heading with `heading_path.len() <= current_level`.
///
/// ## Arguments
///
/// - `toc`: TOC entries to search (may be nested)
/// - `start_line`: Starting line (1-based)
/// - `end_line`: Ending line (1-based, inclusive)
///
/// ## Returns
///
/// `Some((section_start, section_end))` with 0-based indices if a section
/// boundary is found, `None` otherwise (falls back to symmetric padding).
#[tracing::instrument]
fn find_containing_block(
    toc: &[blz_core::TocEntry],
    start_line: usize,
    end_line: usize,
) -> Option<(usize, usize)> {
    // Flatten the TOC tree into a linear list of entries with their boundaries
    let mut flat_toc: Vec<(&blz_core::TocEntry, usize, usize)> = Vec::new();
    flatten_toc(toc, &mut flat_toc);

    // Find the entry containing BOTH start_line and end_line with the deepest
    // nesting level (longest heading_path) to get the most specific section.
    // If no section contains the entire range, the function returns None so the caller
    // can fall back to symmetric padding.
    let containing_entry = flat_toc
        .iter()
        .filter(|(_, block_start, block_end)| {
            *block_start <= start_line
                && start_line <= *block_end
                && *block_start <= end_line
                && end_line <= *block_end
        })
        .max_by_key(|(entry, _, _)| entry.heading_path.len())?;

    let (current_entry, section_start, _) = *containing_entry;
    let current_level = current_entry.heading_path.len();

    tracing::debug!(
        heading = %current_entry.heading_path.join(" > "),
        level = current_level,
        section_lines = %current_entry.lines,
        "Found containing section"
    );

    // Find the next heading of same or higher level (equal or shorter heading_path)
    let section_end = flat_toc
        .iter()
        .skip_while(|(entry, _, _)| !std::ptr::eq(*entry, current_entry))
        .skip(1) // Skip the current entry itself
        .find(|(entry, _, _)| entry.heading_path.len() <= current_level)
        .map_or_else(
            || {
                // No next section found - this is the last section, use its end boundary
                flat_toc.last().map_or(section_start, |(_, _, end)| *end)
            },
            |(_, next_start, _)| next_start.saturating_sub(1), // End just before next section
        );

    tracing::debug!(section_start, section_end, "Expanded to section boundary");

    // Convert to 0-based indexing
    Some((
        section_start.saturating_sub(1),
        section_end.saturating_sub(1),
    ))
}

/// Recursively flatten TOC tree into a linear list of (entry, start, end) tuples.
///
/// This helper function traverses the TOC tree in depth-first order and extracts
/// the line boundaries for each entry.
fn flatten_toc<'a>(
    toc: &'a [blz_core::TocEntry],
    result: &mut Vec<(&'a blz_core::TocEntry, usize, usize)>,
) {
    for entry in toc {
        // Parse the "start-end" format from TOC entry
        let parts: Vec<&str> = entry.lines.split('-').collect();
        if parts.len() != 2 {
            tracing::warn!(
                lines = %entry.lines,
                "Invalid TOC line range format, skipping entry"
            );
            continue;
        }

        let Ok(block_start) = parts[0].parse::<usize>() else {
            tracing::warn!(
                value = %parts[0],
                "Failed to parse TOC start line, skipping entry"
            );
            continue;
        };

        let Ok(block_end) = parts[1].parse::<usize>() else {
            tracing::warn!(
                value = %parts[1],
                "Failed to parse TOC end line, skipping entry"
            );
            continue;
        };

        result.push((entry, block_start, block_end));

        // Recursively process children
        if !entry.children.is_empty() {
            flatten_toc(&entry.children, result);
        }
    }
}

/// Retrieve snippet with context
#[tracing::instrument(skip(storage))]
fn retrieve_snippet(
    storage: &Storage,
    source: &str,
    start: usize,
    end: usize,
    context_mode: &str,
    line_padding: u32,
) -> McpResult<SnippetResult> {
    let content = storage.load_llms_txt(source)?;
    let lines: Vec<&str> = content.lines().collect();

    // Check for empty document
    if lines.is_empty() {
        return Err(crate::error::McpError::Internal(format!(
            "Source '{source}' has no content"
        )));
    }

    // Convert to 0-based indexing
    let start_idx = start.saturating_sub(1);
    let end_idx = end.saturating_sub(1);

    // Validate line ranges
    if start_idx >= lines.len() || end_idx >= lines.len() {
        return Err(crate::error::McpError::Internal(format!(
            "Line range {start}-{end} exceeds document length {} for source '{source}'",
            lines.len()
        )));
    }

    let (actual_start, actual_end) = match context_mode {
        "all" => {
            // Try to load TOC and find containing block
            match storage.load_llms_json(source) {
                Ok(llms_json) => {
                    // Search for containing block using 1-based line numbers
                    if let Some((block_start, block_end)) =
                        find_containing_block(&llms_json.toc, start, end)
                    {
                        tracing::debug!(
                            requested = %format!("{start}-{end}"),
                            block = %format!("{}-{}", block_start + 1, block_end + 1),
                            "Found containing block from TOC"
                        );

                        let max_idx = lines.len().saturating_sub(1);
                        let clamped_start = block_start.min(max_idx);
                        let clamped_end = block_end.min(max_idx);

                        if clamped_start != block_start || clamped_end != block_end {
                            tracing::warn!(
                                requested = %format!("{start}-{end}"),
                                toc_block = %format!("{}-{}", block_start + 1, block_end + 1),
                                clamped_block = %format!(
                                    "{}-{}",
                                    clamped_start + 1,
                                    clamped_end + 1
                                ),
                                total_lines = lines.len(),
                                "TOC block exceeded document bounds; clamped to file length"
                            );
                        }

                        (clamped_start, clamped_end)
                    } else {
                        // No containing block found, fall back to symmetric padding
                        tracing::warn!(
                            source,
                            "No TOC entry contains requested range, using symmetric fallback"
                        );
                        let padding = 20_usize;
                        let range_start = start_idx.saturating_sub(padding);
                        let range_end = (end_idx + padding).min(lines.len().saturating_sub(1));
                        (range_start, range_end)
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        source,
                        error = %e,
                        "Failed to load llms.json, using symmetric fallback"
                    );
                    let padding = 20_usize;
                    let range_start = start_idx.saturating_sub(padding);
                    let range_end = (end_idx + padding).min(lines.len().saturating_sub(1));
                    (range_start, range_end)
                },
            }
        },
        "symmetric" => {
            let padding = line_padding as usize;
            let range_start = start_idx.saturating_sub(padding);
            let range_end = (end_idx + padding).min(lines.len().saturating_sub(1));
            (range_start, range_end)
        },
        _ => {
            // "none" - just the requested range
            (start_idx, end_idx.min(lines.len().saturating_sub(1)))
        },
    };

    let snippet_lines = &lines[actual_start..=actual_end];
    let snippet_content = snippet_lines.join("\n");

    Ok(SnippetResult {
        source: source.to_string(),
        content: snippet_content,
        line_start: actual_start + 1, // Convert back to 1-based
        line_end: actual_end + 1,     // Convert back to 1-based
    })
}

/// Main handler for find tool
#[tracing::instrument(skip(storage, index_cache))]
#[allow(clippy::too_many_lines)] // Complex search logic with validation, caching, and multi-source merging
pub async fn handle_find(
    params: FindParams,
    storage: &Storage,
    index_cache: &IndexCache,
) -> McpResult<FindOutput> {
    // Validate that at least one parameter is provided
    if params.query.is_none() && params.snippets.is_none() {
        return Err(crate::error::McpError::Internal(
            "Either query or snippets must be provided".to_string(),
        ));
    }

    // Validate parameters
    if params.line_padding > MAX_LINE_PADDING {
        return Err(crate::error::McpError::InvalidPadding(params.line_padding));
    }

    if params.max_results > MAX_ALLOWED_RESULTS {
        return Err(crate::error::McpError::Internal(format!(
            "max_results {} exceeds limit of {}",
            params.max_results, MAX_ALLOWED_RESULTS
        )));
    }

    let valid_context_modes = ["none", "symmetric", "all"];
    if !valid_context_modes.contains(&params.context_mode.as_str()) {
        return Err(crate::error::McpError::Internal(format!(
            "Invalid context mode: {}. Must be one of: {:?}",
            params.context_mode, valid_context_modes
        )));
    }

    let mut search_results = None;
    let mut snippet_results = None;

    // Execute search if query provided
    if let Some(ref query) = params.query {
        // Validate query is not empty
        if query.trim().is_empty() {
            return Err(crate::error::McpError::Internal(
                "Query cannot be empty".to_string(),
            ));
        }
        tracing::debug!(query, source = ?params.source, "executing search");

        // Determine which sources to search
        let mut sources_to_search = params
            .source
            .as_ref()
            .and_then(SourceFilter::sources)
            .unwrap_or_else(|| storage.list_sources());

        // Validate we have sources to search
        // If no sources are available from storage but we're searching all,
        // fall back to sources in the cache
        if sources_to_search.is_empty() {
            if params.source.is_none() {
                let cache_read = index_cache.read().await;
                let cached_sources: Vec<String> = cache_read.keys().cloned().collect();
                drop(cache_read);

                if cached_sources.is_empty() {
                    let available = storage.list_sources();
                    return Err(crate::error::McpError::Internal(format!(
                        "No sources available to search. Available sources: {available:?}"
                    )));
                }

                sources_to_search = cached_sources;
            } else {
                let available = storage.list_sources();
                return Err(crate::error::McpError::Internal(format!(
                    "No sources available to search for filter {:?}. Available sources: {available:?}",
                    params.source.as_ref()
                )));
            }
        }

        tracing::debug!(
            sources = ?sources_to_search,
            count = sources_to_search.len(),
            "searching sources"
        );

        // Search across all specified sources and merge results
        let source_count = sources_to_search.len().max(1);
        let estimated_capacity = params
            .max_results
            .saturating_mul(source_count)
            .min(MAX_ALLOWED_RESULTS);
        let mut all_hits = Vec::with_capacity(estimated_capacity.max(params.max_results));

        for source in &sources_to_search {
            // Get or load the index for this source
            let index = match cache::get_or_load_index(index_cache, storage, source).await {
                Ok(idx) => idx,
                Err(e) => {
                    tracing::warn!(
                        source,
                        error = %e,
                        "failed to load index, skipping source"
                    );
                    continue;
                },
            };

            // Search this index (it already filters by alias internally)
            match execute_search(
                &index,
                query,
                Some(source),
                params.max_results,
                params.headings_only,
            )
            .await
            {
                Ok(hits) => {
                    all_hits.extend(hits);
                },
                Err(e) => {
                    tracing::warn!(
                        source,
                        error = %e,
                        "search failed for source, skipping"
                    );
                },
            }
        }

        // Sort merged results by score (descending) and limit
        all_hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_hits.truncate(params.max_results);

        tracing::debug!(
            count = all_hits.len(),
            sources_searched = sources_to_search.len(),
            "search completed"
        );
        search_results = Some(all_hits);
    }

    // Retrieve snippets if requested
    if let Some(ref citations) = params.snippets {
        tracing::debug!(count = citations.len(), "retrieving snippets");

        let mut results = Vec::with_capacity(citations.len());

        for citation in citations {
            let (source, ranges) =
                parse_citation(citation).map_err(crate::error::McpError::InvalidCitation)?;

            for (start, end) in ranges {
                let snippet = retrieve_snippet(
                    storage,
                    &source,
                    start,
                    end,
                    &params.context_mode,
                    params.line_padding,
                )?;
                results.push(snippet);
            }
        }

        tracing::debug!(count = results.len(), "snippets retrieved");
        snippet_results = Some(results);
    }

    if matches!(params.format, ResponseFormat::Concise) {
        apply_concise_format(&mut search_results, &mut snippet_results);
    }

    Ok(FindOutput {
        search_results,
        snippet_results,
        executed: FindExecuted {
            search_executed: params.query.is_some(),
            snippets_executed: params.snippets.is_some(),
        },
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::assertions_on_constants)] // Test assertions are intentional
    use super::*;

    #[test]
    fn test_source_filter_deserialization_single_string() {
        let params: FindParams = serde_json::from_value(serde_json::json!({
            "source": "bun",
            "query": "test"
        }))
        .expect("Should deserialize single source");

        assert!(params.source.is_some());
        if let Some(SourceFilter::Single(s)) = params.source {
            assert_eq!(s, "bun");
        } else {
            assert!(false, "Expected Single variant");
        }
    }

    #[test]
    fn test_source_filter_deserialization_all_string() {
        let params: FindParams = serde_json::from_value(serde_json::json!({
            "source": "all",
            "query": "test"
        }))
        .expect("Should deserialize 'all' source");

        assert!(params.source.is_some());
        if let Some(ref filter) = params.source {
            assert!(filter.is_all());
        } else {
            assert!(false, "Expected source to be present");
        }
    }

    #[test]
    fn test_source_filter_deserialization_array() {
        let params: FindParams = serde_json::from_value(serde_json::json!({
            "source": ["bun", "turbo", "react"],
            "query": "test"
        }))
        .expect("Should deserialize array of sources");

        assert!(params.source.is_some());
        if let Some(SourceFilter::Multiple(sources)) = params.source {
            assert_eq!(sources, vec!["bun", "turbo", "react"]);
        } else {
            assert!(false, "Expected Multiple variant");
        }
    }

    #[test]
    fn test_source_filter_deserialization_none() {
        let params: FindParams = serde_json::from_value(serde_json::json!({
            "query": "test"
        }))
        .expect("Should deserialize without source");

        assert!(params.source.is_none());
    }

    #[test]
    fn test_parse_citation_single_range() {
        let result = parse_citation("bun:10-20");
        assert!(result.is_ok());
        if let Ok((source, ranges)) = result {
            assert_eq!(source, "bun");
            assert_eq!(ranges, vec![(10, 20)]);
        }
    }

    #[test]
    fn test_parse_citation_multiple_ranges() {
        let result = parse_citation("react:10-20,30-40");
        assert!(result.is_ok());
        if let Ok((source, ranges)) = result {
            assert_eq!(source, "react");
            assert_eq!(ranges, vec![(10, 20), (30, 40)]);
        }
    }

    #[test]
    fn test_parse_citation_invalid_format() {
        assert!(parse_citation("invalid").is_err());
        assert!(parse_citation("bun:").is_err());
        assert!(parse_citation(":10-20").is_err());
    }

    #[test]
    fn test_parse_citation_invalid_range() {
        assert!(parse_citation("bun:20-10").is_err()); // start > end
        assert!(parse_citation("bun:0-10").is_err()); // zero line number
        assert!(parse_citation("bun:abc-10").is_err()); // non-numeric
    }

    #[test]
    fn test_parse_citation_whitespace_after_colon() {
        let result = parse_citation("bun: 10-20");
        assert!(result.is_ok());
        if let Ok((source, ranges)) = result {
            assert_eq!(source, "bun");
            assert_eq!(ranges, vec![(10, 20)]);
        }
    }

    #[test]
    fn test_parse_citation_whitespace_around_dash() {
        let result = parse_citation("bun:10 - 20");
        assert!(result.is_ok());
        if let Ok((source, ranges)) = result {
            assert_eq!(source, "bun");
            assert_eq!(ranges, vec![(10, 20)]);
        }
    }

    #[test]
    fn test_parse_citation_whitespace_after_comma() {
        let result = parse_citation("bun:10-20, 30-40");
        assert!(result.is_ok());
        if let Ok((source, ranges)) = result {
            assert_eq!(source, "bun");
            assert_eq!(ranges, vec![(10, 20), (30, 40)]);
        }
    }

    #[test]
    fn test_parse_citation_multiple_spaces() {
        let result = parse_citation("bun:  10  -  20  ,  30  -  40");
        assert!(result.is_ok());
        if let Ok((source, ranges)) = result {
            assert_eq!(source, "bun");
            assert_eq!(ranges, vec![(10, 20), (30, 40)]);
        }
    }

    #[test]
    fn test_parse_citation_leading_trailing_whitespace() {
        let result = parse_citation("  bun:10-20  ");
        assert!(result.is_ok());
        if let Ok((source, ranges)) = result {
            assert_eq!(source, "bun");
            assert_eq!(ranges, vec![(10, 20)]);
        }
    }

    #[test]
    fn test_parse_citation_all_whitespace_variations() {
        // Combination of all whitespace patterns
        let result = parse_citation("  react: 10 - 20 , 30 - 40  ");
        assert!(result.is_ok());
        if let Ok((source, ranges)) = result {
            assert_eq!(source, "react");
            assert_eq!(ranges, vec![(10, 20), (30, 40)]);
        }
    }

    #[test]
    fn test_parse_citation_existing_valid_formats_still_work() {
        // Ensure backward compatibility with existing valid formats
        let test_cases = vec![
            ("bun:10-20", vec![(10, 20)]),
            ("react:10-20,30-40", vec![(10, 20), (30, 40)]),
            ("vue:1-5,10-15,20-25", vec![(1, 5), (10, 15), (20, 25)]),
        ];

        for (input, expected_ranges) in test_cases {
            let result = parse_citation(input);
            assert!(result.is_ok(), "Failed to parse: {input}");
            if let Ok((_, ranges)) = result {
                assert_eq!(ranges, expected_ranges);
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::needless_raw_string_hashes)]

    use super::*;
    use crate::{error::McpError, types::IndexCache};
    use blz_core::{SearchIndex, Storage};
    use std::fmt::Write as _;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::RwLock;

    /// Create a test storage with a sample document
    fn setup_test_storage() -> (Storage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let storage =
            Storage::with_root(temp_dir.path().to_path_buf()).expect("Failed to create storage");

        // Create a sample llms.txt file with known content
        let test_content = r#"# Test Documentation
This is line 2
This is line 3
This is line 4
This is line 5

## Section 1
Content for section 1
More content here
Last line of section 1

## Section 2
Content for section 2
More content here
Last line of section 2"#;

        std::fs::create_dir_all(temp_dir.path().join("sources/test-source"))
            .expect("Failed to create sources dir");
        std::fs::write(
            temp_dir.path().join("sources/test-source/llms.txt"),
            test_content,
        )
        .expect("Failed to write test content");

        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_query_only_execution() {
        let (storage, temp_dir) = setup_test_storage();
        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));

        // Create and index the test source
        let index_path = temp_dir.path().join("sources/test-source/.index");
        let index = SearchIndex::create(&index_path).expect("Failed to create index");

        let params = FindParams {
            query: Some("section".to_string()),
            snippets: None,
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 10,
            source: Some(SourceFilter::Single("test-source".to_string())),
            format: ResponseFormat::default(),
            headings_only: false,
        };

        // Store index in cache
        {
            let mut cache = index_cache.write().await;
            cache.insert("test-source".to_string(), Arc::new(index));
        }

        let result = handle_find(params, &storage, &index_cache).await;
        if let Err(ref e) = result {
            eprintln!("Error in test_query_only_execution: {e:?}");
        }
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.executed.search_executed);
        assert!(!output.executed.snippets_executed);
        assert!(output.search_results.is_some());
        assert!(output.snippet_results.is_none());
    }

    #[tokio::test]
    async fn test_snippets_only_execution() {
        let (storage, _temp_dir) = setup_test_storage();
        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));

        let params = FindParams {
            query: None,
            snippets: Some(vec!["test-source:2-4".to_string()]),
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 10,
            source: None,
            format: ResponseFormat::default(),
            headings_only: false,
        };

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.executed.search_executed);
        assert!(output.executed.snippets_executed);
        assert!(output.search_results.is_none());
        assert!(output.snippet_results.is_some());

        let snippets = output.snippet_results.unwrap();
        assert_eq!(snippets.len(), 1);
        assert_eq!(snippets[0].line_start, 2);
        assert_eq!(snippets[0].line_end, 4);
        assert!(snippets[0].content.contains("This is line 2"));
    }

    #[tokio::test]
    async fn test_combined_query_and_snippets() {
        let (storage, temp_dir) = setup_test_storage();
        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));

        // Create and index the test source
        let index_path = temp_dir.path().join("sources/test-source/.index");
        let index = SearchIndex::create(&index_path).expect("Failed to create index");

        let params = FindParams {
            query: Some("section".to_string()),
            snippets: Some(vec!["test-source:2-4".to_string()]),
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 10,
            source: Some(SourceFilter::Single("test-source".to_string())),
            format: ResponseFormat::default(),
            headings_only: false,
        };

        // Store index in cache
        {
            let mut cache = index_cache.write().await;
            cache.insert("test-source".to_string(), Arc::new(index));
        }

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.executed.search_executed);
        assert!(output.executed.snippets_executed);
        assert!(output.search_results.is_some());
        assert!(output.snippet_results.is_some());
    }

    #[tokio::test]
    async fn test_concise_format_truncates_snippet_content() {
        let (storage, temp_dir) = setup_test_storage();
        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let alias = "concise-source";

        // Create a long document to guarantee truncation
        let mut long_content = String::new();
        for i in 1..=200 {
            writeln!(
                &mut long_content,
                "Line {i}: Lorem ipsum dolor sit amet, consectetur adipiscing elit."
            )
            .expect("failed to write test content");
        }

        let source_dir = temp_dir.path().join(format!("sources/{alias}"));
        std::fs::create_dir_all(&source_dir).expect("Failed to create sources dir");
        std::fs::write(source_dir.join("llms.txt"), long_content).expect("Failed to write content");

        let params = FindParams {
            query: None,
            snippets: Some(vec![format!("{alias}:1-200")]),
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 10,
            source: None,
            format: ResponseFormat::Concise,
            headings_only: false,
        };

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        let snippets = output
            .snippet_results
            .expect("expected snippet results in concise mode");
        let snippet = snippets.first().expect("missing snippet content");
        assert!(
            snippet.content.len() <= super::CONCISE_SNIPPET_CONTENT_CHARS + 3,
            "concise content should be truncated"
        );
        assert!(
            snippet.content.ends_with("..."),
            "concise content should end with ellipsis"
        );
    }

    #[tokio::test]
    async fn test_detailed_format_preserves_snippet_content() {
        let (storage, temp_dir) = setup_test_storage();
        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let alias = "detailed-source";

        let mut long_content = String::new();
        for i in 1..=200 {
            writeln!(
                &mut long_content,
                "Line {i}: Detailed format retains full content for validation."
            )
            .expect("failed to write test content");
        }

        let source_dir = temp_dir.path().join(format!("sources/{alias}"));
        std::fs::create_dir_all(&source_dir).expect("Failed to create sources dir");
        std::fs::write(source_dir.join("llms.txt"), long_content.clone())
            .expect("Failed to write content");

        let params = FindParams {
            query: None,
            snippets: Some(vec![format!("{alias}:1-200")]),
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 10,
            source: None,
            format: ResponseFormat::Detailed,
            headings_only: false,
        };

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        let snippets = output
            .snippet_results
            .expect("expected snippet results in detailed mode");
        let snippet = snippets.first().expect("missing snippet content");
        assert!(
            snippet.content.len() > super::CONCISE_SNIPPET_CONTENT_CHARS,
            "detailed format should retain full content"
        );
        assert!(
            !snippet.content.ends_with("..."),
            "detailed content should not be truncated"
        );
        assert!(
            snippet
                .content
                .contains("Detailed format retains full content"),
            "should include full text from source"
        );
    }

    #[tokio::test]
    async fn test_padding_boundary_validation() {
        let (storage, _temp_dir) = setup_test_storage();
        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));

        // Test valid padding values
        for padding in [0, 25, 50] {
            let params = FindParams {
                query: None,
                snippets: Some(vec!["test-source:2-4".to_string()]),
                context_mode: "symmetric".to_string(),
                line_padding: padding,
                max_results: 10,
                source: None,
                format: ResponseFormat::default(),
                headings_only: false,
            };

            let result = handle_find(params, &storage, &index_cache).await;
            assert!(result.is_ok(), "Padding {padding} should be valid");
        }

        // Test invalid padding value
        let params = FindParams {
            query: None,
            snippets: Some(vec!["test-source:2-4".to_string()]),
            context_mode: "symmetric".to_string(),
            line_padding: 51,
            max_results: 10,
            source: None,
            format: ResponseFormat::default(),
            headings_only: false,
        };

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), McpError::InvalidPadding(51)));
    }

    #[tokio::test]
    async fn test_invalid_citation_error_mapping() {
        let (storage, _temp_dir) = setup_test_storage();
        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));

        let params = FindParams {
            query: None,
            snippets: Some(vec!["invalid-citation".to_string()]),
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 10,
            source: None,
            format: ResponseFormat::default(),
            headings_only: false,
        };

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, McpError::InvalidCitation(_)));
        assert_eq!(err.error_code(), -32602); // Invalid params
    }

    #[tokio::test]
    async fn test_empty_query_rejected() {
        let (storage, _temp_dir) = setup_test_storage();
        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));

        // Test completely empty query
        let params = FindParams {
            query: Some(String::new()),
            snippets: None,
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 10,
            source: Some(SourceFilter::Single("test-source".to_string())),
            format: ResponseFormat::default(),
            headings_only: false,
        };

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), McpError::Internal(_)));

        // Test whitespace-only query
        let params = FindParams {
            query: Some("   ".to_string()),
            snippets: None,
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 10,
            source: Some(SourceFilter::Single("test-source".to_string())),
            format: ResponseFormat::default(),
            headings_only: false,
        };

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), McpError::Internal(_)));
    }

    #[tokio::test]
    async fn test_max_results_limit_enforced() {
        let (storage, _temp_dir) = setup_test_storage();
        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));

        // Test at limit
        let params = FindParams {
            query: None,
            snippets: Some(vec!["test-source:2-4".to_string()]),
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 1000,
            source: None,
            format: ResponseFormat::default(),
            headings_only: false,
        };

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_ok());

        // Test over limit
        let params = FindParams {
            query: None,
            snippets: Some(vec!["test-source:2-4".to_string()]),
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 1001,
            source: None,
            format: ResponseFormat::default(),
            headings_only: false,
        };

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), McpError::Internal(_)));
    }

    #[tokio::test]
    async fn test_empty_document_handling() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let storage =
            Storage::with_root(temp_dir.path().to_path_buf()).expect("Failed to create storage");

        // Create an empty llms.txt file
        std::fs::create_dir_all(temp_dir.path().join("sources/empty-source"))
            .expect("Failed to create sources dir");
        std::fs::write(temp_dir.path().join("sources/empty-source/llms.txt"), "")
            .expect("Failed to write empty content");

        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));

        let params = FindParams {
            query: None,
            snippets: Some(vec!["empty-source:1-2".to_string()]),
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 10,
            source: None,
            format: ResponseFormat::default(),
            headings_only: false,
        };

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, McpError::Internal(_)));
        assert!(err.to_string().contains("has no content"));
    }

    #[tokio::test]
    async fn test_both_params_none_rejected() {
        let (storage, _temp_dir) = setup_test_storage();
        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));

        let params = FindParams {
            query: None,
            snippets: None,
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 10,
            source: None,
            format: ResponseFormat::default(),
            headings_only: false,
        };

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, McpError::Internal(_)));
        assert!(
            err.to_string()
                .contains("Either query or snippets must be provided")
        );
    }

    #[tokio::test]
    async fn test_query_without_source_searches_all_sources() {
        let (storage, temp_dir) = setup_test_storage();
        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));

        // Create and index the test source
        let index_path = temp_dir.path().join("sources/test-source/.index");
        let index = SearchIndex::create(&index_path).expect("Failed to create index");

        // Store index in cache
        {
            let mut cache = index_cache.write().await;
            cache.insert("test-source".to_string(), Arc::new(index));
        }

        let params = FindParams {
            query: Some("section".to_string()),
            snippets: None,
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 10,
            source: None, // No source specified - should search all
            format: ResponseFormat::default(),
            headings_only: false,
        };

        let result = handle_find(params, &storage, &index_cache).await;
        if let Err(ref e) = result {
            eprintln!("Error in test_query_without_source_searches_all_sources: {e:?}");
        }
        assert!(
            result.is_ok(),
            "Should search all sources when no source specified"
        );

        let output = result.unwrap();
        assert!(output.executed.search_executed);
        assert!(output.search_results.is_some());
    }
}

#[cfg(test)]
mod block_detection_tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use blz_core::{Storage, TocEntry};

    /// Helper to create a simple TOC structure for testing
    fn create_test_toc() -> Vec<TocEntry> {
        vec![
            TocEntry {
                heading_path: vec!["Introduction".to_string()],
                heading_path_display: Some(vec!["Introduction".to_string()]),
                heading_path_normalized: Some(vec!["introduction".to_string()]),
                lines: "1-10".to_string(),
                anchor: None,
                children: vec![],
            },
            TocEntry {
                heading_path: vec!["Getting Started".to_string()],
                heading_path_display: Some(vec!["Getting Started".to_string()]),
                heading_path_normalized: Some(vec!["getting started".to_string()]),
                lines: "11-50".to_string(),
                anchor: None,
                children: vec![
                    TocEntry {
                        heading_path: vec![
                            "Getting Started".to_string(),
                            "Installation".to_string(),
                        ],
                        heading_path_display: Some(vec![
                            "Getting Started".to_string(),
                            "Installation".to_string(),
                        ]),
                        heading_path_normalized: Some(vec![
                            "getting started".to_string(),
                            "installation".to_string(),
                        ]),
                        lines: "12-25".to_string(),
                        anchor: None,
                        children: vec![],
                    },
                    TocEntry {
                        heading_path: vec![
                            "Getting Started".to_string(),
                            "Configuration".to_string(),
                        ],
                        heading_path_display: Some(vec![
                            "Getting Started".to_string(),
                            "Configuration".to_string(),
                        ]),
                        heading_path_normalized: Some(vec![
                            "getting started".to_string(),
                            "configuration".to_string(),
                        ]),
                        lines: "26-50".to_string(),
                        anchor: None,
                        children: vec![],
                    },
                ],
            },
            TocEntry {
                heading_path: vec!["API Reference".to_string()],
                heading_path_display: Some(vec!["API Reference".to_string()]),
                heading_path_normalized: Some(vec!["api reference".to_string()]),
                lines: "51-100".to_string(),
                anchor: None,
                children: vec![],
            },
        ]
    }

    #[test]
    fn test_find_containing_block_top_level() {
        let toc = create_test_toc();

        // Request falls in top-level "Introduction" block
        let result = find_containing_block(&toc, 5, 8);
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert_eq!(start, 0); // 1-based line 1 -> 0-based index 0
        assert_eq!(end, 9); // 1-based line 10 -> 0-based index 9
    }

    #[test]
    fn test_find_containing_block_nested() {
        let toc = create_test_toc();

        // Request falls in nested "Installation" block (H2 level: heading_path length = 2)
        // Should expand to section boundary: from Installation start until next H2 or H1
        // "Installation" is lines 12-25, next H2 is "Configuration" at line 26
        let result = find_containing_block(&toc, 15, 20);
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        // Should return boundary: from Installation (line 12) to before Configuration (line 25)
        assert_eq!(start, 11); // 1-based line 12 -> 0-based index 11
        assert_eq!(end, 24); // 1-based line 25 -> 0-based index 24
    }

    #[test]
    fn test_find_containing_block_expands_to_section_boundary() {
        let toc = create_test_toc();

        // Request at line 30 falls in both "Getting Started" (11-50) and "Configuration" (26-50)
        // Should find the deepest entry ("Configuration" at H2) and expand to section boundary
        // "Configuration" is H2, next H2 or H1 is "API Reference" at line 51
        let result = find_containing_block(&toc, 30, 35);
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert_eq!(start, 25); // Configuration starts at line 26 -> 0-based index 25
        assert_eq!(end, 49); // Expands to before "API Reference" (line 51 -> end at 50 -> 0-based index 49)
    }

    #[test]
    fn test_find_containing_block_no_match() {
        let toc = create_test_toc();

        // Request outside all blocks (line 200)
        let result = find_containing_block(&toc, 200, 210);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_containing_block_exact_boundaries() {
        let toc = create_test_toc();

        // Request exactly matching "API Reference" block (51-100)
        let result = find_containing_block(&toc, 51, 100);
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert_eq!(start, 50); // Line 51 -> index 50
        assert_eq!(end, 99); // Line 100 -> index 99
    }

    #[test]
    fn test_find_containing_block_single_line() {
        let toc = create_test_toc();

        // Request single line within the Installation block
        // "Installation" is H2 (level 2), next H2 or H1 is "Configuration" at line 26
        let result = find_containing_block(&toc, 15, 15);
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        // Should expand to section boundary: Installation (12-25)
        assert_eq!(start, 11); // Line 12 -> 0-based index 11
        assert_eq!(end, 24); // Line 25 -> 0-based index 24
    }

    #[test]
    fn test_find_containing_block_invalid_format() {
        let toc = vec![
            TocEntry {
                heading_path: vec!["Bad Entry".to_string()],
                heading_path_display: Some(vec!["Bad Entry".to_string()]),
                heading_path_normalized: Some(vec!["bad entry".to_string()]),
                lines: "invalid".to_string(), // Bad format
                anchor: None,
                children: vec![],
            },
            TocEntry {
                heading_path: vec!["Good Entry".to_string()],
                heading_path_display: Some(vec!["Good Entry".to_string()]),
                heading_path_normalized: Some(vec!["good entry".to_string()]),
                lines: "10-20".to_string(),
                anchor: None,
                children: vec![],
            },
        ];

        // Should skip invalid entry and find good one
        let result = find_containing_block(&toc, 15, 18);
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert_eq!(start, 9); // Line 10
        assert_eq!(end, 19); // Line 20
    }

    #[test]
    fn test_find_containing_block_empty_toc() {
        let toc: Vec<TocEntry> = vec![];
        let result = find_containing_block(&toc, 10, 20);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_containing_block_cross_section_range_returns_none() {
        let toc = create_test_toc();

        // Request range 8-15 spans two top-level sections:
        // - Introduction (1-10) contains line 8 but not line 15
        // - Getting Started (11-50) contains line 15 but not line 8
        // Should return None to fall back to symmetric padding
        let result = find_containing_block(&toc, 8, 15);
        assert!(
            result.is_none(),
            "Cross-section ranges should return None to trigger fallback"
        );
    }

    #[test]
    fn test_find_containing_block_range_entirely_within_section() {
        let toc = create_test_toc();

        // Request range 20-24 is entirely within Installation (12-25)
        // Should find and return the Installation section boundary
        let result = find_containing_block(&toc, 20, 24);
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        // Should return Installation section (12-25)
        assert_eq!(start, 11); // Line 12 -> 0-based index 11
        assert_eq!(end, 24); // Line 25 -> 0-based index 24
    }

    #[tokio::test]
    async fn test_context_mode_all_with_toc() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let storage =
            Storage::with_root(temp_dir.path().to_path_buf()).expect("Failed to create storage");

        // Create test content with clear sections
        let test_content = "# Documentation\n\
Line 2\n\
\n\
## Section A\n\
Line 5\n\
Line 6\n\
Line 7\n\
\n\
## Section B\n\
Line 10\n\
Line 11\n\
Line 12";

        // Create llms.txt
        std::fs::create_dir_all(temp_dir.path().join("sources/test-toc"))
            .expect("Failed to create sources dir");
        std::fs::write(
            temp_dir.path().join("sources/test-toc/llms.txt"),
            test_content,
        )
        .expect("Failed to write test content");

        // Create llms.json with TOC
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        #[allow(clippy::cast_possible_wrap)]
        let fetched_at =
            chrono::DateTime::from_timestamp(now as i64, 0).expect("Failed to create timestamp");

        let llms_json = blz_core::LlmsJson {
            source: "test-toc".to_string(),
            metadata: blz_core::Source {
                url: "https://example.com".to_string(),
                etag: None,
                last_modified: None,
                fetched_at,
                sha256: "test".to_string(),
                variant: blz_core::SourceVariant::Llms,
                aliases: vec![],
                tags: vec![],
                description: None,
                category: None,
                npm_aliases: vec![],
                github_aliases: vec![],
                origin: blz_core::SourceOrigin {
                    manifest: None,
                    source_type: None,
                },
            },
            toc: vec![
                TocEntry {
                    heading_path: vec!["Documentation".to_string()],
                    heading_path_display: Some(vec!["Documentation".to_string()]),
                    heading_path_normalized: Some(vec!["documentation".to_string()]),
                    lines: "1-3".to_string(),
                    anchor: None,
                    children: vec![],
                },
                TocEntry {
                    heading_path: vec!["Section A".to_string()],
                    heading_path_display: Some(vec!["Section A".to_string()]),
                    heading_path_normalized: Some(vec!["section a".to_string()]),
                    lines: "4-8".to_string(),
                    anchor: None,
                    children: vec![],
                },
                TocEntry {
                    heading_path: vec!["Section B".to_string()],
                    heading_path_display: Some(vec!["Section B".to_string()]),
                    heading_path_normalized: Some(vec!["section b".to_string()]),
                    lines: "9-12".to_string(),
                    anchor: None,
                    children: vec![],
                },
            ],
            files: vec![],
            line_index: blz_core::LineIndex {
                total_lines: 12,
                byte_offsets: false,
            },
            diagnostics: vec![],
            parse_meta: None,
        };

        let json_str = serde_json::to_string(&llms_json).expect("Failed to serialize JSON");
        std::fs::write(temp_dir.path().join("sources/test-toc/llms.json"), json_str)
            .expect("Failed to write llms.json");

        // Request lines 5-6 (in Section A: 4-8)
        let result = retrieve_snippet(&storage, "test-toc", 5, 6, "all", 0);
        assert!(result.is_ok());

        let snippet = result.unwrap();
        // Should return entire Section A block (lines 4-8)
        assert_eq!(snippet.line_start, 4);
        assert_eq!(snippet.line_end, 8);
        assert!(snippet.content.contains("## Section A"));
        assert!(snippet.content.contains("Line 7"));
        assert!(!snippet.content.contains("Section B"));
    }

    #[tokio::test]
    async fn test_context_mode_all_fallback_no_toc() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let storage =
            Storage::with_root(temp_dir.path().to_path_buf()).expect("Failed to create storage");

        let test_content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n";

        std::fs::create_dir_all(temp_dir.path().join("sources/no-toc"))
            .expect("Failed to create sources dir");
        std::fs::write(
            temp_dir.path().join("sources/no-toc/llms.txt"),
            test_content,
        )
        .expect("Failed to write test content");

        // Don't create llms.json - test fallback behavior

        // Request line 3
        let result = retrieve_snippet(&storage, "no-toc", 3, 3, "all", 0);
        assert!(result.is_ok());

        let snippet = result.unwrap();
        // Should fall back to symmetric padding (20 lines each side)
        // Document only has 5 lines, so should get full document (1-5)
        assert_eq!(snippet.line_start, 1);
        assert_eq!(snippet.line_end, 5);
    }

    #[tokio::test]
    async fn test_context_mode_all_clamps_out_of_bounds_toc() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let storage =
            Storage::with_root(temp_dir.path().to_path_buf()).expect("Failed to create storage");

        let test_content = "Line 1\nLine 2\nLine 3\n";

        let source_dir = temp_dir.path().join("sources/out-of-bounds");
        std::fs::create_dir_all(&source_dir).expect("Failed to create sources dir");
        std::fs::write(source_dir.join("llms.txt"), test_content)
            .expect("Failed to write llms.txt");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        #[allow(clippy::cast_possible_wrap)]
        let fetched_at =
            chrono::DateTime::from_timestamp(now as i64, 0).expect("Failed to create timestamp");

        let llms_json = blz_core::LlmsJson {
            source: "out-of-bounds".to_string(),
            metadata: blz_core::Source {
                url: "https://example.com".to_string(),
                etag: None,
                last_modified: None,
                fetched_at,
                sha256: "test".to_string(),
                variant: blz_core::SourceVariant::Llms,
                aliases: vec![],
                tags: vec![],
                description: None,
                category: None,
                npm_aliases: vec![],
                github_aliases: vec![],
                origin: blz_core::SourceOrigin {
                    manifest: None,
                    source_type: None,
                },
            },
            toc: vec![TocEntry {
                heading_path: vec!["Overflow".to_string()],
                heading_path_display: Some(vec!["Overflow".to_string()]),
                heading_path_normalized: Some(vec!["overflow".to_string()]),
                lines: "1-10".to_string(),
                anchor: None,
                children: vec![],
            }],
            files: vec![],
            line_index: blz_core::LineIndex {
                total_lines: 3,
                byte_offsets: false,
            },
            diagnostics: vec![],
            parse_meta: None,
        };

        let json_str = serde_json::to_string(&llms_json).expect("Failed to serialize JSON");
        std::fs::write(source_dir.join("llms.json"), json_str).expect("Failed to write llms.json");

        let result = retrieve_snippet(&storage, "out-of-bounds", 1, 2, "all", 0);
        assert!(result.is_ok());

        let snippet = result.unwrap();
        assert_eq!(snippet.line_start, 1);
        assert_eq!(snippet.line_end, 3);
        assert!(snippet.content.contains("Line 3"));
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn test_context_mode_all_preserves_requested_range_across_sections() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let storage =
            Storage::with_root(temp_dir.path().to_path_buf()).expect("Failed to create storage");

        let test_content = "# Doc
Line 2

## Section A
A line 1
A line 2
A line 3

## Section B
B line 1
B line 2
B line 3
";

        std::fs::create_dir_all(temp_dir.path().join("sources/cross-sections"))
            .expect("Failed to create sources dir");
        std::fs::write(
            temp_dir.path().join("sources/cross-sections/llms.txt"),
            test_content,
        )
        .expect("Failed to write test content");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        #[allow(clippy::cast_possible_wrap)]
        let fetched_at =
            chrono::DateTime::from_timestamp(now as i64, 0).expect("Failed to create timestamp");

        let llms_json = blz_core::LlmsJson {
            source: "cross-sections".to_string(),
            metadata: blz_core::Source {
                url: "https://example.com".to_string(),
                etag: None,
                last_modified: None,
                fetched_at,
                sha256: "test".to_string(),
                variant: blz_core::SourceVariant::Llms,
                aliases: vec![],
                tags: vec![],
                description: None,
                category: None,
                npm_aliases: vec![],
                github_aliases: vec![],
                origin: blz_core::SourceOrigin {
                    manifest: None,
                    source_type: None,
                },
            },
            toc: vec![
                TocEntry {
                    heading_path: vec!["Section A".to_string()],
                    heading_path_display: Some(vec!["Section A".to_string()]),
                    heading_path_normalized: Some(vec!["section a".to_string()]),
                    lines: "4-7".to_string(),
                    anchor: None,
                    children: vec![],
                },
                TocEntry {
                    heading_path: vec!["Section B".to_string()],
                    heading_path_display: Some(vec!["Section B".to_string()]),
                    heading_path_normalized: Some(vec!["section b".to_string()]),
                    lines: "9-11".to_string(),
                    anchor: None,
                    children: vec![],
                },
            ],
            files: vec![],
            line_index: blz_core::LineIndex {
                total_lines: 12,
                byte_offsets: false,
            },
            diagnostics: vec![],
            parse_meta: None,
        };

        let json_str = serde_json::to_string(&llms_json).expect("Failed to serialize JSON");
        std::fs::write(
            temp_dir.path().join("sources/cross-sections/llms.json"),
            json_str,
        )
        .expect("Failed to write llms.json");

        let result = retrieve_snippet(&storage, "cross-sections", 6, 10, "all", 0);
        assert!(
            result.is_ok(),
            "context all should succeed for multi-section range"
        );

        let snippet = result.unwrap();
        assert!(
            snippet.line_start <= 6,
            "snippet should not start after requested range"
        );
        assert!(
            snippet.line_end >= 10,
            "snippet should include the full requested range"
        );
        assert!(
            snippet.content.contains("A line 3"),
            "snippet should include content from Section A"
        );
        assert!(
            snippet.content.contains("B line 2"),
            "snippet should include content from Section B"
        );
    }
}
