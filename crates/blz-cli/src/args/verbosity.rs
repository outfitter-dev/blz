//! Verbosity level configuration for CLI output.
//!
//! This module provides a structured way to handle output verbosity levels,
//! replacing individual boolean flags (`quiet`, `verbose`, `debug`) with a
//! single, well-defined enum.
//!
//! # Verbosity Levels
//!
//! From quietest to most verbose:
//!
//! | Level | Description | Use Case |
//! |-------|-------------|----------|
//! | `Quiet` | Errors only | Scripts, CI |
//! | `Normal` | Standard output | Interactive use |
//! | `Verbose` | Additional details | Debugging |
//! | `Debug` | Full diagnostics | Development |
//!
//! **Note:** Errors are always shown regardless of verbosity level.
//!
//! # Examples
//!
//! ```
//! use blz_cli::args::Verbosity;
//!
//! let level = Verbosity::default();
//! assert_eq!(level, Verbosity::Normal);
//!
//! // Check if messages should be shown
//! assert!(level.show_info());
//! assert!(!level.show_debug());
//! ```

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Verbosity level for CLI output.
///
/// This enum provides a structured way to control output verbosity,
/// replacing individual boolean flags with a single, clear setting.
///
/// # Ordering
///
/// Levels are ordered from quietest to most verbose:
/// `Quiet < Normal < Verbose < Debug`
///
/// This allows easy comparison:
/// ```
/// use blz_cli::args::Verbosity;
///
/// assert!(Verbosity::Quiet < Verbosity::Normal);
/// assert!(Verbosity::Verbose < Verbosity::Debug);
/// ```
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum Verbosity {
    /// Suppress all output except errors.
    ///
    /// Useful for scripts and CI where only exit codes matter.
    Quiet,

    /// Standard output level (default).
    ///
    /// Shows results, warnings, and important information.
    #[default]
    Normal,

    /// Show additional details.
    ///
    /// Includes timing information, progress indicators, and
    /// contextual details that help understand what's happening.
    Verbose,

    /// Full diagnostic output.
    ///
    /// Shows all available information including performance metrics,
    /// internal state, and debug-level logging. Primarily for
    /// development and troubleshooting.
    Debug,
}

impl Verbosity {
    /// Create a Verbosity from individual boolean flags.
    ///
    /// This provides backward compatibility with the legacy flag pattern.
    /// Priority order: debug > verbose > quiet > normal
    ///
    /// # Examples
    ///
    /// ```
    /// use blz_cli::args::Verbosity;
    ///
    /// // All false = Normal
    /// assert_eq!(Verbosity::from_flags(false, false, false), Verbosity::Normal);
    ///
    /// // Quiet takes precedence when alone
    /// assert_eq!(Verbosity::from_flags(true, false, false), Verbosity::Quiet);
    ///
    /// // Verbose takes precedence over quiet
    /// assert_eq!(Verbosity::from_flags(true, true, false), Verbosity::Verbose);
    ///
    /// // Debug takes precedence over everything
    /// assert_eq!(Verbosity::from_flags(true, true, true), Verbosity::Debug);
    /// ```
    #[must_use]
    pub const fn from_flags(quiet: bool, verbose: bool, debug: bool) -> Self {
        if debug {
            Self::Debug
        } else if verbose {
            Self::Verbose
        } else if quiet {
            Self::Quiet
        } else {
            Self::Normal
        }
    }

    /// Check if warning messages should be shown.
    ///
    /// Warnings are shown at Normal level and above.
    #[must_use]
    pub const fn show_warnings(self) -> bool {
        matches!(self, Self::Normal | Self::Verbose | Self::Debug)
    }

    /// Check if informational messages should be shown.
    ///
    /// Info messages are shown at Normal level and above.
    #[must_use]
    pub const fn show_info(self) -> bool {
        matches!(self, Self::Normal | Self::Verbose | Self::Debug)
    }

    /// Check if verbose output should be shown.
    ///
    /// Verbose output is shown at Verbose level and above.
    #[must_use]
    pub const fn show_verbose(self) -> bool {
        matches!(self, Self::Verbose | Self::Debug)
    }

