//! Display configuration for CLI output.
//!
//! This module provides [`DisplayConfig`], which bundles output formatting
//! and display parameters to reduce argument counts in execute functions.

use crate::args::ShowComponent;
use crate::output::OutputFormat;

/// Display configuration for CLI output.
///
/// Controls how results are formatted and displayed to the user.
///
/// # Examples
///
/// ```ignore
/// use blz_cli::config::DisplayConfig;
/// use blz_cli::output::OutputFormat;
///
/// let config = DisplayConfig::new(OutputFormat::Json)
///     .with_show(vec![ShowComponent::Url, ShowComponent::Lines])
///     .with_no_summary(false);
/// ```
#[derive(Debug, Clone)]
pub struct DisplayConfig {
    /// Output format (text, json, jsonl).
    pub format: OutputFormat,

    /// Additional columns to include in text output.
    pub show: Vec<ShowComponent>,

    /// Hide the summary/footer line.
    pub no_summary: bool,

    /// Show detailed timing breakdown.
    pub timing: bool,

    /// Suppress non-essential output.
    pub quiet: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            format: OutputFormat::Text,
            show: Vec::new(),
            no_summary: false,
            timing: false,
            quiet: false,
        }
    }
}

impl DisplayConfig {
    /// Create a new display configuration with the specified format.
    #[must_use]
    pub const fn new(format: OutputFormat) -> Self {
        Self {
            format,
            show: Vec::new(),
            no_summary: false,
            timing: false,
            quiet: false,
        }
    }

    /// Set the show components.
    #[must_use]
    pub fn with_show(mut self, show: Vec<ShowComponent>) -> Self {
        self.show = show;
        self
    }

    /// Set whether to hide the summary.
    #[must_use]
    pub const fn with_no_summary(mut self, no_summary: bool) -> Self {
        self.no_summary = no_summary;
        self
    }

    /// Set whether to show timing.
    #[must_use]
    pub const fn with_timing(mut self, timing: bool) -> Self {
        self.timing = timing;
        self
    }

    /// Set quiet mode.
    #[must_use]
    pub const fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// Check if output is machine-readable (JSON/JSONL).
    #[must_use]
    pub const fn is_machine_readable(&self) -> bool {
        self.format.is_machine_readable()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let config = DisplayConfig::default();
        assert_eq!(config.format, OutputFormat::Text);
        assert!(config.show.is_empty());
        assert!(!config.no_summary);
        assert!(!config.timing);
        assert!(!config.quiet);
    }

    #[test]
    fn test_new() {
        let config = DisplayConfig::new(OutputFormat::Json);
        assert_eq!(config.format, OutputFormat::Json);
    }

    #[test]
    fn test_builder() {
        let config = DisplayConfig::new(OutputFormat::Json)
            .with_show(vec![ShowComponent::Url, ShowComponent::Lines])
            .with_no_summary(true)
            .with_timing(true)
            .with_quiet(true);

        assert_eq!(config.format, OutputFormat::Json);
        assert_eq!(config.show.len(), 2);
        assert!(config.no_summary);
        assert!(config.timing);
        assert!(config.quiet);
    }

    #[test]
    fn test_is_machine_readable() {
        let config = DisplayConfig::new(OutputFormat::Text);
        assert!(!config.is_machine_readable());

        let config = DisplayConfig::new(OutputFormat::Json);
        assert!(config.is_machine_readable());

        let config = DisplayConfig::new(OutputFormat::Jsonl);
        assert!(config.is_machine_readable());
    }
}
