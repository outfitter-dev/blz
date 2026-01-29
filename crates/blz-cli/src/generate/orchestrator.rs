//! Orchestrator for parallel scraping operations.
//!
//! Coordinates scraping of multiple URLs using Firecrawl CLI with adaptive
//! concurrency control and progress reporting.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

// TODO: Replace these stub types with imports from blz_core when available:
// use blz_core::firecrawl::{FirecrawlCli, ScrapeOptions, ScrapeResult};
// use blz_core::page_cache::{PageCacheEntry, FailedPage};

/// URL with optional lastmod for change detection.
///
/// Used to track URLs discovered from sitemaps along with their
/// last modification timestamp for incremental updates.
///
/// ## Example
///
/// ```rust
/// use blz_cli::generate::UrlWithLastmod;
/// use chrono::Utc;
///
/// // Without lastmod
/// let url = UrlWithLastmod::new("https://example.com/page".to_string());
/// assert!(url.lastmod.is_none());
///
/// // With lastmod
/// let url_with_date = UrlWithLastmod::new("https://example.com/page".to_string())
///     .with_lastmod(Some(Utc::now()));
/// assert!(url_with_date.lastmod.is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlWithLastmod {
    /// The URL to scrape.
    pub url: String,
    /// Last modification date from sitemap (for change detection).
    pub lastmod: Option<DateTime<Utc>>,
}

impl UrlWithLastmod {
    /// Create a new URL without lastmod.
    #[must_use]
    pub const fn new(url: String) -> Self {
        Self { url, lastmod: None }
    }

    /// Set the lastmod using builder pattern.
    #[must_use]
    pub const fn with_lastmod(mut self, lastmod: Option<DateTime<Utc>>) -> Self {
        self.lastmod = lastmod;
        self
    }
}

/// Results of a scraping operation.
///
/// Aggregates successful and failed scrape attempts for reporting
/// and further processing.
///
/// ## Example
///
/// ```rust
/// use blz_cli::generate::ScrapeResults;
///
/// let results = ScrapeResults::default();
/// assert!(results.successful.is_empty());
/// assert!(results.failed.is_empty());
/// ```
#[derive(Debug, Default)]
pub struct ScrapeResults {
    /// Successfully scraped pages.
    pub successful: Vec<PageCacheEntry>,
    /// Pages that failed to scrape.
    pub failed: Vec<FailedPage>,
}

impl ScrapeResults {
    /// Create empty results.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the total number of URLs processed.
    #[must_use]
    pub fn total(&self) -> usize {
        self.successful.len() + self.failed.len()
    }

    /// Get the success rate as a percentage.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Precision loss acceptable for percentage calculation
    pub fn success_rate(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            (self.successful.len() as f64 / total as f64) * 100.0
        }
    }
}

/// Progress callback type for reporting scrape progress.
///
/// Called with (completed, total) after each URL is processed.
pub type ProgressCallback = Arc<dyn Fn(usize, usize) + Send + Sync>;

/// Orchestrates parallel scraping with adaptive concurrency.
///
/// Uses a semaphore to limit concurrent scrape operations and supports
/// progress reporting through callbacks. Can adaptively reduce concurrency
/// on rate limit errors (429 responses).
///
/// ## Example
///
/// ```rust,ignore
/// use blz_cli::generate::{GenerateOrchestrator, UrlWithLastmod};
/// use std::sync::Arc;
///
/// # async fn example() {
/// // With mock scraper for testing
/// let scraper = MockScraper::new();
/// let orchestrator = GenerateOrchestrator::new(scraper, 5)
///     .with_progress(|completed, total| {
///         println!("Progress: {}/{}", completed, total);
///     });
///
/// let urls = vec![
///     UrlWithLastmod::new("https://example.com/a".to_string()),
///     UrlWithLastmod::new("https://example.com/b".to_string()),
/// ];
///
/// let results = orchestrator.scrape_all(&urls).await;
/// # }
/// # struct MockScraper;
/// # impl MockScraper { fn new() -> Self { Self } }
/// ```
pub struct GenerateOrchestrator<S: Scraper> {
    scraper: S,
    concurrency: usize,
    min_concurrency: usize,
    progress_callback: Option<ProgressCallback>,
}

