//! Lastmod-based sync optimization for generated sources.
//!
//! This module provides cost-optimized sync for sources created via `blz generate`.
//! It uses sitemap lastmod timestamps to skip unchanged pages, saving 90%+ on
//! typical syncs.
//!
//! ## Cost Optimization Flow
//!
//! 1. Detect: source has `generate.json` -> generated source
//! 2. Fetch `sitemap.xml` (FREE - direct HTTP)
//! 3. Compare each URL's lastmod vs cached `sitemap_lastmod`
//! 4. Skip unchanged pages (FREE!)
//! 5. Scrape only new/changed pages (costs credits)
//! 6. Retry failed pages from previous sync
//! 7. Re-assemble with updated pages
//! 8. Update `generate.json` manifest
//!
//! ## Example
//!
//! ```rust,no_run
//! use blz_cli::commands::sync::generated::{is_generated_source, pages_needing_update};
//! use blz_core::Storage;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let storage = Storage::new()?;
//!
//! if is_generated_source(&storage, "hono") {
//!     // Load manifest and compare with sitemap
//!     // ...
//! }
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use blz_core::Storage;
use blz_core::discovery::SitemapEntry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Manifest for generated sources, stored as `generate.json`.
///
/// Tracks all scraped pages, their lastmod timestamps for change detection,
/// and any pages that failed scraping for retry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Exported for future Firecrawl integration
pub struct GenerateManifest {
    /// Version of the manifest format.
    pub version: u32,
    /// When the source was first generated.
    pub created_at: DateTime<Utc>,
    /// When the source was last synced.
    pub last_sync: DateTime<Utc>,
    /// URL of the sitemap used for discovery.
    pub sitemap_url: String,
    /// All successfully scraped pages.
    pub pages: Vec<PageCacheEntry>,
    /// Pages that failed to scrape (for retry).
    #[serde(default)]
    pub failed: Vec<FailedPage>,
    /// Total line count in assembled document.
    pub total_lines: usize,
}

#[allow(dead_code)] // Exported for future Firecrawl integration
impl GenerateManifest {
    /// Current manifest version.
    pub const VERSION: u32 = 1;

    /// Create a new manifest.
    #[must_use]
    pub fn new(sitemap_url: String) -> Self {
        let now = Utc::now();
        Self {
            version: Self::VERSION,
            created_at: now,
            last_sync: now,
            sitemap_url,
            pages: Vec::new(),
            failed: Vec::new(),
            total_lines: 0,
        }
    }

    /// Get a page by URL.
    #[must_use]
    pub fn get_page(&self, url: &str) -> Option<&PageCacheEntry> {
        self.pages.iter().find(|p| p.url == url)
    }

    /// Build a `HashMap` of pages by URL for O(1) lookup.
    #[must_use]
    pub fn pages_by_url(&self) -> HashMap<&str, &PageCacheEntry> {
        self.pages.iter().map(|p| (p.url.as_str(), p)).collect()
    }
}

/// A cached page from web scraping.
///
/// Stores the scraped content along with metadata for change detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Exported for future Firecrawl integration
pub struct PageCacheEntry {
    /// Source URL.
    pub url: String,
    /// Page title.
    pub title: Option<String>,
    /// When this page was fetched.
    pub fetched_at: DateTime<Utc>,
    /// Last modified date from sitemap (for change detection).
    pub sitemap_lastmod: Option<DateTime<Utc>>,
    /// Extracted markdown content.
    pub markdown: String,
    /// Number of lines in markdown.
    pub line_count: usize,
}

#[allow(dead_code)] // Exported for future Firecrawl integration
impl PageCacheEntry {
    /// Create a new page cache entry.
    #[must_use]
    pub fn new(url: String, markdown: String) -> Self {
        let line_count = markdown.lines().count();
        Self {
            url,
            title: None,
            fetched_at: Utc::now(),
            sitemap_lastmod: None,
            markdown,
            line_count,
        }
    }

    /// Set the title using builder pattern.
    #[must_use]
    pub fn with_title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    /// Set the lastmod using builder pattern.
    #[must_use]
    pub const fn with_lastmod(mut self, lastmod: Option<DateTime<Utc>>) -> Self {
        self.sitemap_lastmod = lastmod;
        self
    }
}

/// A page that failed to scrape.
///
/// Tracked for retry on subsequent syncs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Exported for future Firecrawl integration
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

