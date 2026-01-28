//! Error types and handling for blz-core operations.
//!
//! This module provides a comprehensive error type that covers all possible failures
//! in the blz cache system. Errors are categorized for easier handling and include
//! context about recoverability for retry logic.
//!
//! ## Error Categories
//!
//! Errors are organized into logical categories:
//!
//! - **I/O Errors**: File system operations, disk access
//! - **Network Errors**: HTTP requests, connectivity issues  
//! - **Parse Errors**: Markdown parsing, TOML/JSON deserialization
//! - **Index Errors**: Search index operations
//! - **Storage Errors**: Cache storage operations
//! - **Configuration Errors**: Invalid settings or config files
//! - **Resource Errors**: Memory limits, timeouts, quotas
//!
//! ## Recovery Hints
//!
//! Errors include information about whether they might be recoverable through retries:
//!
//! ```rust
//! use blz_core::{Error, Result, MarkdownParser};
//!
//! fn handle_operation() -> Result<()> {
//!     match perform_operation() {
//!         Err(e) if e.is_recoverable() => {
//!             println!("Temporary failure, retrying...");
//!             // Implement retry logic
//!         }
//!         Err(e) => {
//!             println!("Permanent failure: {}", e);
//!             println!("Category: {}", e.category());
//!         }
//!         Ok(()) => println!("Success"),
//!     }
//!     Ok(())
//! }
//!
//! fn perform_operation() -> Result<()> { Ok(()) }
//! ```

use thiserror::Error;

/// The main error type for blz-core operations.
///
/// All public functions in blz-core return `Result<T, Error>` for consistent error handling.
/// The error type includes automatic conversion from common standard library errors and
/// provides additional metadata for error handling logic.
///
/// ## Error Source Chain
///
/// Errors maintain the full error chain through the `source()` method, allowing
/// for detailed error inspection and debugging.
///
/// ## Display vs Debug
///
/// - `Display` provides user-friendly error messages
/// - `Debug` includes full error details and source chain information
#[derive(Error, Debug)]
pub enum Error {
    /// I/O operation failed.
    ///
    /// Covers file system operations like reading/writing files, creating directories,
    /// checking file permissions, etc. The underlying `std::io::Error` is preserved
    /// to maintain detailed error information.
    ///
    /// ## Recoverability
    ///
    /// Some I/O errors are recoverable (timeouts, interruptions), while others
    /// are permanent (permission denied, file not found).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Network operation failed.
    ///
    /// Covers HTTP requests for fetching llms.txt files, checking `ETags`,
    /// and other network operations. The underlying `reqwest::Error` is preserved
    /// for detailed connection information.
    ///
    /// ## Recoverability
    ///
    /// Connection and timeout errors are typically recoverable, while
    /// authentication and malformed URL errors are permanent.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Parsing operation failed.
    ///
    /// Occurs when markdown content cannot be parsed, TOML/JSON deserialization
    /// fails, or content doesn't match expected format.
    ///
    /// ## Common Causes
    ///
    /// - Malformed markdown syntax
    /// - Invalid TOML configuration
    /// - Unexpected content structure
    /// - Character encoding issues
    #[error("Parse error: {0}")]
    Parse(String),

    /// Search index operation failed.
    ///
    /// Covers failures in creating, updating, or querying the search index.
    /// This includes Tantivy-related errors and index corruption.
    ///
    /// ## Common Causes
    ///
    /// - Index corruption
    /// - Disk space exhaustion during indexing
    /// - Invalid search queries
    /// - Schema version mismatches
    #[error("Index error: {0}")]
    Index(String),

    /// Storage operation failed.
    ///
    /// Covers cache storage operations beyond basic file I/O, such as
    /// managing archived versions, checksum validation, and cache consistency.
    ///
    /// ## Common Causes
    ///
    /// - Cache corruption
    /// - Concurrent access conflicts
    /// - Checksum mismatches
    /// - Archive management failures
    #[error("Storage error: {0}")]
    Storage(String),

