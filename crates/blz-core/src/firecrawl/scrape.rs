//! Scrape and map operations for Firecrawl CLI.
//!
//! This module provides the core operations for fetching web content via
//! the Firecrawl CLI: scraping individual URLs and mapping domains to
//! discover URLs.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use blz_core::firecrawl::{FirecrawlCli, ScrapeOptions, ScrapeResult};
//!
//! # async fn example() -> blz_core::Result<()> {
//! let cli = FirecrawlCli::detect().await?;
//!
//! // Scrape a single URL
//! let options = ScrapeOptions::default().with_main_content_only(true);
//! let result = cli.scrape("https://example.com/docs", options).await?;
//! println!("Got {} bytes of markdown", result.markdown.len());
//!
//! // Map a domain to discover URLs
//! let map_result = cli.map("https://example.com").await?;
//! println!("Found {} URLs", map_result.urls.len());
//! # Ok(())
//! # }
//! ```

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::process::Command;
use tracing::instrument;

use super::FirecrawlCli;

/// Options for scraping a URL.
///
/// Controls how the Firecrawl CLI processes the target URL.
#[derive(Debug, Clone, Default)]
pub struct ScrapeOptions {
    /// Only extract the main content (filters out navigation, ads, etc.).
    ///
    /// Maps to the `--only-main-content` flag.
    pub only_main_content: bool,

    /// Timeout for the scrape operation.
    ///
    /// If not set, uses the default timeout of 60 seconds.
    pub timeout: Option<Duration>,
}

impl ScrapeOptions {
    /// Create new scrape options with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to extract only main content.
    #[must_use]
    pub const fn with_main_content_only(mut self, only_main: bool) -> Self {
        self.only_main_content = only_main;
        self
    }

    /// Set the timeout for the operation.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Result of a scrape operation.
///
/// Contains the extracted content and metadata from the scraped URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrapeResult {
    /// Extracted markdown content from the page.
    pub markdown: String,

    /// Page title (if available).
    #[serde(default)]
    pub title: Option<String>,

    /// Page description/meta description (if available).
    #[serde(default)]
    pub description: Option<String>,

    /// Source URL that was scraped.
    pub url: String,

    /// HTTP status code from the request (if available).
    #[serde(default)]
    pub status_code: Option<u16>,
}

/// Result of a map operation.
///
/// Contains the discovered URLs from the mapped domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapResult {
    /// Discovered URLs from the domain.
    pub urls: Vec<String>,

    /// Base URL that was mapped.
    pub base_url: String,
}

/// Default timeout for scrape operations (60 seconds).
const DEFAULT_SCRAPE_TIMEOUT: Duration = Duration::from_secs(60);

/// Default timeout for map operations (120 seconds).
const DEFAULT_MAP_TIMEOUT: Duration = Duration::from_secs(120);