#[allow(dead_code)] // Exported for future Firecrawl integration
impl FailedPage {
    /// Create a new failed page entry.
    #[must_use]
    pub fn new(url: String, error: String) -> Self {
        Self {
            url,
            error,
            attempts: 1,
            last_attempt: Utc::now(),
        }
    }

    /// Increment the attempt count.
    pub fn increment_attempts(&mut self) {
        self.attempts += 1;
        self.last_attempt = Utc::now();
    }
}

/// URL with optional lastmod for change detection.
///
/// Used to track URLs that need to be scraped during sync.
#[derive(Debug, Clone)]
pub struct UrlWithLastmod {
    /// The URL to scrape.
    pub url: String,
    /// Last modification date from sitemap.
    pub lastmod: Option<DateTime<Utc>>,
}

impl UrlWithLastmod {
    /// Create a new URL entry.
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

// ============================================================
// Core Detection and Comparison Functions
// ============================================================

/// Check if a source was generated (has generate.json).
///
/// Generated sources use lastmod-based sync optimization instead of
/// the standard refresh flow.
///
/// # Arguments
///
/// * `storage` - Storage instance for file access.
/// * `alias` - Source alias to check.
///
/// # Returns
///
/// `true` if the source has a `generate.json` manifest file.
#[must_use]
pub fn is_generated_source(storage: &Storage, alias: &str) -> bool {
    generate_manifest_path(storage, alias)
        .map(|p| p.exists())
        .unwrap_or(false)
}

/// Get the path to the generate.json manifest for a source.
///
/// # Errors
///
/// Returns an error if the alias is invalid.
pub fn generate_manifest_path(storage: &Storage, alias: &str) -> Result<PathBuf> {
    let tool_dir = storage.tool_dir(alias).context("Invalid alias")?;
    Ok(tool_dir.join("generate.json"))
}

/// Load the generate manifest for a source.
///
/// # Errors
///
/// Returns an error if the manifest doesn't exist or can't be parsed.
pub fn load_generate_manifest(storage: &Storage, alias: &str) -> Result<GenerateManifest> {
    let path = generate_manifest_path(storage, alias)?;

    if !path.exists() {
        anyhow::bail!("Source '{alias}' is not a generated source (no generate.json)");
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read generate.json for '{alias}'"))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse generate.json for '{alias}'"))
}

/// Save the generate manifest for a source.
///
/// # Errors
///
/// Returns an error if the manifest can't be serialized or written.
#[allow(dead_code)] // Exported for future Firecrawl integration
pub fn save_generate_manifest(
    storage: &Storage,
    alias: &str,
    manifest: &GenerateManifest,
) -> Result<()> {
    let path = generate_manifest_path(storage, alias)?;

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory for '{alias}'"))?;
    }

    let content = serde_json::to_string_pretty(manifest)
        .with_context(|| format!("Failed to serialize generate.json for '{alias}'"))?;

    // Write atomically via temp file
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &content)
        .with_context(|| format!("Failed to write temp generate.json for '{alias}'"))?;

    fs::rename(&tmp_path, &path)
        .with_context(|| format!("Failed to commit generate.json for '{alias}'"))?;

    Ok(())
}

/// Check if a page should be scraped based on lastmod comparison.
///
/// Returns `true` if:
/// - Sitemap lastmod is newer than cached lastmod
/// - Cached page has no lastmod but sitemap does (new timestamp available)
/// - Neither has lastmod (conservative: always scrape)
///
/// Returns `false` only when both have lastmod and sitemap <= cached.
#[must_use]
pub fn should_scrape(cached: &PageCacheEntry, sitemap: &SitemapEntry) -> bool {
    match (cached.sitemap_lastmod, sitemap.lastmod) {
        // Both have timestamps: compare them
        (Some(cached_mod), Some(sitemap_mod)) => sitemap_mod > cached_mod,
        // Cached has no timestamp, or sitemap has no timestamp: conservative, scrape
        (None, Some(_)) | (_, None) => true,
    }
}