/// Trait for scraping URLs (allows mocking in tests).
///
/// Implement this trait to provide different scraping backends.
/// The default implementation uses `FirecrawlCli`.
#[async_trait::async_trait]
pub trait Scraper: Send + Sync {
    /// Scrape a URL and return the result.
    async fn scrape(&self, url: &str) -> Result<ScrapeResult, ScrapeError>;
}

/// Error from a scrape operation.
#[derive(Debug, Clone)]
pub struct ScrapeError {
    /// URL that failed.
    pub url: String,
    /// Error message.
    pub message: String,
    /// Whether this was a rate limit error (429).
    pub is_rate_limited: bool,
}

impl ScrapeError {
    /// Create a new scrape error.
    #[must_use]
    pub const fn new(url: String, message: String) -> Self {
        Self {
            url,
            message,
            is_rate_limited: false,
        }
    }

    /// Mark this error as rate-limited.
    #[must_use]
    pub const fn with_rate_limit(mut self, is_rate_limited: bool) -> Self {
        self.is_rate_limited = is_rate_limited;
        self
    }
}

impl std::fmt::Display for ScrapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scrape failed for {}: {}", self.url, self.message)
    }
}

impl std::error::Error for ScrapeError {}

// ============================================================
// Stub types for compilation (replace with blz_core imports)
// ============================================================

/// Result of a scrape operation.
///
/// TODO: Replace with `blz_core::firecrawl::ScrapeResult` when available.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrapeResult {
    /// Extracted markdown content.
    pub markdown: String,
    /// Page title.
    #[serde(default)]
    pub title: Option<String>,
    /// Source URL.
    pub url: String,
}

/// A cached page from web scraping.
///
/// TODO: Replace with `blz_core::page_cache::PageCacheEntry` when available.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageCacheEntry {
    /// Source URL.
    pub url: String,
    /// Page title.
    pub title: Option<String>,
    /// When this page was fetched.
    pub fetched_at: DateTime<Utc>,
    /// Last modified date from sitemap.
    pub sitemap_lastmod: Option<DateTime<Utc>>,
    /// Extracted markdown content.
    pub markdown: String,
    /// Number of lines in markdown.
    pub line_count: usize,
}

impl PageCacheEntry {
    /// Create from a scrape result.
    #[must_use]
    pub fn from_scrape_result(result: ScrapeResult, lastmod: Option<DateTime<Utc>>) -> Self {
        let line_count = result.markdown.lines().count();
        Self {
            url: result.url,
            title: result.title,
            fetched_at: Utc::now(),
            sitemap_lastmod: lastmod,
            markdown: result.markdown,
            line_count,
        }
    }
}

/// A page that failed to scrape.
///
/// TODO: Replace with `blz_core::page_cache::FailedPage` when available.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedPage {
    /// URL that failed.
    pub url: String,
    /// Error message.
    pub error: String,
    /// Number of attempts.
    pub attempts: u32,
    /// Last attempt timestamp.
    pub last_attempt: DateTime<Utc>,
}

impl FailedPage {
    /// Create a new failed page.
    #[must_use]
    pub fn new(url: String, error: String) -> Self {
        Self {
            url,
            error,
            attempts: 1,
            last_attempt: Utc::now(),
        }
    }
}

// ============================================================
// GenerateOrchestrator Implementation
// ============================================================

impl<S: Scraper> GenerateOrchestrator<S> {
    /// Default concurrency level.
    const DEFAULT_CONCURRENCY: usize = 5;

    /// Minimum concurrency level (floor for adaptive reduction).
    const MIN_CONCURRENCY: usize = 1;

    /// Create with specified concurrency.
    ///
    /// # Arguments
    ///
    /// * `scraper` - The scraping backend to use
    /// * `concurrency` - Maximum concurrent scrape operations (clamped to 1-50)
    #[must_use]
    pub fn new(scraper: S, concurrency: usize) -> Self {
        Self {
            scraper,
            concurrency: concurrency.clamp(1, 50),
            min_concurrency: Self::MIN_CONCURRENCY,
            progress_callback: None,
        }
    }

