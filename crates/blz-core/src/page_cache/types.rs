//! Type definitions for the page cache.
//!
//! Contains the core types used for storing and tracking scraped web pages.

use std::fmt::Write;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Durable page identifier: `pg_<sha256_12>`
///
/// Generated from URL to provide stable IDs across fetches.
/// The format is `pg_` followed by the first 12 characters of
/// the SHA-256 hash of the URL, giving 15 total characters.
///
/// ## Stability
///
/// The same URL will always produce the same `PageId`, making
/// it suitable for caching and incremental updates.
///
/// ## Example
///
/// ```rust
/// use blz_core::page_cache::PageId;
///
/// let id = PageId::from_url("https://example.com/page");
/// assert!(id.as_str().starts_with("pg_"));
/// assert_eq!(id.as_str().len(), 15);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageId(String);

impl PageId {
    /// Create a `PageId` from a URL.
    ///
    /// Uses first 12 chars of SHA-256 hash with "pg_" prefix.
    ///
    /// # Example
    ///
    /// ```rust
    /// use blz_core::page_cache::PageId;
    ///
    /// let id1 = PageId::from_url("https://example.com/a");
    /// let id2 = PageId::from_url("https://example.com/a");
    /// assert_eq!(id1, id2); // Same URL = same ID
    ///
    /// let id3 = PageId::from_url("https://example.com/b");
    /// assert_ne!(id1, id3); // Different URL = different ID
    /// ```
    #[must_use]
    pub fn from_url(url: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let result = hasher.finalize();
        // Take first 6 bytes (12 hex chars)
        let hex = result.iter().take(6).fold(String::new(), |mut acc, b| {
            // write! to String is infallible
            let _ = write!(acc, "{b:02x}");
            acc
        });
        Self(format!("pg_{hex}"))
    }

    /// Get the string representation.
    ///
    /// Returns the full ID including the `pg_` prefix.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A cached page from web scraping.
///
/// Stores the content and metadata for a successfully scraped web page.
/// Content is stored as markdown for consistent processing.
///
/// ## Serialization
///
/// Uses `camelCase` field names for JSON compatibility with web APIs.
///
/// ## Example
///
/// ```rust
/// use blz_core::page_cache::PageCacheEntry;
///
/// let entry = PageCacheEntry::new(
///     "https://example.com/page".to_string(),
///     "# Title\n\nContent".to_string(),
/// )
/// .with_title(Some("Title".to_string()));
///
/// assert_eq!(entry.line_count, 3);
/// assert_eq!(entry.title, Some("Title".to_string()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageCacheEntry {
    /// Unique identifier for this page.
    pub id: PageId,
    /// Source URL.
    pub url: String,
    /// Page title (from HTML or markdown).
    pub title: Option<String>,
    /// Section within the document (if applicable).
    pub section: Option<String>,
    /// When this page was fetched.
    pub fetched_at: DateTime<Utc>,
    /// Last modified date from sitemap (for change detection).
    pub sitemap_lastmod: Option<DateTime<Utc>>,
    /// Extracted markdown content.
    pub markdown: String,
    /// Number of lines in markdown.
    pub line_count: usize,
}

impl PageCacheEntry {
    /// Create a new cache entry.
    ///
    /// Automatically generates the `PageId` from the URL and
    /// calculates the line count from the markdown content.
    ///
    /// # Example
    ///
    /// ```rust
    /// use blz_core::page_cache::PageCacheEntry;
    ///
    /// let entry = PageCacheEntry::new(
    ///     "https://example.com/docs".to_string(),
    ///     "# Docs\n\nHello world".to_string(),
    /// );
    /// assert_eq!(entry.line_count, 3);
    /// assert!(entry.title.is_none());
    /// ```
    #[must_use]
    pub fn new(url: String, markdown: String) -> Self {
        let id = PageId::from_url(&url);
        let line_count = markdown.lines().count();
        Self {
            id,
            url,
            title: None,
            section: None,
            fetched_at: Utc::now(),
            sitemap_lastmod: None,
            markdown,
            line_count,
        }
    }

    /// Set the title using builder pattern.
    ///
    /// # Example
    ///
    /// ```rust
    /// use blz_core::page_cache::PageCacheEntry;
    ///
    /// let entry = PageCacheEntry::new(
    ///     "https://example.com".to_string(),
    ///     "# Title".to_string(),
    /// ).with_title(Some("Title".to_string()));
    ///
    /// assert_eq!(entry.title, Some("Title".to_string()));
    /// ```
    #[must_use]
    pub fn with_title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    /// Set the sitemap lastmod using builder pattern.
    ///
    /// # Example
    ///
    /// ```rust
    /// use blz_core::page_cache::PageCacheEntry;
    /// use chrono::Utc;
    ///
    /// let lastmod = Utc::now();
    /// let entry = PageCacheEntry::new(
    ///     "https://example.com".to_string(),
    ///     "content".to_string(),
    /// ).with_lastmod(Some(lastmod));
    ///
    /// assert!(entry.sitemap_lastmod.is_some());
    /// ```
    #[must_use]
    pub const fn with_lastmod(mut self, lastmod: Option<DateTime<Utc>>) -> Self {
        self.sitemap_lastmod = lastmod;
        self
    }