impl FirecrawlCli {
    /// Scrape a URL and return markdown content.
    ///
    /// Uses the Firecrawl CLI to fetch the URL, convert it to markdown,
    /// and extract metadata.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to scrape
    /// * `options` - Configuration options for the scrape operation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The URL is invalid
    /// - The Firecrawl CLI command fails
    /// - The JSON output cannot be parsed
    /// - The operation times out
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # async fn example() -> blz_core::Result<()> {
    /// use blz_core::firecrawl::{FirecrawlCli, ScrapeOptions};
    ///
    /// let cli = FirecrawlCli::detect().await?;
    /// let result = cli.scrape(
    ///     "https://example.com",
    ///     ScrapeOptions::default().with_main_content_only(true)
    /// ).await?;
    /// println!("Title: {:?}", result.title);
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(level = "debug", skip(self, options), fields(url = %url))]
    pub async fn scrape(&self, url: &str, options: ScrapeOptions) -> Result<ScrapeResult> {
        let timeout = options.timeout.unwrap_or(DEFAULT_SCRAPE_TIMEOUT);

        // Build command arguments
        let mut args = vec!["scrape", url, "-f", "markdown", "--json"];

        if options.only_main_content {
            args.push("--only-main-content");
        }

        tracing::debug!(
            path = %self.path(),
            ?args,
            "Executing firecrawl scrape command"
        );

        // Execute with timeout
        let output = tokio::time::timeout(timeout, self.execute_command(&args)).await;

        let output = match output {
            Ok(result) => result.map_err(|e| Error::FirecrawlScrapeFailed {
                url: url.to_string(),
                reason: e.to_string(),
            })?,
            Err(_) => {
                return Err(Error::FirecrawlScrapeFailed {
                    url: url.to_string(),
                    reason: format!("timed out after {}s", timeout.as_secs()),
                });
            },
        };

        // Parse JSON output
        let result: ScrapeResult = serde_json::from_slice(&output.stdout).map_err(|e| {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(
                error = %e,
                stderr = %stderr,
                "Failed to parse firecrawl scrape output"
            );
            Error::FirecrawlScrapeFailed {
                url: url.to_string(),
                reason: format!("failed to parse output: {e}"),
            }
        })?;

        Ok(result)
    }

    /// Map a domain to discover URLs.
    ///
    /// Uses the Firecrawl CLI to crawl a domain and return all discovered URLs.
    /// This is useful for finding all documentation pages on a site.
    ///
    /// # Arguments
    ///
    /// * `url` - The base URL to map (typically the domain root or docs root)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The URL is invalid
    /// - The Firecrawl CLI command fails
    /// - The JSON output cannot be parsed
    /// - The operation times out
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # async fn example() -> blz_core::Result<()> {
    /// use blz_core::firecrawl::FirecrawlCli;
    ///
    /// let cli = FirecrawlCli::detect().await?;
    /// let result = cli.map("https://docs.example.com").await?;
    ///
    /// for url in &result.urls {
    ///     println!("Found: {}", url);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(level = "debug", skip(self), fields(url = %url))]
    pub async fn map(&self, url: &str) -> Result<MapResult> {
        let args = vec!["map", url, "--json"];

        tracing::debug!(
            path = %self.path(),
            ?args,
            "Executing firecrawl map command"
        );

        // Execute with timeout
        let output = tokio::time::timeout(DEFAULT_MAP_TIMEOUT, self.execute_command(&args)).await;

        let output = match output {
            Ok(result) => result?,
            Err(_) => {
                return Err(Error::Timeout(format!(
                    "Firecrawl map timed out after {}s",
                    DEFAULT_MAP_TIMEOUT.as_secs()
                )));
            },
        };

        // Parse JSON output
        let result: MapResult = serde_json::from_slice(&output.stdout).map_err(|e| {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(
                error = %e,
                stderr = %stderr,
                "Failed to parse firecrawl map output"
            );
            Error::Parse(format!("Failed to parse firecrawl map output: {e}"))
        })?;

        Ok(result)
    }

    /// Execute a firecrawl command and return the output.
    ///
    /// Internal helper that handles command execution and error checking.
    async fn execute_command(&self, args: &[&str]) -> Result<std::process::Output> {
        let output = Command::new(self.path())
            .args(args)
            .output()
            .await
            .map_err(Error::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            tracing::warn!(
                exit_code = ?output.status.code(),
                stderr = %stderr,
                stdout = %stdout,
                "Firecrawl command failed"
            );

            return Err(Error::FirecrawlCommandFailed(stderr.trim().to_string()));
        }

        Ok(output)
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_macros
)]
mod tests {
    use super::*;

    // ============================================================
    // ScrapeOptions Tests
    // ============================================================

    #[test]
    fn test_scrape_options_default() {
        let opts = ScrapeOptions::default();
        assert!(!opts.only_main_content);
        assert!(opts.timeout.is_none());
    }

    #[test]
    fn test_scrape_options_new() {
        let opts = ScrapeOptions::new();
        assert!(!opts.only_main_content);
        assert!(opts.timeout.is_none());
    }

    #[test]
    fn test_scrape_options_with_main_content_only() {
        let opts = ScrapeOptions::default().with_main_content_only(true);
        assert!(opts.only_main_content);

        let opts = ScrapeOptions::default().with_main_content_only(false);
        assert!(!opts.only_main_content);
    }

    #[test]
    fn test_scrape_options_with_timeout() {
        let timeout = Duration::from_secs(30);
        let opts = ScrapeOptions::default().with_timeout(timeout);
        assert_eq!(opts.timeout, Some(timeout));
    }

