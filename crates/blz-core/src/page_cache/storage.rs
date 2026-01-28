//! Storage layer for page cache.
//!
//! Provides persistent storage for scraped pages with atomic writes,
//! backup support, and failed page tracking.
//!
//! ## Storage Layout
//!
//! ```text
//! sources/<alias>/
//!   .cache/
//!     pages/
//!       pg_a1b2c3d4e5f6.json   # Individual cached pages
//!       pg_b2c3d4e5f6a7.json
//!     failed.json              # Failed pages for retry
//!     pages.bak/               # Backup directory
//!       index.json             # Backup manifest
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use blz_core::page_cache::{PageCacheStorage, PageCacheEntry};
//! use tempfile::TempDir;
//!
//! let temp = TempDir::new().unwrap();
//! let storage = PageCacheStorage::new(temp.path());
//!
//! // Save a page
//! let entry = PageCacheEntry::new(
//!     "https://example.com/page".to_string(),
//!     "# Content".to_string(),
//! );
//! storage.save_page("my-source", &entry).unwrap();
//!
//! // Load it back
//! let loaded = storage.load_page("my-source", &entry.id).unwrap();
//! assert_eq!(loaded.markdown, "# Content");
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::page_cache::{FailedPage, PageCacheEntry, PageId};
use crate::{Error, Result};

/// Information about a backup operation.
///
/// Returned when pages are backed up, containing metadata about
/// what was backed up and where it was stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    /// When the backup was created.
    pub backed_up_at: DateTime<Utc>,
    /// Reason for the backup (e.g., "pre-upgrade", "manual").
    pub reason: String,
    /// Number of pages included in the backup.
    pub page_count: usize,
    /// Relative path to the backup directory.
    pub path: String,
}

/// Storage for page cache with atomic writes and backup support.
///
/// Manages persistent storage of scraped pages, supporting:
/// - Atomic writes to prevent corruption
/// - Individual page files for efficient updates
/// - Failed page tracking for retry logic
/// - Backup and restore capabilities
///
/// ## Thread Safety
///
/// Operations are not thread-safe across processes. Use external
/// locking if concurrent access from multiple processes is needed.
pub struct PageCacheStorage {
    root: PathBuf,
}

impl PageCacheStorage {
    /// Create storage with the given root directory.
    ///
    /// The root directory should be the blz data directory
    /// (e.g., `~/.blz` or `$XDG_DATA_HOME/blz`).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use blz_core::page_cache::PageCacheStorage;
    ///
    /// let storage = PageCacheStorage::new("/home/user/.blz");
    /// ```
    #[must_use]
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Get the cache directory for a source.
    ///
    /// Returns `sources/<alias>/.cache`.
    fn cache_dir(&self, alias: &str) -> PathBuf {
        self.root.join("sources").join(alias).join(".cache")
    }

    /// Get the pages directory for a source.
    ///
    /// Returns `sources/<alias>/.cache/pages`.
    fn pages_dir(&self, alias: &str) -> PathBuf {
        self.cache_dir(alias).join("pages")
    }

    /// Get the path to a specific page file.
    fn page_path(&self, alias: &str, id: &PageId) -> PathBuf {
        self.pages_dir(alias).join(format!("{}.json", id.as_str()))
    }

    /// Get the path to the failed pages file.
    fn failed_pages_path(&self, alias: &str) -> PathBuf {
        self.cache_dir(alias).join("failed.json")
    }

    // === Page Operations ===