    /// Set the section using builder pattern.
    #[must_use]
    pub fn with_section(mut self, section: Option<String>) -> Self {
        self.section = section;
        self
    }
}

/// A page that failed to scrape.
///
/// Tracks failed scraping attempts with retry logic. After 3 failed
/// attempts, the page is no longer eligible for retry.
///
/// ## Example
///
/// ```rust
/// use blz_core::page_cache::FailedPage;
///
/// let mut failed = FailedPage::new(
///     "https://example.com/broken".to_string(),
///     "connection timeout".to_string(),
/// );
/// assert_eq!(failed.attempts, 1);
/// assert!(failed.should_retry());
///
/// failed.record_attempt("server error".to_string());
/// assert_eq!(failed.attempts, 2);
/// assert_eq!(failed.error, "server error");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedPage {
    /// Source URL that failed.
    pub url: String,
    /// Error message from the most recent attempt.
    pub error: String,
    /// Number of attempts made.
    pub attempts: u32,
    /// When the last attempt was made.
    pub last_attempt: DateTime<Utc>,
}

impl FailedPage {
    /// Maximum number of retry attempts allowed.
    const MAX_ATTEMPTS: u32 = 3;

    /// Create a new failed page record.
    ///
    /// Records the first failed attempt with the provided error message.
    ///
    /// # Example
    ///
    /// ```rust
    /// use blz_core::page_cache::FailedPage;
    ///
    /// let failed = FailedPage::new(
    ///     "https://example.com/broken".to_string(),
    ///     "404 not found".to_string(),
    /// );
    /// assert_eq!(failed.attempts, 1);
    /// assert_eq!(failed.error, "404 not found");
    /// ```
    #[must_use]
    pub fn new(url: String, error: String) -> Self {
        Self {
            url,
            error,
            attempts: 1,
            last_attempt: Utc::now(),
        }
    }

    /// Record a retry attempt.
    ///
    /// Increments the attempt counter and updates the error message
    /// and timestamp.
    ///
    /// # Example
    ///
    /// ```rust
    /// use blz_core::page_cache::FailedPage;
    ///
    /// let mut failed = FailedPage::new(
    ///     "https://example.com".to_string(),
    ///     "error 1".to_string(),
    /// );
    /// failed.record_attempt("error 2".to_string());
    /// assert_eq!(failed.attempts, 2);
    /// assert_eq!(failed.error, "error 2");
    /// ```
    pub fn record_attempt(&mut self, error: String) {
        self.attempts += 1;
        self.error = error;
        self.last_attempt = Utc::now();
    }

