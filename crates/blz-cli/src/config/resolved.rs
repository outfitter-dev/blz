//! Resolved execution configuration for CLI commands.
//!
//! This module provides [`ExecutionConfig`], a struct that bundles common
//! execution parameters that are shared across most commands.

use blz_core::PerformanceMetrics;

use crate::args::{OutputFormat, Verbosity};

/// Common execution configuration shared across CLI commands.
///
/// This struct bundles the runtime configuration that most commands need,
/// reducing parameter explosion in execute functions.
///
/// # Components
///
/// - **verbosity**: Controls output level (quiet/normal/verbose/debug)
/// - **format**: Resolved output format (text/json/jsonl/raw)
/// - **metrics**: Performance tracking for timing and resource usage
///
/// # Usage
///
/// Create from CLI args and pass to command execute functions:
///
/// ```ignore
/// let config = ExecutionConfig::new(
///     Verbosity::from_flags(cli.quiet, cli.verbose, cli.debug),
///     format.resolve(),
///     PerformanceMetrics::default(),
/// );
///
/// commands::find::execute(&find_args, &config).await?;
/// ```
///
/// # Design Notes
///
/// This struct intentionally excludes:
/// - Command-specific parameters (query, limit, etc.)
/// - Mutable state (`CliPreferences`, `ResourceMonitor`)
///
/// Mutable state should be passed separately as needed, as bundling
/// mutable references in a config struct creates lifetime complications.
#[derive(Clone, Debug)]
pub struct ExecutionConfig {
    /// Output verbosity level.
    pub verbosity: Verbosity,

    /// Resolved output format.
    pub format: OutputFormat,

    /// Performance metrics tracker.
    pub metrics: PerformanceMetrics,
}

impl ExecutionConfig {
    /// Create a new execution configuration.
    #[must_use]
    pub const fn new(
        verbosity: Verbosity,
        format: OutputFormat,
        metrics: PerformanceMetrics,
    ) -> Self {
        Self {
            verbosity,
            format,
            metrics,
        }
    }

    /// Check if output should be suppressed (quiet mode).
    #[must_use]
    pub const fn is_quiet(&self) -> bool {
        matches!(self.verbosity, Verbosity::Quiet)
    }

    /// Check if verbose output is enabled.
    #[must_use]
    pub const fn is_verbose(&self) -> bool {
        matches!(self.verbosity, Verbosity::Verbose | Verbosity::Debug)
    }

    /// Check if debug output is enabled.
    #[must_use]
    pub const fn is_debug(&self) -> bool {
        matches!(self.verbosity, Verbosity::Debug)
    }

    /// Check if output format is machine-readable (JSON/JSONL).
    #[must_use]
    pub const fn is_machine_readable(&self) -> bool {
        self.format.is_machine_readable()
    }

    /// Create a builder for more complex configuration.
    #[must_use]
    pub fn builder() -> ExecutionConfigBuilder {
        ExecutionConfigBuilder::default()
    }

    /// Create a minimal configuration for testing or simple commands.
    #[must_use]
    pub fn minimal(format: OutputFormat) -> Self {
        Self {
            verbosity: Verbosity::Normal,
            format,
            metrics: PerformanceMetrics::default(),
        }
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            verbosity: Verbosity::Normal,
            format: OutputFormat::Text,
            metrics: PerformanceMetrics::default(),
        }
    }
}

/// Builder for [`ExecutionConfig`].
///
/// Provides a fluent interface for creating configuration with optional
/// parameters and sensible defaults.
///
/// # Examples
///
/// ```ignore
/// let config = ExecutionConfig::builder()
///     .verbosity(Verbosity::Verbose)
///     .format(OutputFormat::Json)
///     .build();
/// ```
#[derive(Default)]
pub struct ExecutionConfigBuilder {
    verbosity: Option<Verbosity>,
    format: Option<OutputFormat>,
    metrics: Option<PerformanceMetrics>,
}

impl ExecutionConfigBuilder {
    /// Set the verbosity level.
    #[must_use]
    pub const fn verbosity(mut self, verbosity: Verbosity) -> Self {
        self.verbosity = Some(verbosity);
        self
    }

    /// Set the output format.
    #[must_use]
    pub const fn format(mut self, format: OutputFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set the performance metrics.
    #[must_use]
    pub fn metrics(mut self, metrics: PerformanceMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Build the configuration.
    #[must_use]
    pub fn build(self) -> ExecutionConfig {
        ExecutionConfig {
            verbosity: self.verbosity.unwrap_or(Verbosity::Normal),
            format: self.format.unwrap_or(OutputFormat::Text),
            metrics: self.metrics.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let config = ExecutionConfig::new(
            Verbosity::Verbose,
            OutputFormat::Json,
            PerformanceMetrics::default(),
        );

        assert_eq!(config.verbosity, Verbosity::Verbose);
        assert_eq!(config.format, OutputFormat::Json);
    }

    #[test]
    fn test_minimal() {
        let config = ExecutionConfig::minimal(OutputFormat::Json);

        assert_eq!(config.verbosity, Verbosity::Normal);
        assert_eq!(config.format, OutputFormat::Json);
    }

    #[test]
    fn test_default() {
        let config = ExecutionConfig::default();

        assert_eq!(config.verbosity, Verbosity::Normal);
        assert_eq!(config.format, OutputFormat::Text);
    }

    #[test]
    fn test_is_quiet() {
        let quiet = ExecutionConfig::new(
            Verbosity::Quiet,
            OutputFormat::Text,
            PerformanceMetrics::default(),
        );
        let normal = ExecutionConfig::default();

        assert!(quiet.is_quiet());
        assert!(!normal.is_quiet());
    }

    #[test]
    fn test_is_verbose() {
        let verbose = ExecutionConfig::new(
            Verbosity::Verbose,
            OutputFormat::Text,
            PerformanceMetrics::default(),
        );
        let debug = ExecutionConfig::new(
            Verbosity::Debug,
            OutputFormat::Text,
            PerformanceMetrics::default(),
        );
        let normal = ExecutionConfig::default();

        assert!(verbose.is_verbose());
        assert!(debug.is_verbose());
        assert!(!normal.is_verbose());
    }

    #[test]
    fn test_is_debug() {
        let debug = ExecutionConfig::new(
            Verbosity::Debug,
            OutputFormat::Text,
            PerformanceMetrics::default(),
        );
        let verbose = ExecutionConfig::new(
            Verbosity::Verbose,
            OutputFormat::Text,
            PerformanceMetrics::default(),
        );

        assert!(debug.is_debug());
        assert!(!verbose.is_debug());
    }

    #[test]
    fn test_is_machine_readable() {
        let json = ExecutionConfig::minimal(OutputFormat::Json);
        let jsonl = ExecutionConfig::minimal(OutputFormat::Jsonl);
        let text = ExecutionConfig::minimal(OutputFormat::Text);

        assert!(json.is_machine_readable());
        assert!(jsonl.is_machine_readable());
        assert!(!text.is_machine_readable());
    }

    #[test]
    fn test_builder() {
        let config = ExecutionConfig::builder()
            .verbosity(Verbosity::Verbose)
            .format(OutputFormat::Json)
            .build();

        assert_eq!(config.verbosity, Verbosity::Verbose);
        assert_eq!(config.format, OutputFormat::Json);
    }

    #[test]
    fn test_builder_defaults() {
        let config = ExecutionConfig::builder().build();

        assert_eq!(config.verbosity, Verbosity::Normal);
        assert_eq!(config.format, OutputFormat::Text);
    }
}
