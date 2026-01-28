//! Documentation source discovery for domains.
//!
//! This module provides functionality to automatically discover documentation
//! sources on domains, enabling workflows like `blz add hono.dev` where the
//! system probes for llms.txt, llms-full.txt, and sitemap.xml.
//!
//! ## Quick Start
//!
//! ```no_run
//! use blz_core::discovery::probe_domain;
//!
//! # async fn example() -> blz_core::Result<()> {
//! // Probe a domain for documentation sources
//! let result = probe_domain("hono.dev").await?;
//!
//! if let Some(url) = result.best_url() {
//!     println!("Best documentation source: {}", url);
//! }
//!
//! // Check what was found
//! if result.llms_full_url.is_some() {
//!     println!("Found complete docs at llms-full.txt");
//! }
//! if result.llms_url.is_some() {
//!     println!("Found index at llms.txt");
//! }
//! if result.sitemap_url.is_some() {
//!     println!("Found sitemap for URL discovery");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Probe Order
//!
//! The [`probe_domain`] function checks URLs in this order:
//!
//! 1. `https://{domain}/llms-full.txt` - Complete documentation (preferred)
//! 2. `https://{domain}/llms.txt` - Documentation index
//! 3. `https://{domain}/sitemap.xml` - URL discovery fallback
//! 4. `https://docs.{domain}/*` - Subdomain fallback if main domain has nothing

pub mod alias;
pub mod extract;
pub mod filter;
pub mod probe;
pub mod sitemap;

pub use alias::{
    AliasDerivation, derive_alias, derive_alias_with_collision_check, has_collision, is_valid_alias,
};
pub use extract::{DiscoveredUrl, UrlSource, extract_urls, merge_url_sources};
pub use filter::{filter_to_docs, filter_to_domain, is_likely_docs_path};
pub use probe::{ProbeResult, probe_domain};
pub use sitemap::{ChangeFrequency, SitemapEntry, fetch_sitemap, is_sitemap_index, parse_sitemap};