/// Determine which pages need re-scraping.
///
/// Compares cached pages against sitemap entries and returns URLs that:
/// - Have newer lastmod in sitemap
/// - Are new (in sitemap but not in cache)
/// - Had no lastmod before but now have one
///
/// # Arguments
///
/// * `cached_pages` - Pages from the generate manifest.
/// * `sitemap_entries` - Current sitemap entries.
///
/// # Returns
///
/// Vector of URLs with their lastmod timestamps that need scraping.
#[must_use]
pub fn pages_needing_update(
    cached_pages: &[PageCacheEntry],
    sitemap_entries: &[SitemapEntry],
) -> Vec<UrlWithLastmod> {
    // Build lookup map for O(1) access
    let cached_by_url: HashMap<&str, &PageCacheEntry> =
        cached_pages.iter().map(|p| (p.url.as_str(), p)).collect();

    let mut updates = Vec::new();

    for sitemap_entry in sitemap_entries {
        let needs_update = cached_by_url
            .get(sitemap_entry.url.as_str())
            .is_none_or(|cached| should_scrape(cached, sitemap_entry));

        if needs_update {
            updates.push(
                UrlWithLastmod::new(sitemap_entry.url.clone()).with_lastmod(sitemap_entry.lastmod),
            );
        }
    }

    updates
}

/// Get pages that failed previously and should be retried.
///
/// # Arguments
///
/// * `storage` - Storage instance.
/// * `alias` - Source alias.
///
/// # Errors
///
/// Returns an error if the manifest can't be loaded.
#[allow(dead_code)] // Exported for future Firecrawl integration
pub fn pages_to_retry(storage: &Storage, alias: &str) -> Result<Vec<FailedPage>> {
    let manifest = load_generate_manifest(storage, alias)?;
    Ok(manifest.failed)
}