    #[test]
    fn test_scrape_options_builder_chain() {
        let opts = ScrapeOptions::new()
            .with_main_content_only(true)
            .with_timeout(Duration::from_secs(45));

        assert!(opts.only_main_content);
        assert_eq!(opts.timeout, Some(Duration::from_secs(45)));
    }

    // ============================================================
    // ScrapeResult Deserialization Tests
    // ============================================================

    #[test]
    fn test_scrape_result_deserialize_full() {
        let json = r#"{"markdown": "Hello World", "title": "Hello", "description": "A page", "url": "https://example.com/page", "statusCode": 200}"#;

        let result: ScrapeResult = serde_json::from_str(json).unwrap();

        assert_eq!(result.markdown, "Hello World");
        assert_eq!(result.title, Some("Hello".to_string()));
        assert_eq!(result.description, Some("A page".to_string()));
        assert_eq!(result.url, "https://example.com/page");
        assert_eq!(result.status_code, Some(200));
    }

    #[test]
    fn test_scrape_result_deserialize_minimal() {
        let json = r#"{"markdown": "content", "url": "https://example.com"}"#;

        let result: ScrapeResult = serde_json::from_str(json).unwrap();

        assert_eq!(result.markdown, "content");
        assert_eq!(result.url, "https://example.com");
        assert!(result.title.is_none());
        assert!(result.description.is_none());
        assert!(result.status_code.is_none());
    }

    #[test]
    fn test_scrape_result_deserialize_missing_optional_fields() {
        let json = r#"{"markdown": "content", "url": "https://example.com"}"#;
        let result: ScrapeResult = serde_json::from_str(json).unwrap();

        assert_eq!(result.markdown, "content");
        assert_eq!(result.url, "https://example.com");
        assert!(result.title.is_none());
        assert!(result.description.is_none());
        assert!(result.status_code.is_none());
    }

    #[test]
    fn test_scrape_result_deserialize_null_optional_fields() {
        let json = r#"{"markdown": "content", "url": "https://example.com", "title": null, "description": null, "statusCode": null}"#;

        let result: ScrapeResult = serde_json::from_str(json).unwrap();

        assert!(result.title.is_none());
        assert!(result.description.is_none());
        assert!(result.status_code.is_none());
    }

    #[test]
    fn test_scrape_result_deserialize_empty_markdown() {
        let json = r#"{"markdown": "", "url": "https://example.com"}"#;
        let result: ScrapeResult = serde_json::from_str(json).unwrap();

        assert_eq!(result.markdown, "");
    }

