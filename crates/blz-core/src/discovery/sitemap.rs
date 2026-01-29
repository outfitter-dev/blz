//! Sitemap XML parsing for documentation source discovery.
//!
//! This module provides functionality to parse sitemap.xml files and extract
//! URLs with their metadata. The `lastmod` field is particularly important
//! for cost optimization, allowing sync operations to skip unchanged pages.
//!
//! ## Quick Start
//!
//! ```no_run
//! use blz_core::discovery::sitemap::{parse_sitemap, fetch_sitemap};
//!
//! # async fn example() -> blz_core::Result<()> {
//! // Parse sitemap XML directly
//! let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
//! <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
//!   <url>
//!     <loc>https://example.com/page1</loc>
//!     <lastmod>2024-01-15</lastmod>
//!   </url>
//! </urlset>"#;
//!
//! let entries = parse_sitemap(xml)?;
//! println!("Found {} URLs", entries.len());
//!
//! // Or fetch and parse from URL (handles sitemap indices)
//! let entries = fetch_sitemap("https://example.com/sitemap.xml").await?;
//! for entry in entries {
//!     println!("{} - last modified: {:?}", entry.url, entry.lastmod);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Sitemap Formats
//!
//! Supports both standard sitemaps and sitemap index files:
//!
//! - **Standard sitemap**: Contains `<urlset>` with `<url>` entries
//! - **Sitemap index**: Contains `<sitemapindex>` with `<sitemap>` entries
//!   pointing to other sitemaps (recursively fetched)

use crate::{Error, Result};
use chrono::{DateTime, NaiveDate, Utc};
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::instrument;

/// Default timeout for sitemap fetch requests.
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum recursion depth for sitemap index files.
const MAX_INDEX_DEPTH: u8 = 2;

/// Maximum number of child sitemaps to fetch from an index.
const MAX_CHILD_SITEMAPS: usize = 50;

/// A single entry from a sitemap.
///
/// Contains the URL and optional metadata like last modification date,
/// change frequency, and priority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SitemapEntry {
    /// The URL of the page.
    pub url: String,
    /// Last modification date (critical for change detection).
    pub lastmod: Option<DateTime<Utc>>,
    /// How frequently the page changes.
    pub changefreq: Option<ChangeFrequency>,
    /// Priority of this URL relative to others (0.0 to 1.0).
    pub priority: Option<f32>,
}

/// Change frequency hints from sitemap.
///
/// These values indicate how frequently a page is likely to change,
/// though search engines may not follow these hints strictly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeFrequency {
    /// The page changes every time it is accessed.
    Always,
    /// The page changes hourly.
    Hourly,
    /// The page changes daily.
    Daily,
    /// The page changes weekly.
    Weekly,
    /// The page changes monthly.
    Monthly,
    /// The page changes yearly.
    Yearly,
    /// The page is archived and will not change.
    Never,
}

impl std::str::FromStr for ChangeFrequency {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "always" => Ok(Self::Always),
            "hourly" => Ok(Self::Hourly),
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "monthly" => Ok(Self::Monthly),
            "yearly" => Ok(Self::Yearly),
            "never" => Ok(Self::Never),
            _ => Err(Error::Parse(format!("Invalid changefreq value: {s}"))),
        }
    }
}

/// Result of parsing a sitemap - either entries or a sitemap index.
#[derive(Debug)]
enum SitemapContent {
    /// Standard sitemap with URL entries.
    Entries(Vec<SitemapEntry>),
    /// Sitemap index with URLs to child sitemaps.
    Index(Vec<SitemapIndexEntry>),
}

/// An entry in a sitemap index file.
#[derive(Debug)]
struct SitemapIndexEntry {
    /// URL to the child sitemap.
    loc: String,
    /// Last modification date of the child sitemap.
    #[allow(dead_code)]
    lastmod: Option<DateTime<Utc>>,
}