    /// Configuration is invalid or inaccessible.
    ///
    /// Occurs when configuration files are malformed, contain invalid values,
    /// or cannot be accessed due to permissions or path issues.
    ///
    /// ## Common Causes
    ///
    /// - Invalid TOML syntax in config files
    /// - Missing required configuration fields
    /// - Configuration values outside valid ranges
    /// - Config directory creation failures
    #[error("Configuration error: {0}")]
    Config(String),

    /// Requested resource was not found.
    ///
    /// Used for missing files, non-existent sources, or requested content
    /// that doesn't exist in the cache.
    ///
    /// ## Common Causes
    ///
    /// - Requested source alias doesn't exist
    /// - File was deleted after being indexed
    /// - Cache was cleared but references remain
    #[error("Not found: {0}")]
    NotFound(String),

    /// URL is malformed or invalid.
    ///
    /// Occurs when URLs provided for llms.txt sources cannot be parsed
    /// or contain invalid characters/schemes.
    ///
    /// ## Common Causes
    ///
    /// - Malformed URLs in configuration
    /// - Unsupported URL schemes
    /// - Invalid characters in URLs
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Resource limit was exceeded.
    ///
    /// Used when operations exceed configured limits such as memory usage,
    /// file size, or processing time constraints.
    ///
    /// ## Common Causes
    ///
    /// - Document exceeds maximum size limit
    /// - Memory usage exceeds configured threshold
    /// - Too many concurrent operations
    #[error("Resource limited: {0}")]
    ResourceLimited(String),

    /// Operation timed out.
    ///
    /// Used for operations that exceed their configured timeout duration.
    /// This is typically recoverable with retry logic.
    ///
    /// ## Common Causes
    ///
    /// - Network request timeouts
    /// - Long-running parsing operations
    /// - Index operations on large documents
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Serialization or deserialization failed.
    ///
    /// Occurs when converting between data formats (JSON, TOML, binary)
    /// fails due to incompatible formats or corruption.
    ///
    /// ## Common Causes
    ///
    /// - JSON/TOML syntax errors
    /// - Schema version mismatches
    /// - Data corruption
    /// - Incompatible format versions
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Firecrawl CLI is not installed or not in PATH.
    ///
    /// Indicates that the `firecrawl` command cannot be found. Users need to
    /// install the Firecrawl CLI to use web scraping functionality.
    ///
    /// ## Resolution
    ///
    /// Install Firecrawl CLI with: `npm install -g firecrawl`
    #[error("Firecrawl CLI not installed. Install with: npm install -g firecrawl")]
    FirecrawlNotInstalled,

    /// Firecrawl CLI version is too old.
    ///
    /// The installed Firecrawl CLI version does not meet the minimum
    /// requirements for this version of blz.
    ///
    /// ## Resolution
    ///
    /// Update Firecrawl CLI with: `npm update -g firecrawl`
    #[error("Firecrawl CLI version {found} is too old (minimum required: {required})")]
    FirecrawlVersionTooOld {
        /// Version that was found.
        found: String,
        /// Minimum required version.
        required: String,
    },

    /// Firecrawl CLI is not authenticated.
    ///
    /// The Firecrawl CLI requires authentication to use the API.
    /// Users need to log in before using scrape functionality.
    ///
    /// ## Resolution
    ///
    /// Authenticate with: `firecrawl login`
    #[error("Firecrawl CLI not authenticated. Run: firecrawl login")]
    FirecrawlNotAuthenticated,

    /// Firecrawl scrape operation failed.
    ///
    /// The scrape operation for a specific URL failed. This may be due to
    /// network issues, invalid URLs, or site-specific restrictions.
    ///
    /// ## Recoverability
    ///
    /// This error is typically recoverable - retry may succeed.
    #[error("Firecrawl scrape failed for '{url}': {reason}")]
    FirecrawlScrapeFailed {
        /// URL that failed to scrape.
        url: String,
        /// Reason for the failure.
        reason: String,
    },