    /// Save a page to the cache.
    ///
    /// Uses atomic write (temp file + rename) to prevent corruption
    /// if the process crashes mid-write.
    ///
    /// # Errors
    ///
    /// Returns an error if the page cannot be serialized or written.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use blz_core::page_cache::{PageCacheStorage, PageCacheEntry};
    /// use tempfile::TempDir;
    ///
    /// let temp = TempDir::new().unwrap();
    /// let storage = PageCacheStorage::new(temp.path());
    ///
    /// let entry = PageCacheEntry::new(
    ///     "https://example.com/page".to_string(),
    ///     "# Hello".to_string(),
    /// );
    /// storage.save_page("docs", &entry).unwrap();
    /// ```
    pub fn save_page(&self, alias: &str, entry: &PageCacheEntry) -> Result<()> {
        let pages_dir = self.pages_dir(alias);
        fs::create_dir_all(&pages_dir)
            .map_err(|e| Error::Storage(format!("Failed to create pages directory: {e}")))?;

        let path = self.page_path(alias, &entry.id);
        let json = serde_json::to_string_pretty(entry)
            .map_err(|e| Error::Storage(format!("Failed to serialize page: {e}")))?;

        // Atomic write: temp file + rename
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, json)
            .map_err(|e| Error::Storage(format!("Failed to write temp page file: {e}")))?;