/// Parse a sitemap XML string into entries.
///
/// Handles the standard sitemap format with `<urlset>` containing `<url>` entries.
/// Returns an error if the XML is a sitemap index (use [`fetch_sitemap`] for indices).
///
/// # Arguments
///
/// * `xml` - The sitemap XML content to parse.
///
/// # Returns
///
/// A vector of [`SitemapEntry`] instances parsed from the XML.
///
/// # Errors
///
/// Returns an error if:
/// - The XML is malformed
/// - The XML is a sitemap index (contains `<sitemapindex>`)
/// - Required `<loc>` elements are missing
///
/// # Examples
///
/// ```
/// use blz_core::discovery::sitemap::parse_sitemap;
///
/// let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
/// <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
///   <url>
///     <loc>https://example.com/page1</loc>
///     <lastmod>2024-01-15</lastmod>
///   </url>
/// </urlset>"#;
///
/// let entries = parse_sitemap(xml).unwrap();
/// assert_eq!(entries.len(), 1);
/// assert_eq!(entries[0].url, "https://example.com/page1");
/// ```
#[instrument(skip(xml), fields(xml_len = xml.len()))]
pub fn parse_sitemap(xml: &str) -> Result<Vec<SitemapEntry>> {
    match parse_sitemap_content(xml)? {
        SitemapContent::Entries(entries) => Ok(entries),
        SitemapContent::Index(_) => Err(Error::Parse(
            "XML is a sitemap index, not a standard sitemap. Use fetch_sitemap() for indices."
                .to_string(),
        )),
    }
}

/// Check if the XML content is a sitemap index.
///
/// # Arguments
///
/// * `xml` - The XML content to check.
///
/// # Returns
///
/// `true` if the XML contains a `<sitemapindex>` element.
#[must_use]
pub fn is_sitemap_index(xml: &str) -> bool {
    xml.contains("<sitemapindex") || xml.contains("sitemapindex>")
}

/// Fetch and parse a sitemap from URL.
///
/// Handles sitemap index files recursively, fetching all child sitemaps
/// and merging their entries. Limits recursion depth and number of child
/// sitemaps to prevent abuse.
///
/// # Arguments
///
/// * `url` - The URL of the sitemap to fetch.
///
/// # Returns
///
/// A vector of [`SitemapEntry`] instances from the sitemap (and all child
/// sitemaps if the URL points to a sitemap index).
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP request fails
/// - The response is not valid sitemap XML
/// - Recursion depth exceeds the limit
///
/// # Examples
///
/// ```no_run
/// use blz_core::discovery::sitemap::fetch_sitemap;
///
/// # async fn example() -> blz_core::Result<()> {
/// let entries = fetch_sitemap("https://example.com/sitemap.xml").await?;
/// for entry in entries {
///     println!("{}", entry.url);
/// }
/// # Ok(())
/// # }
/// ```
#[instrument(skip_all, fields(url = %url))]
pub async fn fetch_sitemap(url: &str) -> Result<Vec<SitemapEntry>> {
    let client = build_sitemap_client()?;
    fetch_sitemap_recursive(client, url.to_string(), 0).await
}

/// Internal recursive fetcher for sitemap content.
///
/// Uses `Box::pin` to make the recursive future `Send`-compatible for `tokio::spawn`.
/// Takes owned values to avoid lifetime issues with the recursive call.
fn fetch_sitemap_recursive(
    client: Client,
    url: String,
    depth: u8,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<SitemapEntry>>> + Send>> {
    Box::pin(async move {
        if depth > MAX_INDEX_DEPTH {
            return Err(Error::ResourceLimited(format!(
                "Sitemap index recursion depth exceeded (max: {MAX_INDEX_DEPTH})"
            )));
        }

        tracing::debug!(url = %url, depth = depth, "Fetching sitemap");

        let response = client.get(&url).send().await.map_err(Error::Network)?;

        let response = response.error_for_status().map_err(Error::Network)?;

        let xml = response.text().await.map_err(Error::Network)?;

        match parse_sitemap_content(&xml)? {
            SitemapContent::Entries(entries) => Ok(entries),
            SitemapContent::Index(index_entries) => {
                // Fetch child sitemaps in parallel (limited count)
                let child_urls: Vec<_> = index_entries
                    .into_iter()
                    .take(MAX_CHILD_SITEMAPS)
                    .map(|e| e.loc)
                    .collect();

                tracing::debug!(
                    child_count = child_urls.len(),
                    "Fetching child sitemaps from index"
                );

                let mut all_entries = Vec::new();
                let mut handles = Vec::new();

                for child_url in child_urls {
                    let client_clone = client.clone();
                    let handle =
                        tokio::spawn(fetch_sitemap_recursive(client_clone, child_url, depth + 1));
                    handles.push(handle);
                }

                for handle in handles {
                    match handle.await {
                        Ok(Ok(entries)) => all_entries.extend(entries),
                        Ok(Err(e)) => {
                            tracing::warn!(error = %e, "Failed to fetch child sitemap");
                            // Continue with other sitemaps instead of failing entirely
                        },
                        Err(e) => {
                            tracing::warn!(error = %e, "Child sitemap fetch task panicked");
                        },
                    }
                }

                Ok(all_entries)
            },
        }
    })
}

/// Parse sitemap content and detect whether it's a standard sitemap or index.
fn parse_sitemap_content(xml: &str) -> Result<SitemapContent> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    // First, detect the root element
    let is_index = is_sitemap_index(xml);

    if is_index {
        parse_sitemap_index(&mut reader)
    } else {
        parse_urlset(&mut reader)
    }
}