    /// Firecrawl command execution failed.
    ///
    /// A general Firecrawl CLI command failed to execute properly.
    /// This covers execution failures that aren't specific to scraping.
    ///
    /// ## Recoverability
    ///
    /// This error is typically recoverable - retry may succeed.
    #[error("Firecrawl command failed: {0}")]
    FirecrawlCommandFailed(String),

    /// Generic error for uncategorized failures.
    ///
    /// Used for errors that don't fit other categories or for
    /// wrapping third-party errors that don't have specific mappings.
    #[error("{0}")]
    Other(String),
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<toml::ser::Error> for Error {
    fn from(err: toml::ser::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl Error {
    /// Check if the error might be recoverable through retry logic.
    ///
    /// Returns `true` for errors that are typically temporary and might succeed
    /// if the operation is retried after a delay. This includes network timeouts,
    /// connection failures, and temporary I/O issues.
    ///
    /// # Returns
    ///
    /// - `true` for potentially recoverable errors (timeouts, connection issues)
    /// - `false` for permanent errors (parse failures, invalid configuration)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use blz_core::{Error, Result};
    /// use std::io;
    ///
    /// let recoverable_errors = vec![
    ///     Error::Timeout("Request timed out".to_string()),
    ///     Error::Io(io::Error::new(io::ErrorKind::TimedOut, "timeout")),
    ///     Error::Io(io::Error::new(io::ErrorKind::Interrupted, "interrupted")),
    /// ];
    ///
    /// let permanent_errors = vec![
    ///     Error::Parse("Invalid markdown".to_string()),
    ///     Error::Config("Missing field".to_string()),
    ///     Error::InvalidUrl("Not a URL".to_string()),
    /// ];
    ///
    /// for error in recoverable_errors {
    ///     assert!(error.is_recoverable());
    /// }
    ///
    /// for error in permanent_errors {
    ///     assert!(!error.is_recoverable());
    /// }
    /// ```
    ///
    /// ## Retry Strategy
    ///
    /// When an error is recoverable, consider implementing exponential backoff:
    ///
    /// ```rust
    /// use blz_core::{Error, Result};
    /// use std::time::Duration;
    ///
    /// async fn retry_operation<F, T>(mut op: F, max_attempts: u32) -> Result<T>
    /// where
    ///     F: FnMut() -> Result<T>,
    /// {
    ///     let mut attempts = 0;
    ///     let mut delay = Duration::from_millis(100);
    ///
    ///     loop {
    ///         match op() {
    ///             Ok(result) => return Ok(result),
    ///             Err(e) if e.is_recoverable() && attempts < max_attempts => {
    ///                 attempts += 1;
    ///                 tokio::time::sleep(delay).await;
    ///                 delay *= 2; // Exponential backoff
    ///             }
    ///             Err(e) => return Err(e),
    ///         }
    ///     }
    /// }
    ///
    /// // Example usage:
    /// // let result = retry_operation(|| fetch_document(), 3).await?;
    /// ```
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Network(e) => {
                // Consider connection errors as recoverable
                e.is_timeout() || e.is_connect()
            },
            // Timeout and Firecrawl transient errors are recoverable
            Self::Timeout(_)
            | Self::FirecrawlScrapeFailed { .. }
            | Self::FirecrawlCommandFailed(_) => true,
            Self::Io(e) => {
                // Consider temporary I/O errors as recoverable
                matches!(
                    e.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted
                )
            },
            // All other errors (including Firecrawl installation/auth) are not recoverable
            _ => false,
        }
    }

    /// Get the error category as a string identifier.
    ///
    /// Returns a static string that categorizes the error type for logging,
    /// metrics collection, and error handling logic. This is useful for
    /// grouping errors in monitoring systems or implementing category-specific
    /// error handling.
    ///
    /// # Returns
    ///
    /// A static string representing the error category:
    ///
    /// - `"io"` - File system and I/O operations
    /// - `"network"` - HTTP requests and network operations
    /// - `"parse"` - Content parsing and format conversion
    /// - `"index"` - Search index operations
    /// - `"storage"` - Cache storage and management
    /// - `"config"` - Configuration and settings
    /// - `"not_found"` - Missing resources or files
    /// - `"invalid_url"` - URL format and validation
    /// - `"resource_limited"` - Resource constraints and limits
    /// - `"timeout"` - Operation timeouts
    /// - `"serialization"` - Data format conversion
    /// - `"other"` - Uncategorized errors
    ///
    /// # Examples
    ///
    /// ```rust
    /// use blz_core::Error;
    /// use std::collections::HashMap;
    ///
    /// // Track error counts by category
    /// let mut error_counts: HashMap<String, u32> = HashMap::new();
    ///
    /// fn record_error(error: &Error, counts: &mut HashMap<String, u32>) {
    ///     let category = error.category().to_string();
    ///     *counts.entry(category).or_insert(0) += 1;
    /// }
    ///
    /// // Usage in error handling
    /// let errors = vec![
    ///     Error::Parse("Invalid format".to_string()),
    ///     Error::Config("Missing field".to_string()),
    ///     Error::NotFound("Resource not found".to_string()),
    /// ];
    ///
    /// for error in &errors {
    ///     record_error(error, &mut error_counts);
    /// }
    /// ```
    ///
    /// ## Structured Logging
    ///
    /// ```rust
    /// use blz_core::Error;
    ///
    /// fn log_error(error: &Error) {
    ///     println!(
    ///         "{{\"level\":\"error\",\"category\":\"{}\",\"message\":\"{}\"}}",
    ///         error.category(),
    ///         error
    ///     );
    /// }
    /// ```
    #[must_use]
    pub const fn category(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Network(_) => "network",
            Self::Parse(_) => "parse",
            Self::Index(_) => "index",
            Self::Storage(_) => "storage",
            Self::Config(_) => "config",
            Self::NotFound(_) => "not_found",
            Self::InvalidUrl(_) => "invalid_url",
            Self::ResourceLimited(_) => "resource_limited",
            Self::Timeout(_) => "timeout",
            Self::Serialization(_) => "serialization",
            Self::FirecrawlNotInstalled
            | Self::FirecrawlVersionTooOld { .. }
            | Self::FirecrawlNotAuthenticated
            | Self::FirecrawlScrapeFailed { .. }
            | Self::FirecrawlCommandFailed(_) => "firecrawl",
            Self::Other(_) => "other",
        }
    }
}