    /// Check if we should retry (max 3 attempts).
    ///
    /// Returns `true` if the number of attempts is less than the
    /// maximum allowed (3).
    ///
    /// # Example
    ///
    /// ```rust
    /// use blz_core::page_cache::FailedPage;
    ///
    /// let mut failed = FailedPage::new("https://example.com".to_string(), "e".to_string());
    /// assert!(failed.should_retry());  // 1 attempt
    /// failed.record_attempt("e".to_string());
    /// assert!(failed.should_retry());  // 2 attempts
    /// failed.record_attempt("e".to_string());
    /// assert!(!failed.should_retry()); // 3 attempts = max
    /// ```
    #[must_use]
    pub const fn should_retry(&self) -> bool {
        self.attempts < Self::MAX_ATTEMPTS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // PageId tests
    #[test]
    fn test_page_id_from_url() {
        let id = PageId::from_url("https://example.com/page");
        assert!(id.as_str().starts_with("pg_"));
        assert_eq!(id.as_str().len(), 15); // pg_ (3) + 12 hash chars
    }

    #[test]
    fn test_page_id_deterministic() {
        let id1 = PageId::from_url("https://example.com/page");
        let id2 = PageId::from_url("https://example.com/page");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_page_id_unique_for_different_urls() {
        let id1 = PageId::from_url("https://example.com/page1");
        let id2 = PageId::from_url("https://example.com/page2");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_page_id_display() {
        let id = PageId::from_url("https://example.com/page");
        assert_eq!(format!("{id}"), id.as_str());
    }

    #[test]
    fn test_page_id_hash_uniqueness() {
        // Test that small URL differences produce different IDs
        let id1 = PageId::from_url("https://example.com/a");
        let id2 = PageId::from_url("https://example.com/b");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_page_id_serialization() {
        let id = PageId::from_url("https://example.com/page");
        let json = serde_json::to_string(&id).expect("Should serialize");
        let roundtrip: PageId = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(id, roundtrip);
    }

    // PageCacheEntry tests
    #[test]
    fn test_page_cache_entry_creation() {
        let entry = PageCacheEntry::new(
            "https://example.com/page".to_string(),
            "# Hello\n\nWorld".to_string(),
        );
        assert!(entry.id.as_str().starts_with("pg_"));
        assert_eq!(entry.url, "https://example.com/page");
        assert_eq!(entry.line_count, 3);
        assert!(entry.title.is_none());
    }

    #[test]
    fn test_page_cache_entry_with_title() {
        let entry = PageCacheEntry::new(
            "https://example.com/page".to_string(),
            "# Hello".to_string(),
        )
        .with_title(Some("Hello".to_string()));
        assert_eq!(entry.title, Some("Hello".to_string()));
    }

    #[test]
    fn test_page_cache_entry_with_lastmod() {
        let lastmod = Utc::now();
        let entry = PageCacheEntry::new(
            "https://example.com/page".to_string(),
            "# Hello".to_string(),
        )
        .with_lastmod(Some(lastmod));
        assert_eq!(entry.sitemap_lastmod, Some(lastmod));
    }

    #[test]
    fn test_page_cache_entry_with_section() {
        let entry = PageCacheEntry::new(
            "https://example.com/page".to_string(),
            "content".to_string(),
        )
        .with_section(Some("API Reference".to_string()));
        assert_eq!(entry.section, Some("API Reference".to_string()));
    }

    #[test]
    fn test_page_cache_entry_line_count_empty() {
        let entry = PageCacheEntry::new("https://example.com".to_string(), String::new());
        assert_eq!(entry.line_count, 0);
    }

    #[test]
    fn test_page_cache_entry_line_count_single() {
        let entry =
            PageCacheEntry::new("https://example.com".to_string(), "single line".to_string());
        assert_eq!(entry.line_count, 1);
    }

    #[test]
    fn test_page_cache_entry_serialization() {
        let entry = PageCacheEntry::new(
            "https://example.com/page".to_string(),
            "# Content".to_string(),
        );
        let json = serde_json::to_string(&entry).expect("Should serialize");
        assert!(json.contains("fetchedAt")); // camelCase
        assert!(json.contains("lineCount")); // camelCase

        let roundtrip: PageCacheEntry = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(entry.id, roundtrip.id);
        assert_eq!(entry.markdown, roundtrip.markdown);
    }

    #[test]
    fn test_page_cache_entry_builder_chain() {
        let lastmod = Utc::now();
        let entry = PageCacheEntry::new(
            "https://example.com/page".to_string(),
            "# Title\n\nContent".to_string(),
        )
        .with_title(Some("Title".to_string()))
        .with_lastmod(Some(lastmod))
        .with_section(Some("Docs".to_string()));

        assert_eq!(entry.title, Some("Title".to_string()));
        assert_eq!(entry.sitemap_lastmod, Some(lastmod));
        assert_eq!(entry.section, Some("Docs".to_string()));
        assert_eq!(entry.line_count, 3);
    }

    // FailedPage tests
    #[test]
    fn test_failed_page_creation() {
        let failed = FailedPage::new(
            "https://example.com/page".to_string(),
            "timeout".to_string(),
        );
        assert_eq!(failed.attempts, 1);
        assert!(failed.should_retry());
    }

    #[test]
    fn test_failed_page_record_attempt() {
        let mut failed = FailedPage::new(
            "https://example.com/page".to_string(),
            "timeout".to_string(),
        );
        failed.record_attempt("connection refused".to_string());
        assert_eq!(failed.attempts, 2);
        assert_eq!(failed.error, "connection refused"); // Updated error
    }

    #[test]
    fn test_failed_page_max_retries() {
        let mut failed = FailedPage::new("https://example.com".to_string(), "error".to_string());
        assert!(failed.should_retry()); // 1 attempt
        failed.record_attempt("error".to_string());
        assert!(failed.should_retry()); // 2 attempts
        failed.record_attempt("error".to_string());
        assert!(!failed.should_retry()); // 3 attempts = max
    }

    #[test]
    fn test_failed_page_serialization() {
        let failed = FailedPage::new("https://example.com".to_string(), "error".to_string());
        let json = serde_json::to_string(&failed).expect("Should serialize");
        assert!(json.contains("lastAttempt")); // camelCase

        let roundtrip: FailedPage = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(failed.url, roundtrip.url);
    }

    #[test]
    fn test_failed_page_timestamp_updates() {
        let mut failed = FailedPage::new("https://example.com".to_string(), "error1".to_string());
        let first_attempt = failed.last_attempt;

        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        failed.record_attempt("error2".to_string());
        assert!(failed.last_attempt >= first_attempt);
    }
}