/// Parse a standard sitemap with `<urlset>` root.
fn parse_urlset(reader: &mut Reader<&[u8]>) -> Result<SitemapContent> {
    let mut entries = Vec::new();
    let mut buf = Vec::new();

    // State for parsing current URL entry
    let mut current_url: Option<String> = None;
    let mut current_lastmod: Option<DateTime<Utc>> = None;
    let mut current_changefreq: Option<ChangeFrequency> = None;
    let mut current_priority: Option<f32> = None;
    let mut in_url = false;
    let mut current_element: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match name.as_str() {
                    "url" => {
                        in_url = true;
                        current_url = None;
                        current_lastmod = None;
                        current_changefreq = None;
                        current_priority = None;
                    },
                    "loc" | "lastmod" | "changefreq" | "priority" if in_url => {
                        current_element = Some(name);
                    },
                    _ => {},
                }
            },
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                if name == "url" && in_url {
                    if let Some(url) = current_url.take() {
                        entries.push(SitemapEntry {
                            url,
                            lastmod: current_lastmod.take(),
                            changefreq: current_changefreq.take(),
                            priority: current_priority.take(),
                        });
                    }
                    in_url = false;
                }
                current_element = None;
            },
            Ok(Event::Text(e)) => {
                if let Some(ref element) = current_element {
                    let text = e.unescape().map_err(|e| Error::Parse(e.to_string()))?;
                    let text = text.trim();

                    match element.as_str() {
                        "loc" => current_url = Some(text.to_string()),
                        "lastmod" => current_lastmod = parse_lastmod(text),
                        "changefreq" => current_changefreq = text.parse().ok(),
                        "priority" => current_priority = parse_priority(text),
                        _ => {},
                    }
                }
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Parse(format!("XML parse error: {e}"))),
            _ => {},
        }
        buf.clear();
    }

    Ok(SitemapContent::Entries(entries))
}

/// Parse a sitemap index with `<sitemapindex>` root.
fn parse_sitemap_index(reader: &mut Reader<&[u8]>) -> Result<SitemapContent> {
    let mut entries = Vec::new();
    let mut buf = Vec::new();

    let mut current_loc: Option<String> = None;
    let mut current_lastmod: Option<DateTime<Utc>> = None;
    let mut in_sitemap = false;
    let mut current_element: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match name.as_str() {
                    "sitemap" => {
                        in_sitemap = true;
                        current_loc = None;
                        current_lastmod = None;
                    },
                    "loc" | "lastmod" if in_sitemap => {
                        current_element = Some(name);
                    },
                    _ => {},
                }
            },
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                if name == "sitemap" && in_sitemap {
                    if let Some(loc) = current_loc.take() {
                        entries.push(SitemapIndexEntry {
                            loc,
                            lastmod: current_lastmod.take(),
                        });
                    }
                    in_sitemap = false;
                }
                current_element = None;
            },
            Ok(Event::Text(e)) => {
                if let Some(ref element) = current_element {
                    let text = e.unescape().map_err(|e| Error::Parse(e.to_string()))?;
                    let text = text.trim();

                    match element.as_str() {
                        "loc" => current_loc = Some(text.to_string()),
                        "lastmod" => current_lastmod = parse_lastmod(text),
                        _ => {},
                    }
                }
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Parse(format!("XML parse error: {e}"))),
            _ => {},
        }
        buf.clear();
    }

    Ok(SitemapContent::Index(entries))
}