/// Convenience type alias for `std::result::Result<T, Error>`.
///
/// This type is used throughout blz-core for consistent error handling.
/// It's equivalent to `std::result::Result<T, blz_core::Error>` but more concise.
///
/// # Examples
///
/// ```rust
/// use blz_core::{Result, Config};
///
/// fn load_config() -> Result<Config> {
///     Config::load() // Returns Result<Config>
/// }
///
/// fn main() -> Result<()> {
///     let config = load_config()?;
///     println!("Loaded config with root: {}", config.paths.root.display());
///     Ok(())
/// }
/// ```
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::disallowed_macros,
    clippy::unwrap_used,
    clippy::unnecessary_wraps
)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::io;

    #[test]
    fn test_error_display_formatting() {
        // Given: Different error variants
        let errors = vec![
            Error::Parse("invalid syntax".to_string()),
            Error::Index("search failed".to_string()),
            Error::Storage("disk full".to_string()),
            Error::Config("missing field".to_string()),
            Error::NotFound("document".to_string()),
            Error::InvalidUrl("not a url".to_string()),
            Error::ResourceLimited("too many requests".to_string()),
            Error::Timeout("operation timed out".to_string()),
            Error::Other("unknown error".to_string()),
        ];

        for error in errors {
            // When: Converting to string
            let error_string = error.to_string();

            // Then: Should contain descriptive information
            assert!(!error_string.is_empty());
            match error {
                Error::Parse(msg) => {
                    assert!(error_string.contains("Parse error"));
                    assert!(error_string.contains(&msg));
                },
                Error::Index(msg) => {
                    assert!(error_string.contains("Index error"));
                    assert!(error_string.contains(&msg));
                },
                Error::Storage(msg) => {
                    assert!(error_string.contains("Storage error"));
                    assert!(error_string.contains(&msg));
                },
                Error::Config(msg) => {
                    assert!(error_string.contains("Configuration error"));
                    assert!(error_string.contains(&msg));
                },
                Error::NotFound(msg) => {
                    assert!(error_string.contains("Not found"));
                    assert!(error_string.contains(&msg));
                },
                Error::InvalidUrl(msg) => {
                    assert!(error_string.contains("Invalid URL"));
                    assert!(error_string.contains(&msg));
                },
                Error::ResourceLimited(msg) => {
                    assert!(error_string.contains("Resource limited"));
                    assert!(error_string.contains(&msg));
                },
                Error::Timeout(msg) => {
                    assert!(error_string.contains("Timeout"));
                    assert!(error_string.contains(&msg));
                },
                Error::Other(msg) => {
                    assert_eq!(error_string, msg);
                },
                _ => {},
            }
        }
    }

    #[test]
    fn test_error_from_io_error() {
        // Given: Different types of I/O errors
        let io_errors = vec![
            io::Error::new(io::ErrorKind::NotFound, "file not found"),
            io::Error::new(io::ErrorKind::PermissionDenied, "access denied"),
            io::Error::new(io::ErrorKind::TimedOut, "operation timed out"),
            io::Error::new(io::ErrorKind::Interrupted, "interrupted"),
        ];

        for io_err in io_errors {
            // When: Converting to our Error type
            let error: Error = io_err.into();

            // Then: Should be IO error variant
            match error {
                Error::Io(inner) => {
                    assert!(!inner.to_string().is_empty());
                },
                _ => panic!("Expected IO error variant"),
            }
        }
    }

    #[test]
    fn test_error_from_reqwest_error() {
        // Given: Mock reqwest errors (using builder pattern since reqwest errors are opaque)
        // Note: We can't easily create reqwest::Error instances in tests, so we'll focus on
        // what we can test about the conversion

        // This test ensures the From implementation exists and compiles
        // In practice, reqwest errors would come from actual HTTP operations
        fn create_network_error_result() -> Result<()> {
            // This would typically come from reqwest operations
            let _client = reqwest::Client::new();
            // We can't easily trigger a reqwest error in tests without network calls
            // but we can verify the error type conversion works by checking the variant
            Ok(())
        }

        // When/Then: The conversion should compile and work (tested implicitly)
        assert!(create_network_error_result().is_ok());
    }

    #[test]
    fn test_error_categories() {
        // Given: All error variants
        let error_categories = vec![
            (Error::Io(io::Error::other("test")), "io"),
            (Error::Parse("test".to_string()), "parse"),
            (Error::Index("test".to_string()), "index"),
            (Error::Storage("test".to_string()), "storage"),
            (Error::Config("test".to_string()), "config"),
            (Error::NotFound("test".to_string()), "not_found"),
            (Error::InvalidUrl("test".to_string()), "invalid_url"),
            (
                Error::ResourceLimited("test".to_string()),
                "resource_limited",
            ),
            (Error::Timeout("test".to_string()), "timeout"),
            (Error::Serialization("test".to_string()), "serialization"),
            (Error::FirecrawlNotInstalled, "firecrawl"),
            (
                Error::FirecrawlVersionTooOld {
                    found: "1.0.0".to_string(),
                    required: "1.1.0".to_string(),
                },
                "firecrawl",
            ),
            (Error::FirecrawlNotAuthenticated, "firecrawl"),
            (
                Error::FirecrawlScrapeFailed {
                    url: "test".to_string(),
                    reason: "test".to_string(),
                },
                "firecrawl",
            ),
            (
                Error::FirecrawlCommandFailed("test".to_string()),
                "firecrawl",
            ),
            (Error::Other("test".to_string()), "other"),
        ];

        for (error, expected_category) in error_categories {
            // When: Getting error category
            let category = error.category();

            // Then: Should match expected category
            assert_eq!(category, expected_category);
        }
    }

    #[test]
    fn test_error_recoverability() {
        // Given: Various error scenarios
        let recoverable_errors = vec![
            Error::Io(io::Error::new(io::ErrorKind::TimedOut, "timeout")),
            Error::Io(io::Error::new(io::ErrorKind::Interrupted, "interrupted")),
            Error::Timeout("request timeout".to_string()),
        ];

        let non_recoverable_errors = vec![
            Error::Io(io::Error::new(io::ErrorKind::NotFound, "not found")),
            Error::Io(io::Error::new(io::ErrorKind::PermissionDenied, "denied")),
            Error::Parse("bad syntax".to_string()),
            Error::Index("corrupt index".to_string()),
            Error::Storage("disk failure".to_string()),
            Error::Config("invalid config".to_string()),
            Error::NotFound("missing".to_string()),
            Error::InvalidUrl("bad url".to_string()),
            Error::ResourceLimited("quota exceeded".to_string()),
            Error::Other("generic error".to_string()),
        ];

        // When/Then: Testing recoverability
        for error in recoverable_errors {
            assert!(
                error.is_recoverable(),
                "Expected {error:?} to be recoverable"
            );
        }

        for error in non_recoverable_errors {
            assert!(
                !error.is_recoverable(),
                "Expected {error:?} to be non-recoverable"
            );
        }
    }

    // ============================================================
    // Firecrawl Error Tests
    // ============================================================

    #[test]
    fn test_firecrawl_error_display() {
        // FirecrawlNotInstalled
        assert!(
            Error::FirecrawlNotInstalled
                .to_string()
                .contains("not installed")
        );
        assert!(
            Error::FirecrawlNotInstalled
                .to_string()
                .contains("npm install")
        );

        // FirecrawlVersionTooOld
        let version_error = Error::FirecrawlVersionTooOld {
            found: "1.0.0".to_string(),
            required: "1.1.0".to_string(),
        };
        assert!(version_error.to_string().contains("1.0.0"));
        assert!(version_error.to_string().contains("1.1.0"));
        assert!(version_error.to_string().contains("too old"));

        // FirecrawlNotAuthenticated
        assert!(
            Error::FirecrawlNotAuthenticated
                .to_string()
                .contains("not authenticated")
        );
        assert!(
            Error::FirecrawlNotAuthenticated
                .to_string()
                .contains("firecrawl login")
        );

        // FirecrawlScrapeFailed
        let scrape_error = Error::FirecrawlScrapeFailed {
            url: "https://example.com".to_string(),
            reason: "timeout".to_string(),
        };
        assert!(scrape_error.to_string().contains("https://example.com"));
        assert!(scrape_error.to_string().contains("timeout"));

        // FirecrawlCommandFailed
        let cmd_error = Error::FirecrawlCommandFailed("permission denied".to_string());
        assert!(cmd_error.to_string().contains("permission denied"));
        assert!(cmd_error.to_string().contains("command failed"));
    }

    #[test]
    fn test_firecrawl_error_recoverability() {
        // Permanent errors (require user action)
        assert!(!Error::FirecrawlNotInstalled.is_recoverable());
        assert!(
            !Error::FirecrawlVersionTooOld {
                found: "1.0.0".to_string(),
                required: "1.1.0".to_string(),
            }
            .is_recoverable()
        );
        assert!(!Error::FirecrawlNotAuthenticated.is_recoverable());

        // Recoverable errors (transient failures)
        assert!(
            Error::FirecrawlScrapeFailed {
                url: "https://example.com".to_string(),
                reason: "timeout".to_string(),
            }
            .is_recoverable()
        );
        assert!(Error::FirecrawlCommandFailed("failed".to_string()).is_recoverable());
    }

    #[test]
    fn test_firecrawl_error_category() {
        assert_eq!(Error::FirecrawlNotInstalled.category(), "firecrawl");
        assert_eq!(Error::FirecrawlNotAuthenticated.category(), "firecrawl");
        assert_eq!(
            Error::FirecrawlVersionTooOld {
                found: "1.0.0".to_string(),
                required: "1.1.0".to_string(),
            }
            .category(),
            "firecrawl"
        );
        assert_eq!(
            Error::FirecrawlScrapeFailed {
                url: "test".to_string(),
                reason: "test".to_string(),
            }
            .category(),
            "firecrawl"
        );
        assert_eq!(
            Error::FirecrawlCommandFailed("test".to_string()).category(),
            "firecrawl"
        );
    }

    #[test]
    fn test_error_debug_formatting() {
        // Given: Error with detailed information
        let error = Error::Parse("Failed to parse JSON at line 42".to_string());

        // When: Debug formatting
        let debug_str = format!("{error:?}");

        // Then: Should contain variant name and message
        assert!(debug_str.contains("Parse"));
        assert!(debug_str.contains("Failed to parse JSON at line 42"));
    }

    #[test]
    fn test_error_chain_source() {
        // Given: IO error that can be converted to our error type
        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let blz_error: Error = io_error.into();

        // When: Checking error source
        let source = std::error::Error::source(&blz_error);

        // Then: Should maintain the source chain
        assert!(source.is_some());
        let source_str = source.unwrap().to_string();
        assert!(source_str.contains("access denied"));
    }

    #[test]
    fn test_result_type_alias() {
        // Given: Function that returns our Result type
        fn test_function() -> Result<i32> {
            Ok(42)
        }

        fn test_error_function() -> Result<i32> {
            Err(Error::Other("test error".to_string()))
        }

        // When: Using the Result type
        let ok_result = test_function();
        let err_result = test_error_function();

        // Then: Should work as expected
        assert!(ok_result.is_ok());
        assert_eq!(ok_result.unwrap(), 42);

        assert!(err_result.is_err());
        if let Err(Error::Other(msg)) = err_result {
            assert_eq!(msg, "test error");
        } else {
            panic!("Expected Other error");
        }
    }

    // Property-based tests
    proptest! {
        #[test]
        fn test_parse_error_with_arbitrary_messages(msg in r".{0,1000}") {
            let error = Error::Parse(msg.clone());
            let error_string = error.to_string();

            prop_assert!(error_string.contains("Parse error"));
            prop_assert!(error_string.contains(&msg));
            prop_assert_eq!(error.category(), "parse");
            prop_assert!(!error.is_recoverable());
        }

        #[test]
        fn test_index_error_with_arbitrary_messages(msg in r".{0,1000}") {
            let error = Error::Index(msg.clone());
            let error_string = error.to_string();

            prop_assert!(error_string.contains("Index error"));
            prop_assert!(error_string.contains(&msg));
            prop_assert_eq!(error.category(), "index");
            prop_assert!(!error.is_recoverable());
        }

        #[test]
        fn test_storage_error_with_arbitrary_messages(msg in r".{0,1000}") {
            let error = Error::Storage(msg.clone());
            let error_string = error.to_string();

            prop_assert!(error_string.contains("Storage error"));
            prop_assert!(error_string.contains(&msg));
            prop_assert_eq!(error.category(), "storage");
            prop_assert!(!error.is_recoverable());
        }

        #[test]
        fn test_config_error_with_arbitrary_messages(msg in r".{0,1000}") {
            let error = Error::Config(msg.clone());
            let error_string = error.to_string();

            prop_assert!(error_string.contains("Configuration error"));
            prop_assert!(error_string.contains(&msg));
            prop_assert_eq!(error.category(), "config");
            prop_assert!(!error.is_recoverable());
        }

        #[test]
        fn test_other_error_with_arbitrary_messages(msg in r".{0,1000}") {
            let error = Error::Other(msg.clone());
            let error_string = error.to_string();

            prop_assert_eq!(error_string, msg);
            prop_assert_eq!(error.category(), "other");
            prop_assert!(!error.is_recoverable());
        }
    }

    // Security-focused tests
    #[test]
    fn test_error_with_malicious_messages() {
        // Given: Error messages with potentially malicious content
        let long_message = "very_long_message_".repeat(1000);
        let malicious_messages = vec![
            "\n\r\x00\x01malicious",
            "<script>alert('xss')</script>",
            "'; DROP TABLE users; --",
            "../../../etc/passwd",
            "\u{202e}reverse text\u{202d}",
            &long_message,
        ];

        for malicious_msg in malicious_messages {
            // When: Creating errors with malicious messages
            let errors = vec![
                Error::Parse(malicious_msg.to_string()),
                Error::Index(malicious_msg.to_string()),
                Error::Storage(malicious_msg.to_string()),
                Error::Config(malicious_msg.to_string()),
                Error::NotFound(malicious_msg.to_string()),
                Error::InvalidUrl(malicious_msg.to_string()),
                Error::ResourceLimited(malicious_msg.to_string()),
                Error::Timeout(malicious_msg.to_string()),
                Error::Other(malicious_msg.to_string()),
            ];

            for error in errors {
                // Then: Should handle malicious content safely
                let error_string = error.to_string();
                assert!(!error_string.is_empty());

                // Should preserve the malicious content (not sanitize it)
                // This is intentional - error handling should not modify user content
                // Sanitization should happen at display time if needed
                assert!(error_string.contains(malicious_msg));
            }
        }
    }

    #[test]
    fn test_error_with_unicode_messages() {
        // Given: Error messages with Unicode content
        let unicode_messages = vec![
            "エラーが発生しました",  // Japanese
            "حدث خطأ",               // Arabic
            "Произошла ошибка",      // Russian
            "🚨 Błąd krytyczny! 🚨", // Polish with emojis
            "Error: файл не найден", // Mixed languages
        ];

        for unicode_msg in unicode_messages {
            // When: Creating errors with Unicode messages
            let error = Error::Parse(unicode_msg.to_string());

            // Then: Should handle Unicode correctly
            let error_string = error.to_string();
            assert!(error_string.contains(unicode_msg));
            assert_eq!(error.category(), "parse");
        }
    }

    #[test]
    fn test_error_empty_messages() {
        // Given: Errors with empty messages
        let errors_with_empty_msgs = vec![
            Error::Parse(String::new()),
            Error::Index(String::new()),
            Error::Storage(String::new()),
            Error::Config(String::new()),
            Error::NotFound(String::new()),
            Error::InvalidUrl(String::new()),
            Error::ResourceLimited(String::new()),
            Error::Timeout(String::new()),
            Error::Other(String::new()),
        ];

        for error in errors_with_empty_msgs {
            // When: Converting to string
            let error_string = error.to_string();

            // Then: Check error formatting behavior
            if let Error::Other(_) = error {
                // Other errors just show the message (which is empty)
                assert_eq!(error_string, "");
            } else {
                // All other errors have descriptive prefixes even with empty messages
                assert!(!error_string.is_empty());
                assert!(
                    error_string.contains(':'),
                    "Error should contain colon separator: '{error_string}'"
                );
            }
        }
    }

    #[test]
    fn test_error_size() {
        // Given: Error enum
        // When: Checking size
        let error_size = std::mem::size_of::<Error>();

        // Then: Should be reasonably sized (not too large)
        // This helps ensure the error type is efficient to pass around
        assert!(error_size <= 64, "Error type too large: {error_size} bytes");
    }
}
