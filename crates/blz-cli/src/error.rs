//! CLI error handling with semantic exit codes.
//!
//! This module provides categorized errors that map to specific exit codes,
//! enabling reliable error handling in shell scripts and CI pipelines.
//!
//! # Exit Code Categories
//!
//! Exit codes follow a semantic scheme where the code indicates the type of failure:
//!
//! | Code | Category | Description |
//! |------|----------|-------------|
//! | 0 | Success | Command completed successfully |
//! | 1 | `Internal` | Unexpected/internal error |
//! | 2 | `Usage` | Invalid arguments or configuration |
//! | 3 | `NotFound` | Requested resource not found |
//! | 4 | `InvalidQuery` | Query syntax or semantic error |
//! | 5 | `Network` | Network or fetch failure |
//! | 6 | `Timeout` | Operation timed out |
//! | 7 | `Integrity` | Index or data corruption |
//!
//! # Usage
//!
//! ```bash
//! # Check for specific error types
//! blz search "query" --source missing
//! case $? in
//!     0) echo "Success" ;;
//!     3) echo "Source not found" ;;
//!     *) echo "Other error" ;;
//! esac
//! ```
//!
//! # Design
//!
//! The error system is designed to:
//! - Provide meaningful exit codes for automation
//! - Preserve error context and chains (via `anyhow`)
//! - Support both machine-readable codes and human-readable messages
//! - Be backward compatible (errors still work as regular `anyhow::Error`)

use std::fmt;
use std::process::ExitCode;

/// Semantic error category determining the exit code.
///
/// Each category maps to a specific exit code for reliable error handling
/// in shell scripts and CI pipelines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ErrorCategory {
    /// Unexpected or internal error (exit code 1).
    ///
    /// Use for programming errors, assertion failures, or errors that
    /// shouldn't happen in normal operation.
    Internal = 1,

    /// Invalid arguments or configuration (exit code 2).
    ///
    /// Use for CLI argument validation failures, invalid flag combinations,
    /// or configuration file errors.
    Usage = 2,

    /// Requested resource not found (exit code 3).
    ///
    /// Use when a source alias, file, or other named resource doesn't exist.
    NotFound = 3,

    /// Query syntax or semantic error (exit code 4).
    ///
    /// Use for malformed search queries or invalid query parameters.
    InvalidQuery = 4,

    /// Network or fetch failure (exit code 5).
    ///
    /// Use for HTTP errors, DNS failures, or connection timeouts
    /// when fetching remote content.
    Network = 5,

    /// Operation timed out (exit code 6).
    ///
    /// Use when an operation exceeds its time limit.
    Timeout = 6,

    /// Index or data corruption (exit code 7).
    ///
    /// Use when local data is corrupted, inconsistent, or unreadable.
    Integrity = 7,
}

impl ErrorCategory {
    /// Get the exit code for this category.
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        self as u8
    }

    /// Create an `ExitCode` from this category.
    #[must_use]
    pub fn as_exit_code(self) -> ExitCode {
        ExitCode::from(self.exit_code())
    }

    /// Get a short description of this error category.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Internal => "internal error",
            Self::Usage => "usage error",
            Self::NotFound => "not found",
            Self::InvalidQuery => "invalid query",
            Self::Network => "network error",
            Self::Timeout => "timeout",
            Self::Integrity => "integrity error",
        }
    }

    /// Infer the error category from an error message.
    ///
    /// This provides a heuristic-based fallback when errors aren't explicitly
    /// categorized. It examines the error message for common patterns.
    #[must_use]
    pub fn infer_from_message(msg: &str) -> Self {
        let msg_lower = msg.to_lowercase();

        // Timeout errors (check before Network so "connection timeout" is categorized correctly)
        if msg_lower.contains("timeout") || msg_lower.contains("timed out") {
            return Self::Timeout;
        }

        // Network errors
        if msg_lower.contains("network")
            || msg_lower.contains("connection")
            || msg_lower.contains("dns")
            || msg_lower.contains("http")
            || msg_lower.contains("fetch")
            || msg_lower.contains("unreachable")
        {
            return Self::Network;
        }

        // Not found errors
        if msg_lower.contains("not found")
            || msg_lower.contains("no such")
            || msg_lower.contains("does not exist")
            || msg_lower.contains("unknown source")
            || msg_lower.contains("source not found")
        {
            return Self::NotFound;
        }

        // Query errors
        if msg_lower.contains("query")
            || msg_lower.contains("invalid search")
            || msg_lower.contains("parse error")
        {
            return Self::InvalidQuery;
        }

        // Integrity errors
        if msg_lower.contains("corrupt")
            || msg_lower.contains("integrity")
            || msg_lower.contains("invalid index")
            || msg_lower.contains("checksum")
        {
            return Self::Integrity;
        }

        // Usage errors
        if msg_lower.contains("invalid argument")
            || msg_lower.contains("missing required")
            || msg_lower.contains("invalid value")
            || msg_lower.contains("cannot use")
        {
            return Self::Usage;
        }

        // Default to internal for unknown errors
        Self::Internal
    }
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// A CLI error with a semantic category for exit code mapping.
///
/// Wraps an `anyhow::Error` with an `ErrorCategory` to enable proper
/// exit codes while preserving full error context and chains.
///
/// # Creating Categorized Errors
///
/// ```rust,ignore
/// use blz_cli::error::{CliError, ErrorCategory};
/// use anyhow::anyhow;
///
/// // Explicit category
/// let err = CliError::new(
///     ErrorCategory::NotFound,
///     anyhow!("Source 'react' not found"),
/// );
///
/// // Using convenience constructors
/// let err = CliError::not_found("Source 'react' not found");
/// let err = CliError::usage("Invalid flag combination");
/// ```
#[derive(Debug)]
pub struct CliError {
    /// The semantic category of this error.
    pub category: ErrorCategory,
    /// The underlying error with full context.
    pub source: anyhow::Error,
}