    #[test]
    fn test_scrape_result_serialize() {
        let result = ScrapeResult {
            markdown: "Test content".to_string(),
            title: Some("Test".to_string()),
            description: None,
            url: "https://example.com".to_string(),
            status_code: Some(200),
        };

        let json = serde_json::to_string(&result).unwrap();

        assert!(json.contains(r#""markdown":"Test content""#));
        assert!(json.contains(r#""title":"Test""#));
        assert!(json.contains(r#""url":"https://example.com""#));
        assert!(json.contains(r#""statusCode":200"#));
    }

    // ============================================================
    // MapResult Deserialization Tests
    // ============================================================

    #[test]
    fn test_map_result_deserialize() {
        let json = r#"{"urls": ["https://example.com/a", "https://example.com/b"], "baseUrl": "https://example.com"}"#;

        let result: MapResult = serde_json::from_str(json).unwrap();

        assert_eq!(result.urls.len(), 2);
        assert_eq!(result.urls[0], "https://example.com/a");
        assert_eq!(result.urls[1], "https://example.com/b");
        assert_eq!(result.base_url, "https://example.com");
    }

    #[test]
    fn test_map_result_deserialize_empty_urls() {
        let json = r#"{"urls": [], "baseUrl": "https://example.com"}"#;

        let result: MapResult = serde_json::from_str(json).unwrap();

        assert!(result.urls.is_empty());
        assert_eq!(result.base_url, "https://example.com");
    }

    #[test]
    fn test_map_result_deserialize_many_urls() {
        let urls: Vec<String> = (0..100)
            .map(|i| format!("https://example.com/page{i}"))
            .collect();
        let json = format!(
            r#"{{"urls": {}, "baseUrl": "https://example.com"}}"#,
            serde_json::to_string(&urls).unwrap()
        );

        let result: MapResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.urls.len(), 100);
    }

    #[test]
    fn test_map_result_serialize() {
        let result = MapResult {
            urls: vec![
                "https://example.com/a".to_string(),
                "https://example.com/b".to_string(),
            ],
            base_url: "https://example.com".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();

        assert!(json.contains("\"urls\":["));
        assert!(json.contains("\"baseUrl\":\"https://example.com\""));
    }

    // ============================================================
    // CLI Argument Building Tests
    // ============================================================

    #[test]
    fn test_build_scrape_args_default() {
        let options = ScrapeOptions::default();
        let url = "https://example.com";

        let mut args = vec!["scrape", url, "-f", "markdown", "--json"];

        if options.only_main_content {
            args.push("--only-main-content");
        }

        assert_eq!(
            args,
            vec!["scrape", "https://example.com", "-f", "markdown", "--json"]
        );
    }

    #[test]
    fn test_build_scrape_args_with_main_content() {
        let options = ScrapeOptions::default().with_main_content_only(true);
        let url = "https://example.com";

        let mut args = vec!["scrape", url, "-f", "markdown", "--json"];

        if options.only_main_content {
            args.push("--only-main-content");
        }

        assert_eq!(
            args,
            vec![
                "scrape",
                "https://example.com",
                "-f",
                "markdown",
                "--json",
                "--only-main-content"
            ]
        );
    }

    #[test]
    fn test_build_map_args() {
        let url = "https://example.com";
        let args = vec!["map", url, "--json"];

        assert_eq!(args, vec!["map", "https://example.com", "--json"]);
    }

    // ============================================================
    // Timeout Tests
    // ============================================================

    #[test]
    fn test_default_scrape_timeout() {
        assert_eq!(DEFAULT_SCRAPE_TIMEOUT, Duration::from_secs(60));
    }

    #[test]
    fn test_default_map_timeout() {
        assert_eq!(DEFAULT_MAP_TIMEOUT, Duration::from_secs(120));
    }

    #[test]
    fn test_custom_timeout_used() {
        let custom_timeout = Duration::from_secs(30);
        let options = ScrapeOptions::default().with_timeout(custom_timeout);

        let timeout = options.timeout.unwrap_or(DEFAULT_SCRAPE_TIMEOUT);
        assert_eq!(timeout, custom_timeout);
    }

    #[test]
    fn test_default_timeout_when_none() {
        let options = ScrapeOptions::default();

        let timeout = options.timeout.unwrap_or(DEFAULT_SCRAPE_TIMEOUT);
        assert_eq!(timeout, DEFAULT_SCRAPE_TIMEOUT);
    }

    // ============================================================
    // Edge Cases
    // ============================================================

    #[test]
    fn test_scrape_result_with_unicode() {
        // JSON with actual Unicode characters (Japanese)
        let json = r#"{"markdown": "Japanese: \u65e5\u672c\u8a9e", "title": "\u30bf\u30a4\u30c8\u30eb", "url": "https://example.com"}"#;

        let result: ScrapeResult = serde_json::from_str(json).unwrap();

        // Should properly decode Unicode
        assert!(!result.markdown.is_empty());
        assert!(result.title.is_some());
    }

    #[test]
    fn test_scrape_result_with_newlines() {
        // JSON with escaped newlines
        let json = r#"{"markdown": "Line 1\nLine 2\nLine 3", "url": "https://example.com"}"#;

        let result: ScrapeResult = serde_json::from_str(json).unwrap();

        assert!(result.markdown.contains('\n'));
        assert!(result.markdown.contains("Line 1"));
        assert!(result.markdown.contains("Line 3"));
    }

    #[test]
    fn test_map_result_with_query_params() {
        let json = r#"{"urls": ["https://example.com/page?foo=bar", "https://example.com/page?baz=qux"], "baseUrl": "https://example.com"}"#;

        let result: MapResult = serde_json::from_str(json).unwrap();

        assert_eq!(result.urls.len(), 2);
        assert!(result.urls[0].contains("?foo=bar"));
        assert!(result.urls[1].contains("?baz=qux"));
    }
}
