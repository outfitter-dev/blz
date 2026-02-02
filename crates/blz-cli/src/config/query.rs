//! Query execution configuration.
//!
//! This module provides [`QueryExecutionConfig`], which bundles all four
//! query-related config structs into a single unified configuration.

use super::{ContentConfig, DisplayConfig, SearchConfig, SnippetConfig};

/// Unified query execution configuration.
///
/// Combines all query-related configs into a single struct to reduce
/// parameter counts in execute functions.
///
/// # Examples
///
/// ```ignore
/// use blz_cli::config::QueryExecutionConfig;
///
/// let config = QueryExecutionConfig::new(
///     SearchConfig::new().with_limit(20),
///     DisplayConfig::new(OutputFormat::Json),
///     SnippetConfig::new(),
///     ContentConfig::new(),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct QueryExecutionConfig {
    /// Search parameters (limit, page, filters).
    pub search: SearchConfig,

    /// Display parameters (format, show, summary).
    pub display: DisplayConfig,

    /// Snippet parameters (lines, `max_chars`, precision).
    pub snippet: SnippetConfig,

    /// Content parameters (context, block, copy).
    pub content: ContentConfig,
}

impl QueryExecutionConfig {
    /// Create a new query execution configuration.
    #[must_use]
    pub const fn new(
        search: SearchConfig,
        display: DisplayConfig,
        snippet: SnippetConfig,
        content: ContentConfig,
    ) -> Self {
        Self {
            search,
            display,
            snippet,
            content,
        }
    }
}

impl Default for QueryExecutionConfig {
    fn default() -> Self {
        Self {
            search: SearchConfig::new(),
            display: DisplayConfig::default(),
            snippet: SnippetConfig::default(),
            content: ContentConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let config = QueryExecutionConfig::default();
        assert_eq!(config.search.limit, 50);
        assert!(!config.display.quiet);
        assert_eq!(config.snippet.lines, 3);
        assert!(!config.content.copy);
    }

    #[test]
    fn test_new() {
        let search = SearchConfig::new().with_limit(100);
        let display = DisplayConfig::default().with_quiet(true);
        let snippet = SnippetConfig::new().with_lines(5);
        let content = ContentConfig::new().with_copy(true);

        let config = QueryExecutionConfig::new(search, display, snippet, content);

        assert_eq!(config.search.limit, 100);
        assert!(config.display.quiet);
        assert_eq!(config.snippet.lines, 5);
        assert!(config.content.copy);
    }
}