impl CliError {
    /// Create a new CLI error with explicit category.
    pub fn new(category: ErrorCategory, source: impl Into<anyhow::Error>) -> Self {
        Self {
            category,
            source: source.into(),
        }
    }

    /// Create a CLI error, inferring the category from the error message.
    pub fn inferred(source: impl Into<anyhow::Error>) -> Self {
        let source = source.into();
        let category = ErrorCategory::infer_from_message(&source.to_string());
        Self { category, source }
    }

    /// Create an internal error.
    pub fn internal(source: impl Into<anyhow::Error>) -> Self {
        Self::new(ErrorCategory::Internal, source)
    }

    /// Create a usage error.
    pub fn usage(source: impl Into<anyhow::Error>) -> Self {
        Self::new(ErrorCategory::Usage, source)
    }

    /// Create a not-found error.
    pub fn not_found(source: impl Into<anyhow::Error>) -> Self {
        Self::new(ErrorCategory::NotFound, source)
    }

    /// Create an invalid-query error.
    pub fn invalid_query(source: impl Into<anyhow::Error>) -> Self {
        Self::new(ErrorCategory::InvalidQuery, source)
    }

    /// Create a network error.
    pub fn network(source: impl Into<anyhow::Error>) -> Self {
        Self::new(ErrorCategory::Network, source)
    }

    /// Create a timeout error.
    pub fn timeout(source: impl Into<anyhow::Error>) -> Self {
        Self::new(ErrorCategory::Timeout, source)
    }

    /// Create an integrity error.
    pub fn integrity(source: impl Into<anyhow::Error>) -> Self {
        Self::new(ErrorCategory::Integrity, source)
    }

    /// Get the exit code for this error.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        self.category.exit_code()
    }

    /// Create an `ExitCode` from this error.
    #[must_use]
    pub fn as_exit_code(&self) -> ExitCode {
        self.category.as_exit_code()
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.source)
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Extension trait for converting errors to `CliError` with category inference.
pub trait IntoCliError {
    /// Convert to a `CliError`, inferring the category from the error message.
    fn into_cli_error(self) -> CliError;

    /// Convert to a `CliError` with an explicit category.
    fn with_category(self, category: ErrorCategory) -> CliError;
}

impl<E: Into<anyhow::Error>> IntoCliError for E {
    fn into_cli_error(self) -> CliError {
        CliError::inferred(self)
    }

    fn with_category(self, category: ErrorCategory) -> CliError {
        CliError::new(category, self)
    }
}

