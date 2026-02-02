//! Snippet display configuration.
//!
//! This module provides [`SnippetConfig`], which bundles snippet rendering
//! parameters to reduce argument counts in execute functions.

/// Snippet display configuration.
///
/// Controls how search result snippets are rendered and truncated.
///
/// # Examples
///
/// ```ignore
/// use blz_cli::config::SnippetConfig;
///
/// let config = SnippetConfig::new()
///     .with_max_chars(300)
///     .with_score_precision(Some(2));
/// ```
#[derive(Debug, Clone)]
pub struct SnippetConfig {
    /// Maximum snippet lines to display around a hit (1-10).
    pub lines: u8,

    /// Maximum total characters in snippet.
    pub max_chars: usize,

    /// Number of decimal places for score display (0-4).
    pub score_precision: Option<u8>,
}

impl Default for SnippetConfig {
    fn default() -> Self {
        Self {
            lines: 3,
            max_chars: 200,
            score_precision: None,
        }
    }
}

impl SnippetConfig {
    /// Create a new snippet configuration with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of snippet lines.
    #[must_use]
    pub const fn with_lines(mut self, lines: u8) -> Self {
        self.lines = lines;
        self
    }

    /// Set the maximum characters in snippet.
    #[must_use]
    pub const fn with_max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars;
        self
    }

    /// Set the score precision.
    #[must_use]
    pub const fn with_score_precision(mut self, precision: Option<u8>) -> Self {
        self.score_precision = precision;
        self
    }

    /// Get the effective score precision (default: 1).
    #[must_use]
    pub const fn effective_score_precision(&self) -> u8 {
        match self.score_precision {
            Some(p) => p,
            None => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let config = SnippetConfig::default();
        assert_eq!(config.lines, 3);
        assert_eq!(config.max_chars, 200);
        assert!(config.score_precision.is_none());
    }

    #[test]
    fn test_builder() {
        let config = SnippetConfig::new()
            .with_lines(5)
            .with_max_chars(500)
            .with_score_precision(Some(2));

        assert_eq!(config.lines, 5);
        assert_eq!(config.max_chars, 500);
        assert_eq!(config.score_precision, Some(2));
    }

    #[test]
    fn test_effective_score_precision() {
        let config = SnippetConfig::default();
        assert_eq!(config.effective_score_precision(), 1);

        let config = SnippetConfig::new().with_score_precision(Some(3));
        assert_eq!(config.effective_score_precision(), 3);
    }
}
