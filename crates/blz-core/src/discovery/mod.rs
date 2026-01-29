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
//!
//! ## Smart URL Resolution
//!
//! The [`probe_url`] function provides smarter resolution when given a URL with a path:
//!
//! 1. Check Link headers on the URL for `rel="llms-txt"` or `rel="llms-full-txt"`
//! 2. Probe path-relative locations (e.g., `/docs/llms-full.txt`)
//! 3. Probe the host root
//! 4. Try docs.* subdomain
//! 5. Suggest parent domain (requires user confirmation)
//!
//! ```no_run
//! use blz_core::discovery::probe_url;
//!
//! # async fn example() -> blz_core::Result<()> {
//! // Smart resolution finds llms-full.txt via Link header or path probing
//! let result = probe_url("https://code.claude.com/docs").await?;
//!
//! if result.requires_confirmation {
//!     println!("Found at different scope - please confirm");
//! }
//! # Ok(())
//! # }
//! ```

pub mod alias;
pub mod probe;
pub mod sitemap;

pub use alias::{
    AliasDerivation, derive_alias, derive_alias_with_collision_check, has_collision, is_valid_alias,
};
pub use probe::{DiscoveryMethod, ProbeResult, probe_domain, probe_url};
pub use sitemap::{ChangeFrequency, SitemapEntry, fetch_sitemap, is_sitemap_index, parse_sitemap};
