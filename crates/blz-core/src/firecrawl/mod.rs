//! Firecrawl CLI integration for web documentation scraping.
//!
//! This module provides detection and integration with the Firecrawl CLI tool,
//! which enables BLZ to scrape web documentation when sites don't provide native
//! llms-full.txt files.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use blz_core::firecrawl::{detect_firecrawl, FirecrawlStatus};
//!
//! # async fn example() -> blz_core::Result<()> {
//! match detect_firecrawl().await {
//!     FirecrawlStatus::Ready { version, path } => {
//!         println!("Firecrawl {} ready at {}", version, path);
//!     }
//!     FirecrawlStatus::VersionTooOld { found, required, .. } => {
//!         println!("Firecrawl {} is too old, need {}", found, required);
//!     }
//!     FirecrawlStatus::NotAuthenticated { .. } => {
//!         println!("Firecrawl not authenticated, run 'firecrawl login'");
//!     }
//!     FirecrawlStatus::NotInstalled => {
//!         println!("Firecrawl not found, install from https://firecrawl.dev");
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod detect;

pub use detect::{FirecrawlCli, FirecrawlStatus, detect_firecrawl};

/// Minimum required version of Firecrawl CLI.
///
/// Features used by BLZ require at least this version.
pub const MIN_VERSION: &str = "1.1.0";
