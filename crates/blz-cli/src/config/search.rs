//! Search configuration for query and find commands.
//!
//! This module provides [`SearchConfig`], which bundles search-specific
//! parameters to reduce argument counts in execute functions.

use crate::utils::heading_filter::HeadingLevelFilter;

/// Search configuration for query and find commands.
///
/// Bundles search parameters like pagination, filtering, and result limiting
/// to reduce the number of arguments passed to execute functions.
///
/// # Examples
///
/// ```ignore
/// use blz_cli::config::SearchConfig;
///
/// let config = SearchConfig::default()
///     .with_limit(20)
///     .with_heading_filter("<=2".parse().ok());
/// ```
#[derive(Debug, Clone, Default)]
pub struct SearchConfig {
    /// Maximum number of results to return per page.
    pub limit: usize,

    /// Page number for pagination (1-indexed).
    pub page: usize,

    /// Show only top N percentile of results (1-100).
    pub top_percentile: Option<u8>,

    /// Filter results by heading level.
    pub heading_filter: Option<HeadingLevelFilter>,

    /// Restrict matches to heading text only.
    pub headings_only: bool,

    /// Jump to the last page of results.
    pub last: bool,

    /// Don't save this search to history.
    pub no_history: bool,
}

impl SearchConfig {
    /// Create a new search configuration with default values.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            limit: 50,
            page: 1,
            top_percentile: None,
            heading_filter: None,
            headings_only: false,
            last: false,
            no_history: false,
        }
    }

    /// Set the result limit.
    #[must_use]
    pub const fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set the page number.
    #[must_use]
    pub const fn with_page(mut self, page: usize) -> Self {
        self.page = page;
        self
    }

    /// Set the top percentile filter.
    #[must_use]
    pub const fn with_top_percentile(mut self, percentile: Option<u8>) -> Self {
        self.top_percentile = percentile;
        self
    }

    /// Set the heading level filter.
    #[must_use]
    pub fn with_heading_filter(mut self, filter: Option<HeadingLevelFilter>) -> Self {
        self.heading_filter = filter;
        self
    }

    /// Set whether to match headings only.
    #[must_use]
    pub const fn with_headings_only(mut self, headings_only: bool) -> Self {
        self.headings_only = headings_only;
        self
    }

    /// Set whether to jump to the last page.
    #[must_use]
    pub const fn with_last(mut self, last: bool) -> Self {
        self.last = last;
        self
    }

    /// Set whether to skip history recording.
    #[must_use]
    pub const fn with_no_history(mut self, no_history: bool) -> Self {
        self.no_history = no_history;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let config = SearchConfig::default();
        assert_eq!(config.limit, 0);
        assert_eq!(config.page, 0);
        assert!(config.top_percentile.is_none());
        assert!(config.heading_filter.is_none());
        assert!(!config.headings_only);
        assert!(!config.last);
        assert!(!config.no_history);
    }

    #[test]
    fn test_new() {
        let config = SearchConfig::new();
        assert_eq!(config.limit, 50);
        assert_eq!(config.page, 1);
        assert!(!config.no_history);
    }

    #[test]
    fn test_builder() {
        let config = SearchConfig::new()
            .with_limit(20)
            .with_page(3)
            .with_top_percentile(Some(90))
            .with_headings_only(true)
            .with_last(true)
            .with_no_history(true);

        assert_eq!(config.limit, 20);
        assert_eq!(config.page, 3);
        assert_eq!(config.top_percentile, Some(90));
        assert!(config.headings_only);
        assert!(config.last);
        assert!(config.no_history);
    }

    #[test]
    fn test_with_heading_filter() {
        let parsed = "<=2".parse::<HeadingLevelFilter>();
        assert!(parsed.is_ok(), "expected valid heading filter");
        let filter = parsed.ok();
        let config = SearchConfig::new().with_heading_filter(filter.clone());

        assert_eq!(config.heading_filter, filter);
    }
}
