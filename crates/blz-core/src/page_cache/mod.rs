//! Page cache for storing scraped web content.
//!
//! This module provides types for caching web pages fetched via Firecrawl,
//! with durable identifiers for incremental updates and change detection.
//!
//! ## Key Types
//!
//! - [`PageId`]: Durable identifier derived from URL (SHA-256 prefix)
//! - [`PageCacheEntry`]: Successfully scraped page with markdown content
//! - [`FailedPage`]: Record of scraping failures with retry tracking
//!
//! ## Example
//!
//! ```rust
//! use blz_core::page_cache::{PageId, PageCacheEntry, FailedPage};
//!
//! // Create a page ID from URL
//! let id = PageId::from_url("https://example.com/docs/api");
//! assert!(id.as_str().starts_with("pg_"));
//!
//! // Create a cache entry
//! let entry = PageCacheEntry::new(
//!     "https://example.com/docs/api".to_string(),
//!     "# API Documentation\n\nContent here...".to_string(),
//! );
//! assert_eq!(entry.line_count, 3);
//!
//! // Track failed pages
//! let mut failed = FailedPage::new(
//!     "https://example.com/broken".to_string(),
//!     "timeout".to_string(),
//! );
//! assert!(failed.should_retry());
//! ```

mod storage;
mod types;

pub use storage::{BackupInfo, PageCacheStorage};
pub use types::{FailedPage, PageCacheEntry, PageId};
