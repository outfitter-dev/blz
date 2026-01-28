//! Generate command orchestration for creating llms-full.txt files.
//!
//! This module provides the orchestration layer for the `blz generate` command,
//! which scrapes discovered URLs via Firecrawl and assembles them into a
//! complete llms-full.txt file.
//!
//! ## Key Components
//!
//! - [`GenerateOrchestrator`]: Coordinates parallel scraping with adaptive concurrency
//! - [`UrlWithLastmod`]: URL with optional lastmod for change detection
//! - [`ScrapeResults`]: Aggregated results from scraping operations
//!
//! ## Example
//!
//! ```rust,no_run
//! use blz_cli::generate::{GenerateOrchestrator, UrlWithLastmod, ScrapeResults};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // URLs would come from sitemap discovery
//! let urls = vec![
//!     UrlWithLastmod::new("https://example.com/docs/intro".to_string()),
//!     UrlWithLastmod::new("https://example.com/docs/api".to_string()),
//! ];
//!
//! // Create orchestrator (would use real FirecrawlCli)
//! // let cli = FirecrawlCli::detect().await?;
//! // let orchestrator = GenerateOrchestrator::new(cli, 5)
//! //     .with_progress(|completed, total| {
//! //         println!("Progress: {}/{}", completed, total);
//! //     });
//! //
//! // let results = orchestrator.scrape_all(&urls).await;
//! // println!("Successful: {}, Failed: {}", results.successful.len(), results.failed.len());
//! # Ok(())
//! # }
//! ```

mod orchestrator;

pub use orchestrator::{
    GenerateOrchestrator, ProgressCallback, ScrapeError, ScrapeResult, ScrapeResults, Scraper,
    UrlWithLastmod,
};
