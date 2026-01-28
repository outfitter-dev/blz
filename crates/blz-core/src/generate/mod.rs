//! Generation pipeline for creating llms.txt from web scraping.
//!
//! This module provides types and utilities for assembling llms.txt documents
//! from scraped web pages, tracking generation metadata, and supporting
//! incremental updates.
//!
//! ## Key Types
//!
//! - [`GenerateManifest`]: Metadata about a generated source including pages,
//!   stats, and version info for migrations
//! - [`GeneratedSourceType`]: Whether the source is generated or native
//! - [`DiscoveryInfo`]: How URLs were discovered (llms.txt, sitemap, crawl)
//! - [`PageMeta`]: Metadata about each page in the assembled document
//!
//! ## Example
//!
//! ```rust
//! use blz_core::generate::{
//!     GenerateManifest, DiscoveryInfo, UrlSourceCounts, PageMeta,
//! };
//! use blz_core::page_cache::PageId;
//!
//! let discovery = DiscoveryInfo {
//!     input: "hono.dev".to_string(),
//!     index_url: Some("https://hono.dev/llms.txt".to_string()),
//!     sitemap_url: None,
//!     url_sources: UrlSourceCounts {
//!         llms_txt: 45,
//!         sitemap: 0,
//!         crawl: 0,
//!     },
//! };
//!
//! let pages = vec![
//!     PageMeta {
//!         id: PageId::from_url("https://hono.dev/docs/getting-started"),
//!         url: "https://hono.dev/docs/getting-started".to_string(),
//!         title: Some("Getting Started".to_string()),
//!         line_range: "1-50".to_string(),
//!     },
//! ];
//!
//! let manifest = GenerateManifest::new(
//!     discovery,
//!     pages,
//!     vec![],
//!     "1.2.0".to_string(),
//! );
//!
//! assert_eq!(manifest.stats.successful_pages, 1);
//! ```

mod manifest;

pub use manifest::{
    DiscoveryInfo, GenerateManifest, GenerateStats, GeneratedSourceType, PageMeta, SCHEMA_VERSION,
    UrlSourceCounts,
};
