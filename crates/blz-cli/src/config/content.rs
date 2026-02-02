//! Content retrieval configuration.
//!
//! This module provides [`ContentConfig`], which bundles content retrieval
//! parameters like context lines and block expansion.

use crate::args::ContextMode;

/// Content retrieval configuration.
///
/// Controls how content is retrieved and expanded with context.
///
/// # Examples
///
/// ```ignore
/// use blz_cli::config::ContentConfig;
/// use blz_cli::args::ContextMode;
///
/// let config = ContentConfig::new()
///     .with_context(Some(ContextMode::Symmetric(5)))
///     .with_max_lines(Some(100));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ContentConfig {
    /// Context mode for result expansion.
    pub context: Option<ContextMode>,

    /// Maximum lines for block expansion.
    pub max_lines: Option<usize>,

    /// Copy results to clipboard using OSC 52.
    pub copy: bool,

    /// Legacy block expansion mode (--block flag).
    pub block: bool,
}

impl ContentConfig {
    /// Create a new content configuration with default values.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            context: None,
            max_lines: None,
            copy: false,
            block: false,
        }
    }

    /// Set the context mode.
    #[must_use]
    pub const fn with_context(mut self, context: Option<ContextMode>) -> Self {
        self.context = context;
        self
    }

    /// Set the maximum lines for block expansion.
    #[must_use]
    pub const fn with_max_lines(mut self, max_lines: Option<usize>) -> Self {
        self.max_lines = max_lines;
        self
    }

    /// Set whether to copy results to clipboard.
    #[must_use]
    pub const fn with_copy(mut self, copy: bool) -> Self {
        self.copy = copy;
        self
    }

    /// Set the legacy block expansion mode.
    #[must_use]
    pub const fn with_block(mut self, block: bool) -> Self {
        self.block = block;
        self
    }

    /// Check if any context is requested.
    #[must_use]
    pub const fn has_context(&self) -> bool {
        self.context.is_some() || self.block
    }

    /// Get the before/after context lines, accounting for block mode.
    ///
    /// Returns `(before, after, is_block)` tuple.
    #[must_use]
    pub const fn resolve_context(&self) -> (usize, usize, bool) {
        match &self.context {
            Some(ContextMode::All) => (0, 0, true),
            Some(ContextMode::Symmetric(n)) => (*n, *n, false),
            Some(ContextMode::Asymmetric { before, after }) => (*before, *after, false),
            None => (0, 0, self.block),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let config = ContentConfig::default();
        assert!(config.context.is_none());
        assert!(config.max_lines.is_none());
        assert!(!config.copy);
        assert!(!config.block);
    }

    #[test]
    fn test_builder() {
        let config = ContentConfig::new()
            .with_context(Some(ContextMode::Symmetric(5)))
            .with_max_lines(Some(100))
            .with_copy(true)
            .with_block(false);

        assert_eq!(config.context, Some(ContextMode::Symmetric(5)));
        assert_eq!(config.max_lines, Some(100));
        assert!(config.copy);
        assert!(!config.block);
    }

    #[test]
    fn test_has_context() {
        let config = ContentConfig::default();
        assert!(!config.has_context());

        let config = ContentConfig::new().with_context(Some(ContextMode::Symmetric(5)));
        assert!(config.has_context());

        let config = ContentConfig::new().with_block(true);
        assert!(config.has_context());
    }

    #[test]
    fn test_resolve_context() {
        // No context
        let config = ContentConfig::default();
        assert_eq!(config.resolve_context(), (0, 0, false));

        // Symmetric context
        let config = ContentConfig::new().with_context(Some(ContextMode::Symmetric(5)));
        assert_eq!(config.resolve_context(), (5, 5, false));

        // Asymmetric context
        let config = ContentConfig::new().with_context(Some(ContextMode::Asymmetric {
            before: 3,
            after: 7,
        }));
        assert_eq!(config.resolve_context(), (3, 7, false));

        // All context (block mode)
        let config = ContentConfig::new().with_context(Some(ContextMode::All));
        assert_eq!(config.resolve_context(), (0, 0, true));

        // Legacy block flag
        let config = ContentConfig::new().with_block(true);
        assert_eq!(config.resolve_context(), (0, 0, true));
    }
}