/// Determine the exit code from an `anyhow::Error`.
///
/// If the error is a `CliError`, returns its category's exit code.
/// Otherwise, infers the category from the error message.
#[must_use]
pub fn exit_code_from_error(err: &anyhow::Error) -> u8 {
    // Check if this is already a CliError
    if let Some(cli_err) = err.downcast_ref::<CliError>() {
        return cli_err.exit_code();
    }

    // Infer from the error message
    ErrorCategory::infer_from_message(&err.to_string()).exit_code()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    mod error_category {
        use super::*;

        #[test]
        fn test_exit_codes() {
            assert_eq!(ErrorCategory::Internal.exit_code(), 1);
            assert_eq!(ErrorCategory::Usage.exit_code(), 2);
            assert_eq!(ErrorCategory::NotFound.exit_code(), 3);
            assert_eq!(ErrorCategory::InvalidQuery.exit_code(), 4);
            assert_eq!(ErrorCategory::Network.exit_code(), 5);
            assert_eq!(ErrorCategory::Timeout.exit_code(), 6);
            assert_eq!(ErrorCategory::Integrity.exit_code(), 7);
        }

        #[test]
        fn test_infer_network() {
            assert_eq!(
                ErrorCategory::infer_from_message("Connection refused"),
                ErrorCategory::Network
            );
            assert_eq!(
                ErrorCategory::infer_from_message("HTTP 500 error"),
                ErrorCategory::Network
            );
            assert_eq!(
                ErrorCategory::infer_from_message("Failed to fetch URL"),
                ErrorCategory::Network
            );
        }

        #[test]
        fn test_infer_timeout() {
            assert_eq!(
                ErrorCategory::infer_from_message("Operation timed out"),
                ErrorCategory::Timeout
            );
            assert_eq!(
                ErrorCategory::infer_from_message("Request timeout after 30s"),
                ErrorCategory::Timeout
            );
        }

        #[test]
        fn test_infer_not_found() {
            assert_eq!(
                ErrorCategory::infer_from_message("Source not found: react"),
                ErrorCategory::NotFound
            );
            assert_eq!(
                ErrorCategory::infer_from_message("No such file or directory"),
                ErrorCategory::NotFound
            );
            assert_eq!(
                ErrorCategory::infer_from_message("Unknown source 'test'"),
                ErrorCategory::NotFound
            );
        }

        #[test]
        fn test_infer_query() {
            assert_eq!(
                ErrorCategory::infer_from_message("Invalid query syntax"),
                ErrorCategory::InvalidQuery
            );
            assert_eq!(
                ErrorCategory::infer_from_message("Query parse error at position 5"),
                ErrorCategory::InvalidQuery
            );
        }

        #[test]
        fn test_infer_integrity() {
            assert_eq!(
                ErrorCategory::infer_from_message("Index corrupted"),
                ErrorCategory::Integrity
            );
            assert_eq!(
                ErrorCategory::infer_from_message("Checksum mismatch"),
                ErrorCategory::Integrity
            );
        }

        #[test]
        fn test_infer_usage() {
            assert_eq!(
                ErrorCategory::infer_from_message("Invalid argument: --foo"),
                ErrorCategory::Usage
            );
            assert_eq!(
                ErrorCategory::infer_from_message("Missing required field"),
                ErrorCategory::Usage
            );
        }

        #[test]
        fn test_infer_default() {
            assert_eq!(
                ErrorCategory::infer_from_message("Something went wrong"),
                ErrorCategory::Internal
            );
        }
    }

    mod cli_error {
        use super::*;

        #[test]
        fn test_new() {
            let err = CliError::new(ErrorCategory::NotFound, anyhow!("Source not found"));
            assert_eq!(err.category, ErrorCategory::NotFound);
            assert_eq!(err.exit_code(), 3);
        }

        #[test]
        fn test_inferred() {
            let err = CliError::inferred(anyhow!("Connection refused"));
            assert_eq!(err.category, ErrorCategory::Network);
        }

        #[test]
        fn test_convenience_constructors() {
            assert_eq!(
                CliError::internal(anyhow!("err")).category,
                ErrorCategory::Internal
            );
            assert_eq!(
                CliError::usage(anyhow!("err")).category,
                ErrorCategory::Usage
            );
            assert_eq!(
                CliError::not_found(anyhow!("err")).category,
                ErrorCategory::NotFound
            );
            assert_eq!(
                CliError::invalid_query(anyhow!("err")).category,
                ErrorCategory::InvalidQuery
            );
            assert_eq!(
                CliError::network(anyhow!("err")).category,
                ErrorCategory::Network
            );
            assert_eq!(
                CliError::timeout(anyhow!("err")).category,
                ErrorCategory::Timeout
            );
            assert_eq!(
                CliError::integrity(anyhow!("err")).category,
                ErrorCategory::Integrity
            );
        }

        #[test]
        fn test_display() {
            let err = CliError::not_found(anyhow!("Source 'react' not found"));
            assert_eq!(err.to_string(), "Source 'react' not found");
        }
    }

    mod exit_code_from_error {
        use super::*;

        #[test]
        fn test_cli_error() {
            let cli_err = CliError::not_found(anyhow!("Not found"));
            let err: anyhow::Error = cli_err.into();
            assert_eq!(exit_code_from_error(&err), 3);
        }

        #[test]
        fn test_regular_error() {
            // Use "Operation timed out" instead of "Connection timeout"
            // because the latter contains "connection" which matches Network first
            let err = anyhow!("Operation timed out");
            assert_eq!(exit_code_from_error(&err), 6);
        }
    }

    mod into_cli_error {
        use super::*;

        #[test]
        fn test_into_cli_error() {
            let err = anyhow!("Source not found").into_cli_error();
            assert_eq!(err.category, ErrorCategory::NotFound);
        }

        #[test]
        fn test_with_category() {
            let err = anyhow!("Something failed").with_category(ErrorCategory::Timeout);
            assert_eq!(err.category, ErrorCategory::Timeout);
        }
    }
}
