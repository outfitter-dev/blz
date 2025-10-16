//! Find tool implementation for searching and retrieving documentation snippets

use blz_core::{SearchIndex, Storage};
use serde::{Deserialize, Serialize};

use crate::{cache, error::McpResult, types::IndexCache};

/// Default maximum number of search results
const DEFAULT_MAX_RESULTS: usize = 10;
/// Maximum line padding allowed
const MAX_LINE_PADDING: u32 = 50;
/// Maximum allowed search results
const MAX_ALLOWED_RESULTS: usize = 1000;

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
    let ranges_str = parts[1];

    let mut ranges = Vec::new();

    for range_str in ranges_str.split(',') {
        let range_parts: Vec<&str> = range_str.split('-').collect();

        if range_parts.len() != 2 {
            return Err(format!(
                "Invalid range format: {range_str}. Expected 'start-end'"
            ));
        }

        let start = range_parts[0]
            .parse::<usize>()
            .map_err(|_| format!("Invalid line number: {}", range_parts[0]))?;

        let end = range_parts[1]
            .parse::<usize>()
            .map_err(|_| format!("Invalid line number: {}", range_parts[1]))?;

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
            // Find the block boundaries - this is simplified, could be improved
            // For now, just return entire document for "all" mode
            (0, lines.len().saturating_sub(1))
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
            crate::error::McpError::Internal(
                "Source must be specified for search (multi-source search not yet implemented)"
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
}

#[cfg(test)]
mod integration_tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::needless_raw_string_hashes)]

    use super::*;
    use crate::{error::McpError, types::IndexCache};
    use blz_core::{SearchIndex, Storage};
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
}
