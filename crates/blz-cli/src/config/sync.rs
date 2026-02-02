//! Sync configuration for sync and refresh commands.
//!
//! This module provides [`SyncConfig`], which bundles sync/refresh parameters
//! to reduce argument counts in execute functions.

/// Sync configuration.
///
/// Controls how documentation sources are synced/refreshed.
///
/// # Examples
///
/// ```ignore
/// use blz_cli::config::SyncConfig;
///
/// let config = SyncConfig::new()
///     .with_reindex(true)
///     .with_quiet(true);
/// ```
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct SyncConfig {
    /// Skip confirmation prompts.
    pub yes: bool,

    /// Force re-parse and re-index even if content unchanged.
    pub reindex: bool,

    /// Content filters to enable (comma-separated).
    pub filter: Option<String>,

    /// Disable all content filters.
    pub no_filter: bool,

    /// Suppress informational output.
    pub quiet: bool,
}

impl SyncConfig {
    /// Create a new sync configuration with default values.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            yes: false,
            reindex: false,
            filter: None,
            no_filter: false,
            quiet: false,
        }
    }

    /// Set whether to skip confirmations.
    #[must_use]
    pub const fn with_yes(mut self, yes: bool) -> Self {
        self.yes = yes;
        self
    }

    /// Set whether to force reindex.
    #[must_use]
    pub const fn with_reindex(mut self, reindex: bool) -> Self {
        self.reindex = reindex;
        self
    }

    /// Set the filter expression.
    #[must_use]
    pub fn with_filter(mut self, filter: Option<String>) -> Self {
        self.filter = filter;
        self
    }

    /// Set whether to disable filters.
    #[must_use]
    pub const fn with_no_filter(mut self, no_filter: bool) -> Self {
        self.no_filter = no_filter;
        self
    }

    /// Set quiet mode.
    #[must_use]
    pub const fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let config = SyncConfig::default();
        assert!(!config.yes);
        assert!(!config.reindex);
        assert!(config.filter.is_none());
        assert!(!config.no_filter);
        assert!(!config.quiet);
    }

    #[test]
    fn test_new() {
        let config = SyncConfig::new();
        assert!(!config.yes);
        assert!(!config.reindex);
        assert!(config.filter.is_none());
        assert!(!config.no_filter);
        assert!(!config.quiet);
    }

    #[test]
    fn test_builder() {
        let config = SyncConfig::new()
            .with_yes(true)
            .with_reindex(true)
            .with_filter(Some("lang".to_string()))
            .with_quiet(true);

        assert!(config.yes);
        assert!(config.reindex);
        assert_eq!(config.filter, Some("lang".to_string()));
        assert!(!config.no_filter);
        assert!(config.quiet);
    }

    #[test]
    fn test_no_filter() {
        let config = SyncConfig::new().with_no_filter(true);

        assert!(config.no_filter);
        assert!(config.filter.is_none());
    }
}
