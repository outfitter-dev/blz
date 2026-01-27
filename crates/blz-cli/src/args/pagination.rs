//! Pagination argument groups for CLI commands.
//!
//! This module provides reusable pagination arguments that can be composed
//! across multiple commands using clap's `#[command(flatten)]` attribute.
//!
//! # Examples
//!
//! ```
//! use blz_cli::args::PaginationArgs;
//! use clap::Parser;
//!
//! #[derive(Parser)]
//! struct MyCommand {
//!     #[command(flatten)]
//!     pagination: PaginationArgs,
//! }
//! ```

use clap::Args;
use serde::{Deserialize, Serialize};

/// Validates that a limit value is at least 1.
fn validate_limit(s: &str) -> Result<usize, String> {
    let value: usize = s
        .parse()
        .map_err(|_| format!("'{s}' is not a valid number"))?;
    if value == 0 {
        Err("limit must be at least 1".to_string())
    } else {
        Ok(value)
    }
}

/// Shared pagination arguments for commands that support limiting results.
///
/// This group provides consistent limit/offset behavior across commands
/// that return multiple results (search, list, history, etc.).
///
/// # Usage
///
/// Flatten into command structs:
///
/// ```ignore
/// #[derive(Args)]
/// struct SearchArgs {
///     #[command(flatten)]
///     pagination: PaginationArgs,
///     // ... other args
/// }
/// ```
///
/// # Examples
///
/// ```bash
/// blz search "async" --limit 10
/// blz search "async" --limit 10 --offset 20
/// blz list --limit 5
/// ```
#[derive(Args, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginationArgs {
    /// Maximum number of results to return
    ///
    /// Limits the output to the specified count. Must be at least 1.
    #[arg(
        short = 'l',
        long,
        value_name = "COUNT",
        value_parser = validate_limit,
        display_order = 50
    )]
    pub limit: Option<usize>,

    /// Number of results to skip before returning
    ///
    /// Used for paginating through large result sets.
    #[arg(long, value_name = "COUNT", display_order = 51)]
    pub offset: Option<usize>,
}

impl PaginationArgs {
    /// Create pagination args with a specific limit.
    #[must_use]
    pub const fn with_limit(limit: usize) -> Self {
        Self {
            limit: Some(limit),
            offset: None,
        }
    }

    /// Create pagination args with limit and offset.
    #[must_use]
    pub const fn with_limit_and_offset(limit: usize, offset: usize) -> Self {
        Self {
            limit: Some(limit),
            offset: Some(offset),
        }
    }

    /// Get the effective limit, falling back to a default if not specified.
    #[must_use]
    pub const fn limit_or(&self, default: usize) -> usize {
        match self.limit {
            Some(limit) => limit,
            None => default,
        }
    }

    /// Get the effective offset, defaulting to 0 if not specified.
    #[must_use]
    pub const fn offset_or_default(&self) -> usize {
        match self.offset {
            Some(offset) => offset,
            None => 0,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_limit_valid() {
        assert_eq!(validate_limit("1"), Ok(1));
        assert_eq!(validate_limit("100"), Ok(100));
        assert_eq!(validate_limit("999999"), Ok(999_999));
    }

    #[test]
    fn test_validate_limit_zero() {
        assert!(validate_limit("0").is_err());
        assert!(validate_limit("0").unwrap_err().contains("at least 1"));
    }

    #[test]
    fn test_validate_limit_invalid() {
        assert!(validate_limit("abc").is_err());
        assert!(validate_limit("-1").is_err());
        assert!(validate_limit("1.5").is_err());
    }

    #[test]
    fn test_default() {
        let args = PaginationArgs::default();
        assert_eq!(args.limit, None);
        assert_eq!(args.offset, None);
    }

    #[test]
    fn test_with_limit() {
        let args = PaginationArgs::with_limit(10);
        assert_eq!(args.limit, Some(10));
        assert_eq!(args.offset, None);
    }

    #[test]
    fn test_with_limit_and_offset() {
        let args = PaginationArgs::with_limit_and_offset(10, 20);
        assert_eq!(args.limit, Some(10));
        assert_eq!(args.offset, Some(20));
    }

    #[test]
    fn test_limit_or() {
        let args = PaginationArgs::default();
        assert_eq!(args.limit_or(50), 50);

        let args = PaginationArgs::with_limit(10);
        assert_eq!(args.limit_or(50), 10);
    }

    #[test]
    fn test_offset_or_default() {
        let args = PaginationArgs::default();
        assert_eq!(args.offset_or_default(), 0);

        let args = PaginationArgs::with_limit_and_offset(10, 20);
        assert_eq!(args.offset_or_default(), 20);
    }
}