    /// Check if debug output should be shown.
    ///
    /// Debug output is only shown at Debug level.
    #[must_use]
    pub const fn show_debug(self) -> bool {
        matches!(self, Self::Debug)
    }

    /// Check if output should be suppressed (quiet mode).
    #[must_use]
    pub const fn is_quiet(self) -> bool {
        matches!(self, Self::Quiet)
    }

    /// Check if verbose mode is enabled.
    #[must_use]
    pub const fn is_verbose(self) -> bool {
        matches!(self, Self::Verbose | Self::Debug)
    }

    /// Check if debug mode is enabled.
    #[must_use]
    pub const fn is_debug(self) -> bool {
        matches!(self, Self::Debug)
    }
}

impl std::fmt::Display for Verbosity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Quiet => write!(f, "quiet"),
            Self::Normal => write!(f, "normal"),
            Self::Verbose => write!(f, "verbose"),
            Self::Debug => write!(f, "debug"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_normal() {
        assert_eq!(Verbosity::default(), Verbosity::Normal);
    }

    #[test]
    fn test_ordering() {
        assert!(Verbosity::Quiet < Verbosity::Normal);
        assert!(Verbosity::Normal < Verbosity::Verbose);
        assert!(Verbosity::Verbose < Verbosity::Debug);
    }

    #[test]
    fn test_from_flags_priority() {
        // Debug takes highest priority
        assert_eq!(Verbosity::from_flags(true, true, true), Verbosity::Debug);
        assert_eq!(Verbosity::from_flags(false, false, true), Verbosity::Debug);

        // Verbose takes next priority
        assert_eq!(Verbosity::from_flags(true, true, false), Verbosity::Verbose);
        assert_eq!(
            Verbosity::from_flags(false, true, false),
            Verbosity::Verbose
        );

        // Quiet takes next priority
        assert_eq!(Verbosity::from_flags(true, false, false), Verbosity::Quiet);

        // Default to Normal
        assert_eq!(
            Verbosity::from_flags(false, false, false),
            Verbosity::Normal
        );
    }

    #[test]
    fn test_show_methods() {
        // Quiet - only errors (not controlled by these methods)
        assert!(!Verbosity::Quiet.show_warnings());
        assert!(!Verbosity::Quiet.show_info());
        assert!(!Verbosity::Quiet.show_verbose());
        assert!(!Verbosity::Quiet.show_debug());

        // Normal - warnings and info
        assert!(Verbosity::Normal.show_warnings());
        assert!(Verbosity::Normal.show_info());
        assert!(!Verbosity::Normal.show_verbose());
        assert!(!Verbosity::Normal.show_debug());

        // Verbose - adds verbose output
        assert!(Verbosity::Verbose.show_warnings());
        assert!(Verbosity::Verbose.show_info());
        assert!(Verbosity::Verbose.show_verbose());
        assert!(!Verbosity::Verbose.show_debug());

        // Debug - shows everything
        assert!(Verbosity::Debug.show_warnings());
        assert!(Verbosity::Debug.show_info());
        assert!(Verbosity::Debug.show_verbose());
        assert!(Verbosity::Debug.show_debug());
    }

    #[test]
    fn test_is_methods() {
        assert!(Verbosity::Quiet.is_quiet());
        assert!(!Verbosity::Normal.is_quiet());

        assert!(!Verbosity::Quiet.is_verbose());
        assert!(Verbosity::Verbose.is_verbose());
        assert!(Verbosity::Debug.is_verbose());

        assert!(!Verbosity::Verbose.is_debug());
        assert!(Verbosity::Debug.is_debug());
    }

    #[test]
    fn test_display() {
        assert_eq!(Verbosity::Quiet.to_string(), "quiet");
        assert_eq!(Verbosity::Normal.to_string(), "normal");
        assert_eq!(Verbosity::Verbose.to_string(), "verbose");
        assert_eq!(Verbosity::Debug.to_string(), "debug");
    }
}