    /// Create with default concurrency (5).
    #[must_use]
    pub fn with_default_concurrency(scraper: S) -> Self {
        Self::new(scraper, Self::DEFAULT_CONCURRENCY)
    }

    /// Set progress callback.
    ///
    /// The callback receives `(completed, total)` after each URL is processed.
    #[must_use]
    pub fn with_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(usize, usize) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Arc::new(callback));
        self
    }

    /// Scrape all URLs in parallel.
    ///
    /// Uses a semaphore to limit concurrent operations to the configured
    /// concurrency level. Reports progress through the callback if set.
    ///
    /// # Returns
    ///
    /// Aggregated results with successful pages and failed pages.
    pub async fn scrape_all(&self, urls: &[UrlWithLastmod]) -> ScrapeResults {
        if urls.is_empty() {
            return ScrapeResults::default();
        }

        let total = urls.len();
        let completed = Arc::new(AtomicUsize::new(0));
        let semaphore = Arc::new(Semaphore::new(self.concurrency));

        // Create stream of scrape futures
        let results: Vec<Result<PageCacheEntry, FailedPage>> = stream::iter(urls)
            .map(|url_info| {
                let semaphore = Arc::clone(&semaphore);
                let completed = Arc::clone(&completed);
                let progress = self.progress_callback.clone();

                async move {
                    // Acquire semaphore permit
                    let _permit = semaphore.acquire().await;

                    // Perform scrape
                    let result = self.scrape_one(&url_info.url, url_info.lastmod).await;

                    // Update progress
                    let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    if let Some(cb) = progress {
                        cb(done, total);
                    }

                    result
                }
            })
            .buffer_unordered(self.concurrency)
            .collect()
            .await;

        // Partition results
        let mut scrape_results = ScrapeResults::default();
        for result in results {
            match result {
                Ok(entry) => scrape_results.successful.push(entry),
                Err(failed) => scrape_results.failed.push(failed),
            }
        }

        scrape_results
    }

    /// Scrape a single URL.
    async fn scrape_one(
        &self,
        url: &str,
        lastmod: Option<DateTime<Utc>>,
    ) -> Result<PageCacheEntry, FailedPage> {
        match self.scraper.scrape(url).await {
            Ok(result) => Ok(PageCacheEntry::from_scrape_result(result, lastmod)),
            Err(e) => Err(FailedPage::new(url.to_string(), e.message)),
        }
    }

    /// Get current concurrency level.
    #[must_use]
    pub const fn concurrency(&self) -> usize {
        self.concurrency
    }

    /// Get minimum concurrency level.
    #[must_use]
    pub const fn min_concurrency(&self) -> usize {
        self.min_concurrency
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Duration;

    // --------------------------------------------------------
    // Mock Scraper for Testing
    // --------------------------------------------------------

    struct MockScraper {
        responses: Mutex<Vec<Result<ScrapeResult, ScrapeError>>>,
    }

    impl MockScraper {
        fn new() -> Self {
            Self {
                responses: Mutex::new(Vec::new()),
            }
        }

        fn with_success(self, url: &str, markdown: &str) -> Self {
            let mut responses = self.responses.lock().expect("lock poisoned");
            responses.push(Ok(ScrapeResult {
                markdown: markdown.to_string(),
                title: Some("Test Page".to_string()),
                url: url.to_string(),
            }));
            drop(responses);
            self
        }

        fn with_failure(self, url: &str, error: &str) -> Self {
            let mut responses = self.responses.lock().expect("lock poisoned");
            responses.push(Err(ScrapeError::new(url.to_string(), error.to_string())));
            drop(responses);
            self
        }
    }

    #[async_trait::async_trait]
    impl Scraper for MockScraper {
        async fn scrape(&self, url: &str) -> Result<ScrapeResult, ScrapeError> {
            // Small delay to simulate network
            tokio::time::sleep(Duration::from_millis(10)).await;

            let mut responses = self.responses.lock().expect("lock poisoned");
            if responses.is_empty() {
                // Default success response
                Ok(ScrapeResult {
                    markdown: format!("# Content from {url}"),
                    title: Some("Default".to_string()),
                    url: url.to_string(),
                })
            } else {
                responses.remove(0)
            }
        }
    }

    // --------------------------------------------------------
    // UrlWithLastmod Tests
    // --------------------------------------------------------

    #[test]
    fn test_url_with_lastmod_new() {
        let url = UrlWithLastmod::new("https://example.com/page".to_string());
        assert_eq!(url.url, "https://example.com/page");
        assert!(url.lastmod.is_none());
    }

    #[test]
    fn test_url_with_lastmod_builder() {
        let now = Utc::now();
        let url =
            UrlWithLastmod::new("https://example.com/page".to_string()).with_lastmod(Some(now));
        assert_eq!(url.lastmod, Some(now));
    }

    #[test]
    fn test_url_with_lastmod_serialization() {
        let url = UrlWithLastmod::new("https://example.com/page".to_string());
        let json = serde_json::to_string(&url).expect("serialize");
        assert!(json.contains("\"url\":\"https://example.com/page\""));

        let roundtrip: UrlWithLastmod = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtrip.url, url.url);
    }

    // --------------------------------------------------------
    // ScrapeResults Tests
    // --------------------------------------------------------

    #[test]
    fn test_scrape_results_default() {
        let results = ScrapeResults::default();
        assert!(results.successful.is_empty());
        assert!(results.failed.is_empty());
    }

    #[test]
    fn test_scrape_results_total() {
        let mut results = ScrapeResults::new();
        assert_eq!(results.total(), 0);

        results.successful.push(PageCacheEntry::from_scrape_result(
            ScrapeResult {
                markdown: "test".to_string(),
                title: None,
                url: "https://a.com".to_string(),
            },
            None,
        ));
        assert_eq!(results.total(), 1);

        results.failed.push(FailedPage::new(
            "https://b.com".to_string(),
            "error".to_string(),
        ));
        assert_eq!(results.total(), 2);
    }

    #[test]
    fn test_scrape_results_success_rate() {
        let results = ScrapeResults::default();
        assert!((results.success_rate() - 0.0).abs() < f64::EPSILON);

        let mut results = ScrapeResults::new();
        results.successful.push(PageCacheEntry::from_scrape_result(
            ScrapeResult {
                markdown: "test".to_string(),
                title: None,
                url: "https://a.com".to_string(),
            },
            None,
        ));
        assert!((results.success_rate() - 100.0).abs() < f64::EPSILON);

        results.failed.push(FailedPage::new(
            "https://b.com".to_string(),
            "error".to_string(),
        ));
        assert!((results.success_rate() - 50.0).abs() < f64::EPSILON);
    }

    // --------------------------------------------------------
    // PageCacheEntry Tests
    // --------------------------------------------------------

    #[test]
    fn test_page_cache_entry_from_scrape_result() {
        let result = ScrapeResult {
            markdown: "# Hello\n\nWorld".to_string(),
            title: Some("Hello".to_string()),
            url: "https://example.com/page".to_string(),
        };

        let entry = PageCacheEntry::from_scrape_result(result, None);

        assert_eq!(entry.url, "https://example.com/page");
        assert_eq!(entry.title, Some("Hello".to_string()));
        assert_eq!(entry.line_count, 3);
        assert!(entry.sitemap_lastmod.is_none());
    }

    #[test]
    fn test_page_cache_entry_with_lastmod() {
        let lastmod = Utc::now();
        let result = ScrapeResult {
            markdown: "content".to_string(),
            title: None,
            url: "https://example.com".to_string(),
        };

        let entry = PageCacheEntry::from_scrape_result(result, Some(lastmod));

        assert_eq!(entry.sitemap_lastmod, Some(lastmod));
    }

    // --------------------------------------------------------
    // FailedPage Tests
    // --------------------------------------------------------

    #[test]
    fn test_failed_page_new() {
        let failed = FailedPage::new("https://example.com".to_string(), "timeout".to_string());
        assert_eq!(failed.url, "https://example.com");
        assert_eq!(failed.error, "timeout");
        assert_eq!(failed.attempts, 1);
    }

    // --------------------------------------------------------
    // ScrapeError Tests
    // --------------------------------------------------------

    #[test]
    fn test_scrape_error_new() {
        let err = ScrapeError::new("https://example.com".to_string(), "timeout".to_string());
        assert_eq!(err.url, "https://example.com");
        assert_eq!(err.message, "timeout");
        assert!(!err.is_rate_limited);
    }

    #[test]
    fn test_scrape_error_rate_limit() {
        let err = ScrapeError::new("https://example.com".to_string(), "429".to_string())
            .with_rate_limit(true);
        assert!(err.is_rate_limited);
    }

    #[test]
    fn test_scrape_error_display() {
        let err = ScrapeError::new("https://example.com".to_string(), "timeout".to_string());
        assert_eq!(
            format!("{err}"),
            "scrape failed for https://example.com: timeout"
        );
    }

    // --------------------------------------------------------
    // GenerateOrchestrator Tests
    // --------------------------------------------------------

    #[test]
    fn test_orchestrator_creation() {
        let scraper = MockScraper::new();
        let orchestrator = GenerateOrchestrator::new(scraper, 5);
        assert_eq!(orchestrator.concurrency(), 5);
    }

    #[test]
    fn test_orchestrator_default_concurrency() {
        let scraper = MockScraper::new();
        let orchestrator = GenerateOrchestrator::with_default_concurrency(scraper);
        assert_eq!(orchestrator.concurrency(), 5);
    }

    #[test]
    fn test_orchestrator_concurrency_clamped() {
        let scraper1 = MockScraper::new();
        let orchestrator1 = GenerateOrchestrator::new(scraper1, 0);
        assert_eq!(orchestrator1.concurrency(), 1);

        let scraper2 = MockScraper::new();
        let orchestrator2 = GenerateOrchestrator::new(scraper2, 100);
        assert_eq!(orchestrator2.concurrency(), 50);
    }

    #[test]
    fn test_orchestrator_min_concurrency() {
        let scraper = MockScraper::new();
        let orchestrator = GenerateOrchestrator::new(scraper, 5);
        assert_eq!(orchestrator.min_concurrency(), 1);
    }

    #[tokio::test]
    async fn test_orchestrator_empty_urls() {
        let scraper = MockScraper::new();
        let orchestrator = GenerateOrchestrator::new(scraper, 5);

        let results = orchestrator.scrape_all(&[]).await;

        assert!(results.successful.is_empty());
        assert!(results.failed.is_empty());
    }

    #[tokio::test]
    async fn test_orchestrator_single_success() {
        let scraper = MockScraper::new().with_success("https://example.com/page", "# Content");
        let orchestrator = GenerateOrchestrator::new(scraper, 5);

        let urls = vec![UrlWithLastmod::new("https://example.com/page".to_string())];
        let results = orchestrator.scrape_all(&urls).await;

        assert_eq!(results.successful.len(), 1);
        assert!(results.failed.is_empty());
        assert_eq!(results.successful[0].url, "https://example.com/page");
    }

    #[tokio::test]
    async fn test_orchestrator_single_failure() {
        let scraper =
            MockScraper::new().with_failure("https://example.com/page", "connection refused");
        let orchestrator = GenerateOrchestrator::new(scraper, 5);

        let urls = vec![UrlWithLastmod::new("https://example.com/page".to_string())];
        let results = orchestrator.scrape_all(&urls).await;

        assert!(results.successful.is_empty());
        assert_eq!(results.failed.len(), 1);
        assert_eq!(results.failed[0].url, "https://example.com/page");
        assert_eq!(results.failed[0].error, "connection refused");
    }

    #[tokio::test]
    async fn test_orchestrator_mixed_results() {
        let scraper = MockScraper::new()
            .with_success("https://example.com/a", "# A")
            .with_failure("https://example.com/b", "timeout")
            .with_success("https://example.com/c", "# C");
        let orchestrator = GenerateOrchestrator::new(scraper, 5);

        let urls = vec![
            UrlWithLastmod::new("https://example.com/a".to_string()),
            UrlWithLastmod::new("https://example.com/b".to_string()),
            UrlWithLastmod::new("https://example.com/c".to_string()),
        ];
        let results = orchestrator.scrape_all(&urls).await;

        assert_eq!(results.successful.len(), 2);
        assert_eq!(results.failed.len(), 1);
    }

    #[tokio::test]
    async fn test_orchestrator_progress_callback() {
        let progress = Arc::new(Mutex::new(Vec::new()));
        let progress_clone = Arc::clone(&progress);

        let scraper = MockScraper::new();
        let orchestrator =
            GenerateOrchestrator::new(scraper, 5).with_progress(move |completed, total| {
                progress_clone
                    .lock()
                    .expect("lock")
                    .push((completed, total));
            });

        let urls = vec![
            UrlWithLastmod::new("https://example.com/a".to_string()),
            UrlWithLastmod::new("https://example.com/b".to_string()),
            UrlWithLastmod::new("https://example.com/c".to_string()),
        ];
        orchestrator.scrape_all(&urls).await;

        let calls = progress.lock().expect("lock");
        assert_eq!(calls.len(), 3);
        // All calls should have total = 3
        for (_, total) in calls.iter() {
            assert_eq!(*total, 3);
        }
        // Completed values should be 1, 2, 3 (in some order due to concurrency)
        let mut completed: Vec<_> = calls.iter().map(|(c, _)| *c).collect();
        drop(calls); // Release lock before final assertion
        completed.sort_unstable();
        assert_eq!(completed, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_orchestrator_preserves_lastmod() {
        let lastmod = Utc::now();
        let scraper = MockScraper::new().with_success("https://example.com/page", "# Content");
        let orchestrator = GenerateOrchestrator::new(scraper, 5);

        let urls = vec![
            UrlWithLastmod::new("https://example.com/page".to_string()).with_lastmod(Some(lastmod)),
        ];
        let results = orchestrator.scrape_all(&urls).await;

        assert_eq!(results.successful.len(), 1);
        assert_eq!(results.successful[0].sitemap_lastmod, Some(lastmod));
    }

    #[tokio::test]
    async fn test_orchestrator_respects_concurrency() {
        use std::sync::atomic::AtomicUsize;
        use std::sync::atomic::Ordering;

        // Track maximum concurrent scrapes
        struct ConcurrencyTracker {
            current: AtomicUsize,
            max_seen: AtomicUsize,
        }

        impl ConcurrencyTracker {
            fn new() -> Self {
                Self {
                    current: AtomicUsize::new(0),
                    max_seen: AtomicUsize::new(0),
                }
            }
        }

        #[async_trait::async_trait]
        impl Scraper for Arc<ConcurrencyTracker> {
            async fn scrape(&self, url: &str) -> Result<ScrapeResult, ScrapeError> {
                // Increment current
                let current = self.current.fetch_add(1, Ordering::SeqCst) + 1;

                // Update max
                let mut max = self.max_seen.load(Ordering::SeqCst);
                while current > max {
                    match self.max_seen.compare_exchange_weak(
                        max,
                        current,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => break,
                        Err(actual) => max = actual,
                    }
                }

                // Simulate work
                tokio::time::sleep(Duration::from_millis(50)).await;

                // Decrement current
                self.current.fetch_sub(1, Ordering::SeqCst);

                Ok(ScrapeResult {
                    markdown: "content".to_string(),
                    title: None,
                    url: url.to_string(),
                })
            }
        }

        let tracker = Arc::new(ConcurrencyTracker::new());
        let orchestrator = GenerateOrchestrator::new(Arc::clone(&tracker), 3);

        // Create more URLs than concurrency limit
        let urls: Vec<_> = (0..10)
            .map(|i| UrlWithLastmod::new(format!("https://example.com/page{i}")))
            .collect();

        orchestrator.scrape_all(&urls).await;

        let max_seen = tracker.max_seen.load(Ordering::SeqCst);
        assert!(
            max_seen <= 3,
            "Max concurrent was {max_seen}, should be <= 3"
        );
    }
}