/// Categorize pages for sync reporting.
///
/// Returns (`unchanged_count`, `updated_urls`, `retry_urls`).
#[must_use]
pub fn categorize_sync_pages(
    cached_pages: &[PageCacheEntry],
    sitemap_entries: &[SitemapEntry],
    failed_pages: &[FailedPage],
) -> (usize, Vec<UrlWithLastmod>, Vec<UrlWithLastmod>) {
    let updates = pages_needing_update(cached_pages, sitemap_entries);
    let updated_urls: std::collections::HashSet<_> =
        updates.iter().map(|u| u.url.as_str()).collect();

    // Calculate unchanged (in sitemap and not needing update)
    let unchanged = sitemap_entries
        .iter()
        .filter(|e| !updated_urls.contains(e.url.as_str()))
        .count();

    // Failed pages to retry (not already in updates)
    let retries: Vec<_> = failed_pages
        .iter()
        .filter(|f| !updated_urls.contains(f.url.as_str()))
        .map(|f| UrlWithLastmod::new(f.url.clone()))
        .collect();

    (unchanged, updates, retries)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_macros,
    clippy::unnecessary_wraps
)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // --------------------------------------------------------
    // Test Helpers
    // --------------------------------------------------------

    fn create_test_storage() -> (Storage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let storage = Storage::with_root(temp_dir.path().to_path_buf())
            .expect("Failed to create test storage");
        (storage, temp_dir)
    }

    fn create_cached_page(url: &str, lastmod_date: &str) -> PageCacheEntry {
        let lastmod = chrono::NaiveDate::parse_from_str(lastmod_date, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc());

        PageCacheEntry::new(url.to_string(), format!("# Content for {url}")).with_lastmod(lastmod)
    }

    fn create_sitemap_entry(url: &str, lastmod_date: &str) -> SitemapEntry {
        let lastmod = chrono::NaiveDate::parse_from_str(lastmod_date, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc());

        SitemapEntry {
            url: url.to_string(),
            lastmod,
            changefreq: None,
            priority: None,
        }
    }

    // --------------------------------------------------------
    // is_generated_source Tests
    // --------------------------------------------------------

    #[test]
    fn test_is_generated_source_false_when_no_manifest() {
        let (storage, _temp) = create_test_storage();

        // No generate.json = not generated
        assert!(!is_generated_source(&storage, "native-source"));
    }

    #[test]
    fn test_is_generated_source_true_when_manifest_exists() {
        let (storage, _temp) = create_test_storage();

        // Create generate.json
        let manifest = GenerateManifest::new("https://example.com/sitemap.xml".to_string());
        storage.ensure_tool_dir("generated-source").unwrap();
        save_generate_manifest(&storage, "generated-source", &manifest).unwrap();

        assert!(is_generated_source(&storage, "generated-source"));
    }

    // --------------------------------------------------------
    // should_scrape Tests
    // --------------------------------------------------------

    #[test]
    fn test_should_scrape_newer_lastmod() {
        let cached = create_cached_page("https://example.com", "2024-01-01");
        let sitemap = create_sitemap_entry("https://example.com", "2024-02-01");

        assert!(should_scrape(&cached, &sitemap));
    }

    #[test]
    fn test_should_not_scrape_same_lastmod() {
        let cached = create_cached_page("https://example.com", "2024-01-15");
        let sitemap = create_sitemap_entry("https://example.com", "2024-01-15");

        assert!(!should_scrape(&cached, &sitemap));
    }

    #[test]
    fn test_should_not_scrape_older_lastmod() {
        let cached = create_cached_page("https://example.com", "2024-02-01");
        let sitemap = create_sitemap_entry("https://example.com", "2024-01-01");

        assert!(!should_scrape(&cached, &sitemap));
    }

    #[test]
    fn test_should_scrape_missing_cached_lastmod() {
        let cached = PageCacheEntry::new("https://example.com".to_string(), "content".to_string());
        // No lastmod set

        let sitemap = SitemapEntry {
            url: "https://example.com".to_string(),
            lastmod: Some(Utc::now()),
            changefreq: None,
            priority: None,
        };

        assert!(should_scrape(&cached, &sitemap));
    }

    #[test]
    fn test_should_scrape_when_sitemap_has_no_lastmod() {
        let cached = create_cached_page("https://example.com", "2024-01-15");
        let sitemap = SitemapEntry {
            url: "https://example.com".to_string(),
            lastmod: None,
            changefreq: None,
            priority: None,
        };

        // Conservative: scrape if unsure
        assert!(should_scrape(&cached, &sitemap));
    }

    #[test]
    fn test_should_scrape_both_no_lastmod() {
        let cached = PageCacheEntry::new("https://example.com".to_string(), "content".to_string());
        let sitemap = SitemapEntry {
            url: "https://example.com".to_string(),
            lastmod: None,
            changefreq: None,
            priority: None,
        };

        // Conservative: scrape if unsure
        assert!(should_scrape(&cached, &sitemap));
    }

    // --------------------------------------------------------
    // pages_needing_update Tests
    // --------------------------------------------------------

    #[test]
    fn test_pages_needing_update_detects_changed() {
        let cached = vec![
            create_cached_page("https://example.com/unchanged", "2024-01-15"),
            create_cached_page("https://example.com/changed", "2024-01-15"),
        ];

        let sitemap = vec![
            create_sitemap_entry("https://example.com/unchanged", "2024-01-15"),
            create_sitemap_entry("https://example.com/changed", "2024-02-01"), // newer
        ];

        let updates = pages_needing_update(&cached, &sitemap);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].url, "https://example.com/changed");
    }

    #[test]
    fn test_pages_needing_update_detects_new() {
        let cached = vec![create_cached_page(
            "https://example.com/existing",
            "2024-01-15",
        )];

        let sitemap = vec![
            create_sitemap_entry("https://example.com/existing", "2024-01-15"),
            create_sitemap_entry("https://example.com/new", "2024-02-15"), // new page
        ];

        let updates = pages_needing_update(&cached, &sitemap);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].url, "https://example.com/new");
    }

    #[test]
    fn test_pages_needing_update_mixed() {
        let cached = vec![
            create_cached_page("https://example.com/unchanged", "2024-01-15"),
            create_cached_page("https://example.com/changed", "2024-01-15"),
        ];

        let sitemap = vec![
            create_sitemap_entry("https://example.com/unchanged", "2024-01-15"),
            create_sitemap_entry("https://example.com/changed", "2024-02-01"), // newer
            create_sitemap_entry("https://example.com/new", "2024-02-15"),     // new page
        ];

        let updates = pages_needing_update(&cached, &sitemap);
        assert_eq!(updates.len(), 2); // changed + new
        let urls: Vec<_> = updates.iter().map(|u| u.url.as_str()).collect();
        assert!(urls.contains(&"https://example.com/changed"));
        assert!(urls.contains(&"https://example.com/new"));
    }

    #[test]
    fn test_pages_needing_update_empty_cache() {
        let cached: Vec<PageCacheEntry> = vec![];

        let sitemap = vec![
            create_sitemap_entry("https://example.com/page1", "2024-01-15"),
            create_sitemap_entry("https://example.com/page2", "2024-02-15"),
        ];

        let updates = pages_needing_update(&cached, &sitemap);
        assert_eq!(updates.len(), 2); // All new
    }

    #[test]
    fn test_pages_needing_update_empty_sitemap() {
        let cached = vec![create_cached_page(
            "https://example.com/page1",
            "2024-01-15",
        )];

        let sitemap: Vec<SitemapEntry> = vec![];

        let updates = pages_needing_update(&cached, &sitemap);
        assert!(updates.is_empty()); // Nothing in sitemap to update
    }

    #[test]
    fn test_pages_needing_update_preserves_lastmod() {
        let cached: Vec<PageCacheEntry> = vec![];

        let sitemap = vec![create_sitemap_entry(
            "https://example.com/page1",
            "2024-01-15",
        )];

        let updates = pages_needing_update(&cached, &sitemap);
        assert_eq!(updates.len(), 1);
        assert!(updates[0].lastmod.is_some());
    }

    // --------------------------------------------------------
    // GenerateManifest Tests
    // --------------------------------------------------------

    #[test]
    fn test_manifest_new() {
        let manifest = GenerateManifest::new("https://example.com/sitemap.xml".to_string());

        assert_eq!(manifest.version, GenerateManifest::VERSION);
        assert_eq!(manifest.sitemap_url, "https://example.com/sitemap.xml");
        assert!(manifest.pages.is_empty());
        assert!(manifest.failed.is_empty());
        assert_eq!(manifest.total_lines, 0);
    }

    #[test]
    fn test_manifest_pages_by_url() {
        let mut manifest = GenerateManifest::new("https://example.com/sitemap.xml".to_string());
        manifest.pages.push(create_cached_page(
            "https://example.com/page1",
            "2024-01-15",
        ));
        manifest.pages.push(create_cached_page(
            "https://example.com/page2",
            "2024-02-15",
        ));

        let pages_map = manifest.pages_by_url();

        assert_eq!(pages_map.len(), 2);
        assert!(pages_map.contains_key("https://example.com/page1"));
        assert!(pages_map.contains_key("https://example.com/page2"));
    }

    #[test]
    fn test_manifest_get_page() {
        let mut manifest = GenerateManifest::new("https://example.com/sitemap.xml".to_string());
        manifest.pages.push(create_cached_page(
            "https://example.com/page1",
            "2024-01-15",
        ));

        assert!(manifest.get_page("https://example.com/page1").is_some());
        assert!(
            manifest
                .get_page("https://example.com/nonexistent")
                .is_none()
        );
    }

    #[test]
    fn test_manifest_serialization_roundtrip() {
        let mut manifest = GenerateManifest::new("https://example.com/sitemap.xml".to_string());
        manifest.pages.push(create_cached_page(
            "https://example.com/page1",
            "2024-01-15",
        ));
        manifest.failed.push(FailedPage::new(
            "https://example.com/failed".to_string(),
            "timeout".to_string(),
        ));
        manifest.total_lines = 100;

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let parsed: GenerateManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.version, manifest.version);
        assert_eq!(parsed.sitemap_url, manifest.sitemap_url);
        assert_eq!(parsed.pages.len(), 1);
        assert_eq!(parsed.failed.len(), 1);
        assert_eq!(parsed.total_lines, 100);
    }

    // --------------------------------------------------------
    // PageCacheEntry Tests
    // --------------------------------------------------------

    #[test]
    fn test_page_cache_entry_new() {
        let entry = PageCacheEntry::new(
            "https://example.com/page".to_string(),
            "# Hello\n\nWorld".to_string(),
        );

        assert_eq!(entry.url, "https://example.com/page");
        assert_eq!(entry.line_count, 3);
        assert!(entry.title.is_none());
        assert!(entry.sitemap_lastmod.is_none());
    }

    #[test]
    fn test_page_cache_entry_builder() {
        let now = Utc::now();
        let entry = PageCacheEntry::new(
            "https://example.com/page".to_string(),
            "content".to_string(),
        )
        .with_title(Some("Test Page".to_string()))
        .with_lastmod(Some(now));

        assert_eq!(entry.title, Some("Test Page".to_string()));
        assert_eq!(entry.sitemap_lastmod, Some(now));
    }

    // --------------------------------------------------------
    // FailedPage Tests
    // --------------------------------------------------------

    #[test]
    fn test_failed_page_new() {
        let failed = FailedPage::new(
            "https://example.com/failed".to_string(),
            "connection refused".to_string(),
        );

        assert_eq!(failed.url, "https://example.com/failed");
        assert_eq!(failed.error, "connection refused");
        assert_eq!(failed.attempts, 1);
    }

    #[test]
    fn test_failed_page_increment_attempts() {
        let mut failed = FailedPage::new(
            "https://example.com/failed".to_string(),
            "timeout".to_string(),
        );
        let first_attempt = failed.last_attempt;

        std::thread::sleep(std::time::Duration::from_millis(10));
        failed.increment_attempts();

        assert_eq!(failed.attempts, 2);
        assert!(failed.last_attempt > first_attempt);
    }

    // --------------------------------------------------------
    // Storage Integration Tests
    // --------------------------------------------------------

    #[test]
    fn test_save_and_load_manifest() {
        let (storage, _temp) = create_test_storage();

        let mut manifest = GenerateManifest::new("https://example.com/sitemap.xml".to_string());
        manifest.pages.push(create_cached_page(
            "https://example.com/page1",
            "2024-01-15",
        ));
        manifest.total_lines = 50;

        // Ensure directory exists
        storage.ensure_tool_dir("test-source").unwrap();

        // Save
        save_generate_manifest(&storage, "test-source", &manifest).unwrap();

        // Verify file exists
        let path = generate_manifest_path(&storage, "test-source").unwrap();
        assert!(path.exists());

        // Load and verify
        let loaded = load_generate_manifest(&storage, "test-source").unwrap();
        assert_eq!(loaded.sitemap_url, manifest.sitemap_url);
        assert_eq!(loaded.pages.len(), 1);
        assert_eq!(loaded.total_lines, 50);
    }

    #[test]
    fn test_load_manifest_not_found() {
        let (storage, _temp) = create_test_storage();

        let result = load_generate_manifest(&storage, "nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not a generated source"));
    }

    // --------------------------------------------------------
    // categorize_sync_pages Tests
    // --------------------------------------------------------

    #[test]
    fn test_categorize_sync_pages() {
        let cached = vec![
            create_cached_page("https://example.com/unchanged", "2024-01-15"),
            create_cached_page("https://example.com/changed", "2024-01-15"),
        ];

        let sitemap = vec![
            create_sitemap_entry("https://example.com/unchanged", "2024-01-15"),
            create_sitemap_entry("https://example.com/changed", "2024-02-01"),
            create_sitemap_entry("https://example.com/new", "2024-02-15"),
        ];

        let failed = vec![FailedPage::new(
            "https://example.com/retry".to_string(),
            "timeout".to_string(),
        )];

        let (unchanged, updates, retries) = categorize_sync_pages(&cached, &sitemap, &failed);

        assert_eq!(unchanged, 1); // Only "unchanged"
        assert_eq!(updates.len(), 2); // "changed" + "new"
        assert_eq!(retries.len(), 1); // "retry"
    }

    #[test]
    fn test_categorize_sync_pages_retry_already_in_updates() {
        let cached = vec![create_cached_page("https://example.com/page", "2024-01-15")];

        let sitemap = vec![
            create_sitemap_entry("https://example.com/page", "2024-02-01"), // changed
        ];

        // Failed page is the same as changed page
        let failed = vec![FailedPage::new(
            "https://example.com/page".to_string(),
            "timeout".to_string(),
        )];

        let (unchanged, updates, retries) = categorize_sync_pages(&cached, &sitemap, &failed);

        assert_eq!(unchanged, 0);
        assert_eq!(updates.len(), 1); // Only one entry for "page"
        assert!(retries.is_empty()); // Not duplicated in retries
    }

    // --------------------------------------------------------
    // pages_to_retry Tests
    // --------------------------------------------------------

    #[test]
    fn test_pages_to_retry() {
        let (storage, _temp) = create_test_storage();

        let mut manifest = GenerateManifest::new("https://example.com/sitemap.xml".to_string());
        manifest.failed.push(FailedPage::new(
            "https://example.com/failed1".to_string(),
            "timeout".to_string(),
        ));
        manifest.failed.push(FailedPage::new(
            "https://example.com/failed2".to_string(),
            "403".to_string(),
        ));

        storage.ensure_tool_dir("test-source").unwrap();
        save_generate_manifest(&storage, "test-source", &manifest).unwrap();

        let retries = pages_to_retry(&storage, "test-source").unwrap();
        assert_eq!(retries.len(), 2);
    }
}
