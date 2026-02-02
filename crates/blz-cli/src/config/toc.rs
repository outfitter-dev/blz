//! TOC/Map configuration for table of contents commands.
//!
//! This module provides [`TocConfig`], which bundles TOC display and navigation
//! parameters to reduce argument counts in execute functions.

use crate::output::OutputFormat;
use crate::utils::heading_filter::HeadingLevelFilter;

/// TOC display configuration.
///
/// Controls how table of contents is displayed and navigated.
///
/// # Examples
///
/// ```ignore
/// use blz_cli::config::TocConfig;
/// use blz_cli::output::OutputFormat;
///
/// let config = TocConfig::new(OutputFormat::Json)
///     .with_tree(true)
///     .with_max_depth(Some(2));
/// ```
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct TocConfig {
    /// Output format (text, json, jsonl).
    pub format: OutputFormat,

    /// Filter headings by boolean expression.
    pub filter_expr: Option<String>,

    /// Maximum heading depth to display (1-6).
    pub max_depth: Option<u8>,

    /// Filter by heading level.
    pub heading_level: Option<HeadingLevelFilter>,

    /// Maximum number of headings per page.
    pub limit: Option<usize>,

    /// Page number for pagination.
    pub page: usize,

    /// Display as hierarchical tree.
    pub tree: bool,

    /// Show anchor metadata and remap history.
    pub anchors: bool,

    /// Show anchor slugs in normal output.
    pub show_anchors: bool,

    /// Suppress non-essential output.
    pub quiet: bool,
}

/// TOC navigation configuration.
///
/// Controls pagination and navigation between pages.
#[derive(Debug, Clone, Copy, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct TocNavigation {
    /// Continue from previous results (next page).
    pub next: bool,

    /// Go back to previous page.
    pub previous: bool,

    /// Jump to last page of results.
    pub last: bool,

    /// Include all sources (or bypass pagination limits).
    pub all: bool,
}

impl Default for TocConfig {
    fn default() -> Self {
        Self {
            format: OutputFormat::Text,
            filter_expr: None,
            max_depth: None,
            heading_level: None,
            limit: None,
            page: 1,
            tree: false,
            anchors: false,
            show_anchors: false,
            quiet: false,
        }
    }
}

impl TocConfig {
    /// Create a new TOC configuration with the specified format.
    #[must_use]
    pub const fn new(format: OutputFormat) -> Self {
        Self {
            format,
            filter_expr: None,
            max_depth: None,
            heading_level: None,
            limit: None,
            page: 1,
            tree: false,
            anchors: false,
            show_anchors: false,
            quiet: false,
        }
    }

    /// Set the filter expression.
    #[must_use]
    pub fn with_filter_expr(mut self, filter_expr: Option<String>) -> Self {
        self.filter_expr = filter_expr;
        self
    }

    /// Set the maximum depth.
    #[must_use]
    pub const fn with_max_depth(mut self, max_depth: Option<u8>) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Set the heading level filter.
    #[must_use]
    pub fn with_heading_level(mut self, heading_level: Option<HeadingLevelFilter>) -> Self {
        self.heading_level = heading_level;
        self
    }

    /// Set the result limit.
    #[must_use]
    pub const fn with_limit(mut self, limit: Option<usize>) -> Self {
        self.limit = limit;
        self
    }

    /// Set the page number.
    #[must_use]
    pub const fn with_page(mut self, page: usize) -> Self {
        self.page = page;
        self
    }

    /// Set whether to display as tree.
    #[must_use]
    pub const fn with_tree(mut self, tree: bool) -> Self {
        self.tree = tree;
        self
    }

    /// Set whether to show anchors metadata.
    #[must_use]
    pub const fn with_anchors(mut self, anchors: bool) -> Self {
        self.anchors = anchors;
        self
    }

    /// Set whether to show anchor slugs.
    #[must_use]
    pub const fn with_show_anchors(mut self, show_anchors: bool) -> Self {
        self.show_anchors = show_anchors;
        self
    }

    /// Set quiet mode.
    #[must_use]
    pub const fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }
}

impl TocNavigation {
    /// Create a new navigation configuration.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next: false,
            previous: false,
            last: false,
            all: false,
        }
    }

    /// Set whether to continue to next page.
    #[must_use]
    pub const fn with_next(mut self, next: bool) -> Self {
        self.next = next;
        self
    }

    /// Set whether to go to previous page.
    #[must_use]
    pub const fn with_previous(mut self, previous: bool) -> Self {
        self.previous = previous;
        self
    }

    /// Set whether to jump to last page.
    #[must_use]
    pub const fn with_last(mut self, last: bool) -> Self {
        self.last = last;
        self
    }

    /// Set whether to include all sources.
    #[must_use]
    pub const fn with_all(mut self, all: bool) -> Self {
        self.all = all;
        self
    }

    /// Check if any navigation flag is set.
    #[must_use]
    pub const fn is_navigating(self) -> bool {
        self.next || self.previous || self.last
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toc_config_default() {
        let config = TocConfig::default();
        assert_eq!(config.format, OutputFormat::Text);
        assert!(config.filter_expr.is_none());
        assert!(config.max_depth.is_none());
        assert!(config.heading_level.is_none());
        assert!(config.limit.is_none());
        assert_eq!(config.page, 1);
        assert!(!config.tree);
        assert!(!config.anchors);
        assert!(!config.show_anchors);
        assert!(!config.quiet);
    }

    #[test]
    fn test_toc_config_builder() {
        let config = TocConfig::new(OutputFormat::Json)
            .with_tree(true)
            .with_max_depth(Some(2))
            .with_limit(Some(50))
            .with_page(3)
            .with_anchors(true)
            .with_show_anchors(true)
            .with_quiet(true);

        assert_eq!(config.format, OutputFormat::Json);
        assert!(config.tree);
        assert_eq!(config.max_depth, Some(2));
        assert_eq!(config.limit, Some(50));
        assert_eq!(config.page, 3);
        assert!(config.anchors);
        assert!(config.show_anchors);
        assert!(config.quiet);
    }

    #[test]
    fn test_navigation_default() {
        let nav = TocNavigation::default();
        assert!(!nav.next);
        assert!(!nav.previous);
        assert!(!nav.last);
        assert!(!nav.all);
        assert!(!nav.is_navigating());
    }

    #[test]
    fn test_navigation_builder() {
        let nav = TocNavigation::new().with_next(true).with_all(true);

        assert!(nav.next);
        assert!(!nav.previous);
        assert!(!nav.last);
        assert!(nav.all);
        assert!(nav.is_navigating());
    }

    #[test]
    fn test_is_navigating() {
        assert!(TocNavigation::new().with_next(true).is_navigating());
        assert!(TocNavigation::new().with_previous(true).is_navigating());
        assert!(TocNavigation::new().with_last(true).is_navigating());
        assert!(!TocNavigation::new().with_all(true).is_navigating());
    }
}
