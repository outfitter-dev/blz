//! Find tool implementation for searching and retrieving documentation snippets

use blz_core::{SearchIndex, Storage};
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

/// Parameters for the find tool
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

    /// Optional source filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Response format: "concise" (default) or "detailed"
    ///
    /// Concise returns minimal data, detailed includes all metadata.
    /// Based on Anthropic research showing 30-65% token savings with concise mode.
    #[serde(default)]
    pub format: ResponseFormat,
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
) -> McpResult<Vec<SearchHitResult>> {
    let hits = index.search(query, source_filter, max_results)?;

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

/// Find the smallest TOC entry containing the given line range.
///
/// Recursively searches the TOC tree to find the block that contains
/// the requested lines. Returns the block boundaries if found.
///
/// ## Arguments
///
/// - `toc`: TOC entries to search (may be nested)
/// - `start_line`: Starting line (1-based)
/// - `end_line`: Ending line (1-based, inclusive)
///
/// ## Returns
///
/// `Some((block_start, block_end))` with 0-based indices if a containing
/// block is found, `None` otherwise.
#[tracing::instrument]
fn find_containing_block(
    toc: &[blz_core::TocEntry],
    start_line: usize,
    end_line: usize,
) -> Option<(usize, usize)> {
    let mut best_match: Option<(usize, usize)> = None;
    let mut best_size = usize::MAX;

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

        // Check if this block contains our requested range
        if block_start <= start_line && end_line <= block_end {
            let block_size = block_end - block_start;

            // Track smallest containing block
            if block_size < best_size {
                // Convert to 0-based indexing for return value
                best_match = Some((block_start.saturating_sub(1), block_end.saturating_sub(1)));
                best_size = block_size;
            }

            // Recursively check children for a tighter match
            if !entry.children.is_empty() {
                if let Some(child_match) =
                    find_containing_block(&entry.children, start_line, end_line)
                {
                    let child_size = child_match.1 - child_match.0;
                    if child_size < best_size {
                        best_match = Some(child_match);
                        best_size = child_size;
                    }
                }
            }
        }
    }

    best_match
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

        // For now, we require a source to be specified for search
        // In the future, we could search across all sources
        let source = params.source.as_ref().ok_or_else(|| {
            crate::error::McpError::MissingParameter(
                "source (required for search operations - specify which documentation source to search)"
                    .to_string(),
            )
        })?;

        // Get or load the index for the specified source
        let index = cache::get_or_load_index(index_cache, storage, source).await?;

        let results = execute_search(&index, query, Some(source), params.max_results).await?;

        tracing::debug!(count = results.len(), "search completed");
        search_results = Some(results);
    }

    // Retrieve snippets if requested
    if let Some(ref citations) = params.snippets {
        tracing::debug!(count = citations.len(), "retrieving snippets");

        let mut results = Vec::new();

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
    use super::*;

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
            source: Some("test-source".to_string()),
            format: ResponseFormat::default(),
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
            source: Some("test-source".to_string()),
            format: ResponseFormat::default(),
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
            source: Some("test-source".to_string()),
            format: ResponseFormat::default(),
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
            source: Some("test-source".to_string()),
            format: ResponseFormat::default(),
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
    async fn test_query_without_source_returns_missing_parameter_error() {
        let (storage, _temp_dir) = setup_test_storage();
        let index_cache: IndexCache = Arc::new(RwLock::new(std::collections::HashMap::new()));

        let params = FindParams {
            query: Some("test query".to_string()),
            snippets: None,
            context_mode: "none".to_string(),
            line_padding: 0,
            max_results: 10,
            source: None, // Missing required source parameter
            format: ResponseFormat::default(),
        };

        let result = handle_find(params, &storage, &index_cache).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, McpError::MissingParameter(_)));
        assert_eq!(err.error_code(), -32602); // Invalid params
        assert!(err.to_string().contains("source"));
        assert!(err.to_string().contains("required for search operations"));
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
                lines: "1-10".to_string(),
                anchor: None,
                children: vec![],
            },
            TocEntry {
                heading_path: vec!["Getting Started".to_string()],
                lines: "11-50".to_string(),
                anchor: None,
                children: vec![
                    TocEntry {
                        heading_path: vec![
                            "Getting Started".to_string(),
                            "Installation".to_string(),
                        ],
                        lines: "12-25".to_string(),
                        anchor: None,
                        children: vec![],
                    },
                    TocEntry {
                        heading_path: vec![
                            "Getting Started".to_string(),
                            "Configuration".to_string(),
                        ],
                        lines: "26-50".to_string(),
                        anchor: None,
                        children: vec![],
                    },
                ],
            },
            TocEntry {
                heading_path: vec!["API Reference".to_string()],
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

        // Request falls in nested "Installation" block
        let result = find_containing_block(&toc, 15, 20);
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        // Should return the smallest containing block (Installation: 12-25)
        assert_eq!(start, 11); // 1-based line 12 -> 0-based index 11
        assert_eq!(end, 24); // 1-based line 25 -> 0-based index 24
    }

    #[test]
    fn test_find_containing_block_prefers_smallest() {
        let toc = create_test_toc();

        // Request at line 30 falls in both "Getting Started" (11-50) and "Configuration" (26-50)
        // Should return the smaller "Configuration" block
        let result = find_containing_block(&toc, 30, 35);
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert_eq!(start, 25); // Configuration starts at line 26
        assert_eq!(end, 49); // Configuration ends at line 50
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

        // Request single line within a block
        let result = find_containing_block(&toc, 15, 15);
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        // Should find Installation block (12-25)
        assert_eq!(start, 11);
        assert_eq!(end, 24);
    }

    #[test]
    fn test_find_containing_block_invalid_format() {
        let toc = vec![
            TocEntry {
                heading_path: vec!["Bad Entry".to_string()],
                lines: "invalid".to_string(), // Bad format
                anchor: None,
                children: vec![],
            },
            TocEntry {
                heading_path: vec!["Good Entry".to_string()],
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
                    lines: "1-3".to_string(),
                    anchor: None,
                    children: vec![],
                },
                TocEntry {
                    heading_path: vec!["Section A".to_string()],
                    lines: "4-8".to_string(),
                    anchor: None,
                    children: vec![],
                },
                TocEntry {
                    heading_path: vec!["Section B".to_string()],
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

        // Document has only three lines.
        let test_content = "Line 1\nLine 2\nLine 3\n";

        let source_dir = temp_dir.path().join("sources/out-of-bounds");
        std::fs::create_dir_all(&source_dir).expect("Failed to create sources dir");
        std::fs::write(source_dir.join("llms.txt"), test_content)
            .expect("Failed to write llms.txt");

        // Persist llms.json with a TOC entry that overflows past the end of the file.
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
        // Even though TOC says 1-10, the snippet should clamp to the file length.
        assert_eq!(snippet.line_start, 1);
        assert_eq!(snippet.line_end, 3);
        assert!(snippet.content.contains("Line 3"));
    }
}