/// Parse a lastmod date string into a `DateTime<Utc>`.
///
/// Supports multiple date formats:
/// - `2024-01-15` (date only)
/// - `2024-01-15T10:30:00Z` (ISO 8601 with Z)
/// - `2024-01-15T10:30:00+00:00` (ISO 8601 with offset)
/// - `2024-01-15T10:30:00.000Z` (with milliseconds)
fn parse_lastmod(s: &str) -> Option<DateTime<Utc>> {
    // Try ISO 8601 with timezone first (most common)
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try date-only format
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(date.and_hms_opt(0, 0, 0)?.and_utc());
    }

    // Try ISO 8601 without timezone (assume UTC)
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc());
    }

    // Try with milliseconds but no timezone
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(dt.and_utc());
    }

    tracing::debug!(date_str = %s, "Could not parse lastmod date");
    None
}

/// Parse a priority value, clamping to 0.0-1.0 range.
fn parse_priority(s: &str) -> Option<f32> {
    s.parse::<f32>().ok().map(|p| p.clamp(0.0, 1.0))
}

/// Build an HTTP client configured for fetching sitemaps.
fn build_sitemap_client() -> Result<Client> {
    Client::builder()
        .timeout(FETCH_TIMEOUT)
        .user_agent(concat!("outfitter-blz/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(Error::Network)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_macros,
    clippy::unnecessary_wraps
)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_parses_basic_sitemap() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <loc>https://example.com/page1</loc>
            <lastmod>2024-01-15T10:30:00+00:00</lastmod>
            <changefreq>weekly</changefreq>
            <priority>0.8</priority>
          </url>
        </urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].url, "https://example.com/page1");
        assert!(entries[0].lastmod.is_some());
        assert_eq!(entries[0].changefreq, Some(ChangeFrequency::Weekly));
        assert_eq!(entries[0].priority, Some(0.8));
    }

    #[test]
    fn test_parses_multiple_urls() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <loc>https://example.com/page1</loc>
          </url>
          <url>
            <loc>https://example.com/page2</loc>
          </url>
          <url>
            <loc>https://example.com/page3</loc>
          </url>
        </urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].url, "https://example.com/page1");
        assert_eq!(entries[1].url, "https://example.com/page2");
        assert_eq!(entries[2].url, "https://example.com/page3");
    }

    #[test]
    fn test_handles_missing_optional_fields() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <loc>https://example.com/page1</loc>
          </url>
        </urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].lastmod.is_none());
        assert!(entries[0].changefreq.is_none());
        assert!(entries[0].priority.is_none());
    }

    #[test]
    fn test_handles_date_only_format() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <loc>https://example.com/page1</loc>
            <lastmod>2024-01-15</lastmod>
          </url>
        </urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        assert!(entries[0].lastmod.is_some());
        let lastmod = entries[0].lastmod.unwrap();
        assert_eq!(lastmod.format("%Y-%m-%d").to_string(), "2024-01-15");
    }

    #[test]
    fn test_handles_iso8601_with_z() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <loc>https://example.com/page1</loc>
            <lastmod>2024-01-15T10:30:00Z</lastmod>
          </url>
        </urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        assert!(entries[0].lastmod.is_some());
        let lastmod = entries[0].lastmod.unwrap();
        assert_eq!(
            lastmod.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "2024-01-15T10:30:00"
        );
    }

    #[test]
    fn test_handles_iso8601_with_offset() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <loc>https://example.com/page1</loc>
            <lastmod>2024-01-15T10:30:00+00:00</lastmod>
          </url>
        </urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        assert!(entries[0].lastmod.is_some());
    }

    #[test]
    fn test_handles_iso8601_with_milliseconds() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <loc>https://example.com/page1</loc>
            <lastmod>2024-01-15T10:30:00.123Z</lastmod>
          </url>
        </urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        assert!(entries[0].lastmod.is_some());
    }

    #[test]
    fn test_detects_sitemap_index() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <sitemap>
            <loc>https://example.com/sitemap-1.xml</loc>
          </sitemap>
        </sitemapindex>"#;

        // Should detect this is an index
        assert!(is_sitemap_index(xml));

        // parse_sitemap should return error for index
        let result = parse_sitemap(xml);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("sitemap index"));
    }

    #[test]
    fn test_changefreq_parsing() {
        let test_cases = [
            ("always", ChangeFrequency::Always),
            ("hourly", ChangeFrequency::Hourly),
            ("daily", ChangeFrequency::Daily),
            ("weekly", ChangeFrequency::Weekly),
            ("monthly", ChangeFrequency::Monthly),
            ("yearly", ChangeFrequency::Yearly),
            ("never", ChangeFrequency::Never),
            // Case insensitive
            ("WEEKLY", ChangeFrequency::Weekly),
            ("Weekly", ChangeFrequency::Weekly),
        ];

        for (value, expected) in test_cases {
            let result: Result<ChangeFrequency> = value.parse();
            assert!(result.is_ok(), "Failed to parse: {value}");
            assert_eq!(result.unwrap(), expected);
        }
    }

    #[test]
    fn test_changefreq_invalid_value() {
        let result: Result<ChangeFrequency> = "invalid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_priority_validation() {
        // Valid priorities
        assert_eq!(parse_priority("0.5"), Some(0.5));
        assert_eq!(parse_priority("1.0"), Some(1.0));
        assert_eq!(parse_priority("0.0"), Some(0.0));

        // Out of range values should be clamped
        assert_eq!(parse_priority("1.5"), Some(1.0));
        assert_eq!(parse_priority("-0.5"), Some(0.0));

        // Invalid values
        assert_eq!(parse_priority("not-a-number"), None);
    }

    #[test]
    fn test_handles_whitespace_in_values() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <loc>  https://example.com/page1  </loc>
            <lastmod>  2024-01-15  </lastmod>
            <priority>  0.8  </priority>
          </url>
        </urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        assert_eq!(entries[0].url, "https://example.com/page1");
        assert!(entries[0].lastmod.is_some());
        assert_eq!(entries[0].priority, Some(0.8));
    }

    #[test]
    fn test_handles_empty_sitemap() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
        </urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_handles_malformed_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <loc>https://example.com/page1
          </url>
        </urlset>"#;

        let result = parse_sitemap(xml);
        assert!(result.is_err());
    }

    #[test]
    fn test_skips_urls_without_loc() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <lastmod>2024-01-15</lastmod>
          </url>
          <url>
            <loc>https://example.com/page1</loc>
          </url>
        </urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        // Only the URL with <loc> should be included
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].url, "https://example.com/page1");
    }

    #[test]
    fn test_serialization() {
        let entry = SitemapEntry {
            url: "https://example.com/page1".to_string(),
            lastmod: Some(
                DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            changefreq: Some(ChangeFrequency::Weekly),
            priority: Some(0.8),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"url\":\"https://example.com/page1\""));
        assert!(json.contains("\"changefreq\":\"weekly\""));
        assert!(json.contains("\"priority\":0.8"));

        // Deserialize back
        let parsed: SitemapEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.url, entry.url);
        assert_eq!(parsed.changefreq, entry.changefreq);
        assert_eq!(parsed.priority, entry.priority);
    }

    // Async tests with wiremock
    #[tokio::test]
    async fn test_fetch_sitemap_basic() {
        let mock_server = MockServer::start().await;

        let sitemap_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <loc>https://example.com/page1</loc>
            <lastmod>2024-01-15</lastmod>
          </url>
        </urlset>"#;

        Mock::given(method("GET"))
            .and(path("/sitemap.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(sitemap_xml)
                    .insert_header("Content-Type", "application/xml"),
            )
            .mount(&mock_server)
            .await;

        let url = format!("{}/sitemap.xml", mock_server.uri());
        let entries = fetch_sitemap(&url).await.unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].url, "https://example.com/page1");
    }

    #[tokio::test]
    async fn test_fetch_sitemap_handles_index() {
        let mock_server = MockServer::start().await;

        // Main sitemap index
        let index_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
              <sitemap>
                <loc>{}/sitemap-1.xml</loc>
              </sitemap>
              <sitemap>
                <loc>{}/sitemap-2.xml</loc>
              </sitemap>
            </sitemapindex>"#,
            mock_server.uri(),
            mock_server.uri()
        );

        // Child sitemap 1
        let sitemap1_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url><loc>https://example.com/page1</loc></url>
          <url><loc>https://example.com/page2</loc></url>
        </urlset>"#;

        // Child sitemap 2
        let sitemap2_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url><loc>https://example.com/page3</loc></url>
        </urlset>"#;

        Mock::given(method("GET"))
            .and(path("/sitemap.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(index_xml)
                    .insert_header("Content-Type", "application/xml"),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/sitemap-1.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(sitemap1_xml)
                    .insert_header("Content-Type", "application/xml"),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/sitemap-2.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(sitemap2_xml)
                    .insert_header("Content-Type", "application/xml"),
            )
            .mount(&mock_server)
            .await;

        let url = format!("{}/sitemap.xml", mock_server.uri());
        let entries = fetch_sitemap(&url).await.unwrap();

        // Should have entries from both child sitemaps
        assert_eq!(entries.len(), 3);
        let urls: Vec<_> = entries.iter().map(|e| e.url.as_str()).collect();
        assert!(urls.contains(&"https://example.com/page1"));
        assert!(urls.contains(&"https://example.com/page2"));
        assert!(urls.contains(&"https://example.com/page3"));
    }

    #[tokio::test]
    async fn test_fetch_sitemap_handles_404() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/sitemap.xml"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let url = format!("{}/sitemap.xml", mock_server.uri());
        let result = fetch_sitemap(&url).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_sitemap_continues_on_child_failure() {
        let mock_server = MockServer::start().await;

        // Main sitemap index
        let index_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
              <sitemap>
                <loc>{}/sitemap-1.xml</loc>
              </sitemap>
              <sitemap>
                <loc>{}/sitemap-2.xml</loc>
              </sitemap>
            </sitemapindex>"#,
            mock_server.uri(),
            mock_server.uri()
        );

        // Child sitemap 1 - returns 404
        // Child sitemap 2 - returns valid content
        let sitemap2_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url><loc>https://example.com/page1</loc></url>
        </urlset>"#;

        Mock::given(method("GET"))
            .and(path("/sitemap.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(index_xml)
                    .insert_header("Content-Type", "application/xml"),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/sitemap-1.xml"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/sitemap-2.xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(sitemap2_xml)
                    .insert_header("Content-Type", "application/xml"),
            )
            .mount(&mock_server)
            .await;

        let url = format!("{}/sitemap.xml", mock_server.uri());
        let entries = fetch_sitemap(&url).await.unwrap();

        // Should have entries from the successful child sitemap
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].url, "https://example.com/page1");
    }

    #[test]
    fn test_is_sitemap_index_detection() {
        let index_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <sitemap><loc>https://example.com/sitemap-1.xml</loc></sitemap>
        </sitemapindex>"#;

        let urlset_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url><loc>https://example.com/page1</loc></url>
        </urlset>"#;

        assert!(is_sitemap_index(index_xml));
        assert!(!is_sitemap_index(urlset_xml));
    }

    #[test]
    fn test_handles_xml_entities() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <loc>https://example.com/page?foo=1&amp;bar=2</loc>
          </url>
        </urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        assert_eq!(entries[0].url, "https://example.com/page?foo=1&bar=2");
    }

    #[test]
    fn test_handles_special_characters_in_url() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url>
            <loc>https://example.com/path/to/page%20with%20spaces</loc>
          </url>
        </urlset>"#;

        let entries = parse_sitemap(xml).unwrap();
        assert_eq!(
            entries[0].url,
            "https://example.com/path/to/page%20with%20spaces"
        );
    }
}