        // Handle Windows: remove target before rename
        #[cfg(target_os = "windows")]
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| Error::Storage(format!("Failed to remove existing page: {e}")))?;
        }

        fs::rename(&tmp_path, &path)
            .map_err(|e| Error::Storage(format!("Failed to commit page file: {e}")))?;

        debug!("Saved page {} for {}", entry.id, alias);
        Ok(())
    }

    /// Load a page by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the page doesn't exist or cannot be read.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use blz_core::page_cache::{PageCacheStorage, PageId};
    /// use tempfile::TempDir;
    ///
    /// let temp = TempDir::new().unwrap();
    /// let storage = PageCacheStorage::new(temp.path());
    ///
    /// let id = PageId::from_url("https://example.com/page");
    /// match storage.load_page("docs", &id) {
    ///     Ok(entry) => println!("Found: {}", entry.url),
    ///     Err(_) => println!("Page not found"),
    /// }
    /// ```
    pub fn load_page(&self, alias: &str, id: &PageId) -> Result<PageCacheEntry> {
        let path = self.page_path(alias, id);
        let json = fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read page {id}: {e}")))?;
        let entry: PageCacheEntry = serde_json::from_str(&json)
            .map_err(|e| Error::Storage(format!("Failed to parse page {id}: {e}")))?;
        Ok(entry)
    }

    /// Check if a page exists in the cache.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use blz_core::page_cache::{PageCacheStorage, PageId};
    /// use tempfile::TempDir;
    ///
    /// let temp = TempDir::new().unwrap();
    /// let storage = PageCacheStorage::new(temp.path());
    ///
    /// let id = PageId::from_url("https://example.com/page");
    /// if storage.page_exists("docs", &id) {
    ///     println!("Page is cached");
    /// }
    /// ```
    #[must_use]
    pub fn page_exists(&self, alias: &str, id: &PageId) -> bool {
        self.page_path(alias, id).exists()
    }

    /// Delete a page from the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the page exists but cannot be deleted.
    /// Does not error if the page doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use blz_core::page_cache::{PageCacheStorage, PageId};
    /// use tempfile::TempDir;
    ///
    /// let temp = TempDir::new().unwrap();
    /// let storage = PageCacheStorage::new(temp.path());
    ///
    /// let id = PageId::from_url("https://example.com/page");
    /// storage.delete_page("docs", &id).unwrap();
    /// ```
    pub fn delete_page(&self, alias: &str, id: &PageId) -> Result<()> {
        let path = self.page_path(alias, id);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| Error::Storage(format!("Failed to delete page {id}: {e}")))?;
            debug!("Deleted page {} for {}", id, alias);
        }
        Ok(())
    }

    /// List all cached pages for a source.
    ///
    /// Returns an empty vector if the source has no cached pages
    /// or the pages directory doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if pages exist but cannot be read.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use blz_core::page_cache::PageCacheStorage;
    /// use tempfile::TempDir;
    ///
    /// let temp = TempDir::new().unwrap();
    /// let storage = PageCacheStorage::new(temp.path());
    ///
    /// let pages = storage.list_pages("docs").unwrap();
    /// println!("Found {} cached pages", pages.len());
    /// ```
    pub fn list_pages(&self, alias: &str) -> Result<Vec<PageCacheEntry>> {
        let pages_dir = self.pages_dir(alias);
        if !pages_dir.exists() {
            return Ok(Vec::new());
        }

        let mut pages = Vec::new();
        let entries = fs::read_dir(&pages_dir)
            .map_err(|e| Error::Storage(format!("Failed to read pages directory: {e}")))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| Error::Storage(format!("Failed to read directory entry: {e}")))?;
            let path = entry.path();

            // Skip non-JSON files and temp files
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let json = fs::read_to_string(&path)
                .map_err(|e| Error::Storage(format!("Failed to read page file: {e}")))?;
            let page: PageCacheEntry = serde_json::from_str(&json)
                .map_err(|e| Error::Storage(format!("Failed to parse page file: {e}")))?;
            pages.push(page);
        }

        Ok(pages)
    }

    /// Get page count without loading all pages.
    ///
    /// More efficient than `list_pages().len()` for large caches.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory exists but cannot be read.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use blz_core::page_cache::PageCacheStorage;
    /// use tempfile::TempDir;
    ///
    /// let temp = TempDir::new().unwrap();
    /// let storage = PageCacheStorage::new(temp.path());
    ///
    /// let count = storage.page_count("docs").unwrap();
    /// println!("Source has {} cached pages", count);
    /// ```
    pub fn page_count(&self, alias: &str) -> Result<usize> {
        let pages_dir = self.pages_dir(alias);
        if !pages_dir.exists() {
            return Ok(0);
        }

        let entries = fs::read_dir(&pages_dir)
            .map_err(|e| Error::Storage(format!("Failed to read pages directory: {e}")))?;

        let count = entries
            .filter_map(std::result::Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|ext| ext == "json")
            })
            .count();

        Ok(count)
    }

    // === Failed Pages ===

    /// Save failed pages for retry tracking.
    ///
    /// Overwrites any existing failed pages file.
    ///
    /// # Errors
    ///
    /// Returns an error if the failed pages cannot be written.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use blz_core::page_cache::{PageCacheStorage, FailedPage};
    /// use tempfile::TempDir;
    ///
    /// let temp = TempDir::new().unwrap();
    /// let storage = PageCacheStorage::new(temp.path());
    ///
    /// let failed = vec![
    ///     FailedPage::new("https://example.com/broken".to_string(), "timeout".to_string()),
    /// ];
    /// storage.save_failed_pages("docs", &failed).unwrap();
    /// ```
    pub fn save_failed_pages(&self, alias: &str, failed: &[FailedPage]) -> Result<()> {
        let cache_dir = self.cache_dir(alias);
        fs::create_dir_all(&cache_dir)
            .map_err(|e| Error::Storage(format!("Failed to create cache directory: {e}")))?;

        let path = self.failed_pages_path(alias);
        let json = serde_json::to_string_pretty(failed)
            .map_err(|e| Error::Storage(format!("Failed to serialize failed pages: {e}")))?;

        // Atomic write
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, json)
            .map_err(|e| Error::Storage(format!("Failed to write failed pages: {e}")))?;

        #[cfg(target_os = "windows")]
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                Error::Storage(format!("Failed to remove existing failed pages: {e}"))
            })?;
        }

        fs::rename(&tmp_path, &path)
            .map_err(|e| Error::Storage(format!("Failed to commit failed pages: {e}")))?;

        debug!("Saved {} failed pages for {}", failed.len(), alias);
        Ok(())
    }

    /// Load failed pages for a source.
    ///
    /// Returns an empty vector if no failed pages file exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use blz_core::page_cache::PageCacheStorage;
    /// use tempfile::TempDir;
    ///
    /// let temp = TempDir::new().unwrap();
    /// let storage = PageCacheStorage::new(temp.path());
    ///
    /// let failed = storage.load_failed_pages("docs").unwrap();
    /// for page in &failed {
    ///     if page.should_retry() {
    ///         println!("Should retry: {}", page.url);
    ///     }
    /// }
    /// ```
    pub fn load_failed_pages(&self, alias: &str) -> Result<Vec<FailedPage>> {
        let path = self.failed_pages_path(alias);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let json = fs::read_to_string(&path)
            .map_err(|e| Error::Storage(format!("Failed to read failed pages: {e}")))?;
        let failed: Vec<FailedPage> = serde_json::from_str(&json)
            .map_err(|e| Error::Storage(format!("Failed to parse failed pages: {e}")))?;

        Ok(failed)
    }

    /// Clear failed pages for a source.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be deleted.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use blz_core::page_cache::PageCacheStorage;
    /// use tempfile::TempDir;
    ///
    /// let temp = TempDir::new().unwrap();
    /// let storage = PageCacheStorage::new(temp.path());
    ///
    /// storage.clear_failed_pages("docs").unwrap();
    /// ```
    pub fn clear_failed_pages(&self, alias: &str) -> Result<()> {
        let path = self.failed_pages_path(alias);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| Error::Storage(format!("Failed to clear failed pages: {e}")))?;
            debug!("Cleared failed pages for {}", alias);
        }
        Ok(())
    }

    // === Backup ===

    /// Backup all pages to a timestamped subdirectory.
    ///
    /// Creates a backup of all cached pages with a manifest file
    /// containing metadata about the backup.
    ///
    /// # Errors
    ///
    /// Returns an error if the backup directory cannot be created
    /// or files cannot be copied.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use blz_core::page_cache::PageCacheStorage;
    /// use tempfile::TempDir;
    ///
    /// let temp = TempDir::new().unwrap();
    /// let storage = PageCacheStorage::new(temp.path());
    ///
    /// let backup = storage.backup_pages("docs", "pre-upgrade").unwrap();
    /// println!("Backed up {} pages to {}", backup.page_count, backup.path);
    /// ```
    pub fn backup_pages(&self, alias: &str, reason: &str) -> Result<BackupInfo> {
        let pages_dir = self.pages_dir(alias);
        let page_count = self.page_count(alias)?;

        // Create timestamped backup directory
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_dir_name = format!("pages.bak.{timestamp}");
        let backup_dir = self.cache_dir(alias).join(&backup_dir_name);

        fs::create_dir_all(&backup_dir)
            .map_err(|e| Error::Storage(format!("Failed to create backup directory: {e}")))?;

        // Copy all page files
        if pages_dir.exists() {
            for entry in fs::read_dir(&pages_dir)
                .map_err(|e| Error::Storage(format!("Failed to read pages directory: {e}")))?
            {
                let entry = entry
                    .map_err(|e| Error::Storage(format!("Failed to read directory entry: {e}")))?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let dest = backup_dir.join(entry.file_name());
                    fs::copy(&path, &dest).map_err(|e| {
                        Error::Storage(format!("Failed to copy page to backup: {e}"))
                    })?;
                }
            }
        }

        // Create backup manifest
        let backup_info = BackupInfo {
            backed_up_at: Utc::now(),
            reason: reason.to_string(),
            page_count,
            path: backup_dir_name.clone(),
        };

        let manifest_path = backup_dir.join("index.json");
        let manifest_json = serde_json::to_string_pretty(&backup_info)
            .map_err(|e| Error::Storage(format!("Failed to serialize backup manifest: {e}")))?;
        fs::write(&manifest_path, manifest_json)
            .map_err(|e| Error::Storage(format!("Failed to write backup manifest: {e}")))?;

        debug!(
            "Created backup of {} pages for {} at {}",
            page_count, alias, backup_dir_name
        );
        Ok(backup_info)
    }

    /// Clear all cached pages (but not backups).
    ///
    /// Removes all page files from the pages directory.
    /// Backups and failed pages are preserved.
    ///
    /// # Errors
    ///
    /// Returns an error if pages exist but cannot be deleted.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use blz_core::page_cache::PageCacheStorage;
    /// use tempfile::TempDir;
    ///
    /// let temp = TempDir::new().unwrap();
    /// let storage = PageCacheStorage::new(temp.path());
    ///
    /// storage.clear_pages("docs").unwrap();
    /// assert_eq!(storage.page_count("docs").unwrap(), 0);
    /// ```
    pub fn clear_pages(&self, alias: &str) -> Result<()> {
        let pages_dir = self.pages_dir(alias);
        if pages_dir.exists() {
            fs::remove_dir_all(&pages_dir)
                .map_err(|e| Error::Storage(format!("Failed to clear pages directory: {e}")))?;
            debug!("Cleared pages for {}", alias);
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_entry(url: &str) -> PageCacheEntry {
        PageCacheEntry::new(url.to_string(), "# Test\n\nContent".to_string())
    }

    #[test]
    fn test_save_and_load_page() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        let entry = create_test_entry("https://example.com/page");
        storage.save_page("test", &entry).unwrap();

        let loaded = storage.load_page("test", &entry.id).unwrap();
        assert_eq!(entry.id, loaded.id);
        assert_eq!(entry.markdown, loaded.markdown);
        assert_eq!(entry.url, loaded.url);
    }

    #[test]
    fn test_page_exists() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        let entry = create_test_entry("https://example.com/page");
        assert!(!storage.page_exists("test", &entry.id));

        storage.save_page("test", &entry).unwrap();
        assert!(storage.page_exists("test", &entry.id));
    }

    #[test]
    fn test_delete_page() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        let entry = create_test_entry("https://example.com/page");
        storage.save_page("test", &entry).unwrap();
        assert!(storage.page_exists("test", &entry.id));

        storage.delete_page("test", &entry.id).unwrap();
        assert!(!storage.page_exists("test", &entry.id));
    }

    #[test]
    fn test_delete_nonexistent_page() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        let id = PageId::from_url("https://example.com/nonexistent");

        // Should not error when deleting nonexistent page
        let result = storage.delete_page("test", &id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_pages() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        storage
            .save_page("test", &create_test_entry("https://example.com/1"))
            .unwrap();
        storage
            .save_page("test", &create_test_entry("https://example.com/2"))
            .unwrap();
        storage
            .save_page("test", &create_test_entry("https://example.com/3"))
            .unwrap();

        let pages = storage.list_pages("test").unwrap();
        assert_eq!(pages.len(), 3);
    }

    #[test]
    fn test_list_pages_empty() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        let pages = storage.list_pages("nonexistent").unwrap();
        assert!(pages.is_empty());
    }

    #[test]
    fn test_page_count() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        assert_eq!(storage.page_count("test").unwrap(), 0);

        storage
            .save_page("test", &create_test_entry("https://example.com/1"))
            .unwrap();
        storage
            .save_page("test", &create_test_entry("https://example.com/2"))
            .unwrap();

        assert_eq!(storage.page_count("test").unwrap(), 2);
    }

    #[test]
    fn test_failed_pages() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        let failed = vec![
            FailedPage::new("https://example.com/1".to_string(), "timeout".to_string()),
            FailedPage::new("https://example.com/2".to_string(), "404".to_string()),
        ];

        storage.save_failed_pages("test", &failed).unwrap();
        let loaded = storage.load_failed_pages("test").unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].url, "https://example.com/1");
        assert_eq!(loaded[0].error, "timeout");
        assert_eq!(loaded[1].url, "https://example.com/2");
        assert_eq!(loaded[1].error, "404");
    }

    #[test]
    fn test_load_failed_pages_empty() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        let loaded = storage.load_failed_pages("nonexistent").unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_clear_failed_pages() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        let failed = vec![FailedPage::new(
            "https://example.com".to_string(),
            "error".to_string(),
        )];
        storage.save_failed_pages("test", &failed).unwrap();

        storage.clear_failed_pages("test").unwrap();
        let loaded = storage.load_failed_pages("test").unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_backup_pages() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        storage
            .save_page("test", &create_test_entry("https://example.com/1"))
            .unwrap();
        storage
            .save_page("test", &create_test_entry("https://example.com/2"))
            .unwrap();

        let backup = storage.backup_pages("test", "pre-upgrade").unwrap();
        assert_eq!(backup.page_count, 2);
        assert_eq!(backup.reason, "pre-upgrade");

        // Verify backup directory exists
        let backup_path = temp.path().join("sources/test/.cache").join(&backup.path);
        assert!(backup_path.exists());

        // Verify manifest exists
        let manifest_path = backup_path.join("index.json");
        assert!(manifest_path.exists());

        // Verify page files were copied
        let backup_entries: Vec<_> = fs::read_dir(&backup_path)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .collect();
        // 2 pages + 1 manifest
        assert_eq!(backup_entries.len(), 3);
    }

    #[test]
    fn test_backup_empty_source() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        let backup = storage.backup_pages("empty", "test").unwrap();
        assert_eq!(backup.page_count, 0);
    }

    #[test]
    fn test_clear_pages() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        storage
            .save_page("test", &create_test_entry("https://example.com/1"))
            .unwrap();
        storage
            .save_page("test", &create_test_entry("https://example.com/2"))
            .unwrap();

        storage.clear_pages("test").unwrap();
        assert_eq!(storage.page_count("test").unwrap(), 0);
    }

    #[test]
    fn test_clear_pages_preserves_backups() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        storage
            .save_page("test", &create_test_entry("https://example.com/1"))
            .unwrap();

        let backup = storage.backup_pages("test", "test").unwrap();
        storage.clear_pages("test").unwrap();

        // Verify backup still exists
        let backup_path = temp.path().join("sources/test/.cache").join(&backup.path);
        assert!(backup_path.exists());
    }

    #[test]
    fn test_atomic_write_creates_no_orphan_tmp() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        let entry = create_test_entry("https://example.com/page");
        storage.save_page("test", &entry).unwrap();

        // Verify no .tmp files remain
        let pages_dir = temp.path().join("sources/test/.cache/pages");
        let tmp_files: Vec<_> = fs::read_dir(&pages_dir)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|ext| ext == "tmp")
            })
            .collect();

        assert!(tmp_files.is_empty(), "No .tmp files should remain");
    }

    #[test]
    fn test_isolation_between_aliases() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        storage
            .save_page(
                "source1",
                &create_test_entry("https://example.com/source1/page"),
            )
            .unwrap();
        storage
            .save_page(
                "source2",
                &create_test_entry("https://example.com/source2/page"),
            )
            .unwrap();

        assert_eq!(storage.page_count("source1").unwrap(), 1);
        assert_eq!(storage.page_count("source2").unwrap(), 1);

        // Clear one source shouldn't affect the other
        storage.clear_pages("source1").unwrap();
        assert_eq!(storage.page_count("source1").unwrap(), 0);
        assert_eq!(storage.page_count("source2").unwrap(), 1);
    }

    #[test]
    fn test_overwrite_existing_page() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        let url = "https://example.com/page";
        let entry1 = PageCacheEntry::new(url.to_string(), "# Version 1".to_string());
        storage.save_page("test", &entry1).unwrap();

        let entry2 = PageCacheEntry::new(url.to_string(), "# Version 2".to_string());
        storage.save_page("test", &entry2).unwrap();

        let loaded = storage.load_page("test", &entry1.id).unwrap();
        assert_eq!(loaded.markdown, "# Version 2");

        // Should still be just one page
        assert_eq!(storage.page_count("test").unwrap(), 1);
    }

    #[test]
    fn test_page_serialization_preserves_all_fields() {
        let temp = TempDir::new().unwrap();
        let storage = PageCacheStorage::new(temp.path());

        let lastmod = Utc::now();
        let entry = PageCacheEntry::new(
            "https://example.com/page".to_string(),
            "# Title\n\nBody content".to_string(),
        )
        .with_title(Some("Title".to_string()))
        .with_section(Some("Getting Started".to_string()))
        .with_lastmod(Some(lastmod));

        storage.save_page("test", &entry).unwrap();
        let loaded = storage.load_page("test", &entry.id).unwrap();

        assert_eq!(loaded.id, entry.id);
        assert_eq!(loaded.url, entry.url);
        assert_eq!(loaded.title, Some("Title".to_string()));
        assert_eq!(loaded.section, Some("Getting Started".to_string()));
        assert_eq!(loaded.markdown, entry.markdown);
        assert_eq!(loaded.line_count, 3);
        assert!(loaded.sitemap_lastmod.is_some());
    }
}
