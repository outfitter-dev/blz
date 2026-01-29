//! URL extraction from llms.txt and other markdown content.
//!
//! This module provides functionality to extract URLs from markdown content,
//! particularly llms.txt files, and merge URLs from multiple sources while
//! tracking their origin.
//!
//! ## Quick Start
//!
//! ```rust
//! use blz_core::discovery::extract::{extract_urls, merge_url_sources, UrlSource};
//! use blz_core::discovery::SitemapEntry;
//!
//! // Extract URLs from markdown content
//! let content = r#"
//! # Documentation
//! - [Getting Started](/docs/getting-started)
//! - [API Reference](https://example.com/api)
//! "#;
//!
//! let urls = extract_urls(content, "https://example.com");
//! assert!(urls.contains(&"https://example.com/docs/getting-started".to_string()));
//!
//! // Merge URLs from different sources
//! let llms_urls = vec!["https://example.com/page1".to_string()];
//! let sitemap = vec![SitemapEntry {
//!     url: "https://example.com/page2".to_string(),
//!     lastmod: None,
//!     changefreq: None,
//!     priority: None,
//! }];
//!
//! let merged = merge_url_sources(&llms_urls, &sitemap);
//! assert_eq!(merged.len(), 2);
//! ```

use crate::discovery::SitemapEntry;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::LazyLock;
use url::Url;

/// A discovered URL with its source origin.
///
/// Tracks where a URL was discovered from, which can be useful for
/// prioritization and deduplication strategies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredUrl {
    /// The URL that was discovered.
    pub url: String,
    /// Where this URL was discovered from.
    pub source: UrlSource,
}

/// The source of a discovered URL.
///
/// Used to track where URLs were found, which can inform prioritization
/// decisions (e.g., preferring URLs explicitly listed in llms.txt).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UrlSource {
    /// URL was found in an llms.txt file.
    LlmsTxt,
    /// URL was found in a sitemap.xml file.
    Sitemap,
    /// URL was discovered through crawling.
    Crawl,
}

/// Regex for markdown links: [text](url)
///
/// SAFETY: Pattern is a compile-time constant that is known to be valid.
#[allow(clippy::unwrap_used)]
static MARKDOWN_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").unwrap());

/// Regex for bare URLs
///
/// SAFETY: Pattern is a compile-time constant that is known to be valid.
#[allow(clippy::unwrap_used)]
static BARE_URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"https?://[^\s<>\[\]"'`]+"#).unwrap());

/// Regex for reference link definitions: [ref]: url
///
/// SAFETY: Pattern is a compile-time constant that is known to be valid.
#[allow(clippy::unwrap_used)]
static REFERENCE_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\[([^\]]+)\]:\s*(\S+)").unwrap());

/// Extract URLs from markdown content (llms.txt format).
///
/// Handles multiple URL formats commonly found in markdown:
/// - Markdown links: `[text](url)`
/// - Bare URLs: `https://example.com/page`
/// - Reference links: `[text][ref]` with `[ref]: url`
///
/// Relative URLs are resolved against the provided base URL.
/// URLs are deduplicated and normalized (fragments removed, trailing slashes handled).
///
/// # Arguments
///
/// * `content` - The markdown content to extract URLs from.
/// * `base_url` - The base URL for resolving relative URLs.
///
/// # Returns
///
/// A vector of unique, absolute URLs extracted from the content.
///
/// # Examples
///
/// ```rust
/// use blz_core::discovery::extract::extract_urls;
///
/// let content = r#"
/// # Documentation
/// - [Getting Started](/docs/getting-started)
/// - Check out https://example.com/api for more.
/// "#;
///
/// let urls = extract_urls(content, "https://example.com");
/// assert!(urls.contains(&"https://example.com/docs/getting-started".to_string()));
/// assert!(urls.contains(&"https://example.com/api".to_string()));
/// ```
#[must_use]
pub fn extract_urls(content: &str, base_url: &str) -> Vec<String> {
    let Ok(base) = Url::parse(base_url) else {
        return Vec::new();
    };

    let mut seen = HashSet::new();
    let mut urls = Vec::new();

    // Extract markdown links: [text](url)
    for cap in MARKDOWN_LINK_RE.captures_iter(content) {
        if let Some(url_match) = cap.get(2) {
            if let Some(normalized) = normalize_and_resolve(url_match.as_str(), &base) {
                if seen.insert(normalized.clone()) {
                    urls.push(normalized);
                }
            }
        }
    }

    // Extract reference link definitions: [ref]: url
    for cap in REFERENCE_LINK_RE.captures_iter(content) {
        if let Some(url_match) = cap.get(2) {
            if let Some(normalized) = normalize_and_resolve(url_match.as_str(), &base) {
                if seen.insert(normalized.clone()) {
                    urls.push(normalized);
                }
            }
        }
    }

    // Extract bare URLs: https://example.com/page
    for url_match in BARE_URL_RE.find_iter(content) {
        let url_str = url_match.as_str().trim_end_matches(['.', ',', ')', ']']);
        if let Some(normalized) = normalize_and_resolve(url_str, &base) {
            if seen.insert(normalized.clone()) {
                urls.push(normalized);
            }
        }
    }

    urls
}

/// Normalize and resolve a URL against a base URL.
///
/// - Resolves relative URLs against the base
/// - Removes URL fragments (#section)
/// - Removes trailing slashes for consistency
fn normalize_and_resolve(url_str: &str, base: &Url) -> Option<String> {
    let url_str = url_str.trim();

    // Skip empty strings, anchors, and non-http schemes
    if url_str.is_empty() || url_str.starts_with('#') {
        return None;
    }

    // Skip mailto, tel, javascript, and other non-http schemes
    if url_str.contains(':')
        && !url_str.starts_with("http://")
        && !url_str.starts_with("https://")
        && !url_str.starts_with('/')
    {
        return None;
    }

    // Try to parse as absolute URL first
    let mut resolved = if url_str.starts_with("http://") || url_str.starts_with("https://") {
        Url::parse(url_str).ok()?
    } else {
        // Resolve relative URL against base
        base.join(url_str).ok()?
    };

    // Normalize: remove fragment
    resolved.set_fragment(None);

    let mut result = resolved.to_string();

    // Remove trailing slash for consistency (but keep root paths like "https://example.com/")
    if result.ends_with('/') && result.matches('/').count() > 3 {
        result.pop();
    }

    Some(result)
}

/// Merge URLs from multiple sources, deduplicating by URL.
///
/// URLs from llms.txt take precedence in the output order, followed by
/// sitemap URLs. Duplicate URLs are removed, keeping the first occurrence.
///
/// # Arguments
///
/// * `llms_urls` - URLs extracted from llms.txt content.
/// * `sitemap` - Entries from sitemap.xml.
///
/// # Returns
///
/// A vector of [`DiscoveredUrl`] instances with source tracking.
///
/// # Examples
///
/// ```rust
/// use blz_core::discovery::extract::{merge_url_sources, UrlSource};
/// use blz_core::discovery::SitemapEntry;
///
/// let llms_urls = vec!["https://example.com/page1".to_string()];
/// let sitemap = vec![SitemapEntry {
///     url: "https://example.com/page2".to_string(),
///     lastmod: None,
///     changefreq: None,
///     priority: None,
/// }];
///
/// let merged = merge_url_sources(&llms_urls, &sitemap);
/// assert_eq!(merged.len(), 2);
/// ```
#[must_use]
pub fn merge_url_sources(llms_urls: &[String], sitemap: &[SitemapEntry]) -> Vec<DiscoveredUrl> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    // Add llms.txt URLs first (higher priority)
    for url in llms_urls {
        let normalized = normalize_url_for_dedup(url);
        if seen.insert(normalized) {
            result.push(DiscoveredUrl {
                url: url.clone(),
                source: UrlSource::LlmsTxt,
            });
        }
    }

    // Add sitemap URLs
    for entry in sitemap {
        let normalized = normalize_url_for_dedup(&entry.url);
        if seen.insert(normalized) {
            result.push(DiscoveredUrl {
                url: entry.url.clone(),
                source: UrlSource::Sitemap,
            });
        }
    }

    result
}

/// Normalize a URL for deduplication purposes.
///
/// Removes trailing slashes and fragments, lowercases the scheme and host.
fn normalize_url_for_dedup(url: &str) -> String {
    Url::parse(url).map_or_else(
        |_| url.to_string(),
        |mut parsed| {
            parsed.set_fragment(None);
            let mut result = parsed.to_string();
            // Remove trailing slash for comparison
            if result.ends_with('/') && result.matches('/').count() > 3 {
                result.pop();
            }
            result
        },
    )
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

    // extract_urls tests

    #[test]
    fn test_extracts_markdown_links() {
        let content = r#"
# Documentation
- [Getting Started](/docs/getting-started)
- [API Reference](https://example.com/api)
"#;
        let urls = extract_urls(content, "https://example.com");
        assert!(urls.contains(&"https://example.com/docs/getting-started".to_string()));
        assert!(urls.contains(&"https://example.com/api".to_string()));
    }

    #[test]
    fn test_extracts_bare_urls() {
        let content = "Check out https://example.com/page for more.";
        let urls = extract_urls(content, "https://example.com");
        assert!(urls.contains(&"https://example.com/page".to_string()));
    }

    #[test]
    fn test_resolves_relative_urls() {
        let content = "[Link](/relative/path)";
        let urls = extract_urls(content, "https://example.com");
        assert_eq!(urls[0], "https://example.com/relative/path");
    }

    #[test]
    fn test_handles_reference_links() {
        let content = r#"
[Getting Started][gs]

[gs]: /docs/getting-started
"#;
        let urls = extract_urls(content, "https://example.com");
        assert!(urls.contains(&"https://example.com/docs/getting-started".to_string()));
    }

    #[test]
    fn test_deduplicates_urls() {
        let content = "[Link1](/page) and [Link2](/page)";
        let urls = extract_urls(content, "https://example.com");
        assert_eq!(urls.iter().filter(|u| u.ends_with("/page")).count(), 1);
    }

    #[test]
    fn test_removes_fragments() {
        let content = "[Link](/page#section)";
        let urls = extract_urls(content, "https://example.com");
        assert_eq!(urls[0], "https://example.com/page");
    }

    #[test]
    fn test_skips_anchor_only_links() {
        let content = "[Link](#section)";
        let urls = extract_urls(content, "https://example.com");
        assert!(urls.is_empty());
    }

    #[test]
    fn test_skips_mailto_links() {
        let content = "[Email](mailto:test@example.com)";
        let urls = extract_urls(content, "https://example.com");
        assert!(urls.is_empty());
    }

    #[test]
    fn test_skips_javascript_links() {
        let content = "[Click](javascript:void(0))";
        let urls = extract_urls(content, "https://example.com");
        assert!(urls.is_empty());
    }

    #[test]
    fn test_handles_empty_content() {
        let urls = extract_urls("", "https://example.com");
        assert!(urls.is_empty());
    }

    #[test]
    fn test_handles_invalid_base_url() {
        let content = "[Link](/page)";
        let urls = extract_urls(content, "not-a-url");
        assert!(urls.is_empty());
    }

    #[test]
    fn test_extracts_multiple_urls_from_same_line() {
        let content = "[Link1](/page1) and [Link2](/page2)";
        let urls = extract_urls(content, "https://example.com");
        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://example.com/page1".to_string()));
        assert!(urls.contains(&"https://example.com/page2".to_string()));
    }

    #[test]
    fn test_handles_urls_with_query_params() {
        let content = "[Link](/page?foo=bar&baz=qux)";
        let urls = extract_urls(content, "https://example.com");
        assert_eq!(urls[0], "https://example.com/page?foo=bar&baz=qux");
    }

    #[test]
    fn test_bare_url_with_trailing_punctuation() {
        let content = "Visit https://example.com/page. Also https://example.com/other,";
        let urls = extract_urls(content, "https://example.com");
        assert!(urls.contains(&"https://example.com/page".to_string()));
        assert!(urls.contains(&"https://example.com/other".to_string()));
    }

    // merge_url_sources tests

    #[test]
    fn test_merge_deduplicates() {
        let llms = vec!["https://example.com/page1".to_string()];
        let sitemap = vec![SitemapEntry {
            url: "https://example.com/page1".to_string(),
            lastmod: None,
            changefreq: None,
            priority: None,
        }];
        let merged = merge_url_sources(&llms, &sitemap);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn test_merge_preserves_source() {
        let llms = vec!["https://example.com/from-llms".to_string()];
        let sitemap = vec![SitemapEntry {
            url: "https://example.com/from-sitemap".to_string(),
            lastmod: None,
            changefreq: None,
            priority: None,
        }];
        let merged = merge_url_sources(&llms, &sitemap);

        let llms_entry = merged.iter().find(|u| u.url.contains("from-llms")).unwrap();
        assert_eq!(llms_entry.source, UrlSource::LlmsTxt);

        let sitemap_entry = merged
            .iter()
            .find(|u| u.url.contains("from-sitemap"))
            .unwrap();
        assert_eq!(sitemap_entry.source, UrlSource::Sitemap);
    }

    #[test]
    fn test_merge_llms_takes_precedence() {
        let llms = vec!["https://example.com/page".to_string()];
        let sitemap = vec![SitemapEntry {
            url: "https://example.com/page".to_string(),
            lastmod: None,
            changefreq: None,
            priority: None,
        }];
        let merged = merge_url_sources(&llms, &sitemap);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, UrlSource::LlmsTxt);
    }

    #[test]
    fn test_merge_empty_inputs() {
        let merged = merge_url_sources(&[], &[]);
        assert!(merged.is_empty());
    }

    #[test]
    fn test_merge_only_llms() {
        let llms = vec![
            "https://example.com/page1".to_string(),
            "https://example.com/page2".to_string(),
        ];
        let merged = merge_url_sources(&llms, &[]);
        assert_eq!(merged.len(), 2);
        assert!(merged.iter().all(|u| u.source == UrlSource::LlmsTxt));
    }

    #[test]
    fn test_merge_only_sitemap() {
        let sitemap = vec![
            SitemapEntry {
                url: "https://example.com/page1".to_string(),
                lastmod: None,
                changefreq: None,
                priority: None,
            },
            SitemapEntry {
                url: "https://example.com/page2".to_string(),
                lastmod: None,
                changefreq: None,
                priority: None,
            },
        ];
        let merged = merge_url_sources(&[], &sitemap);
        assert_eq!(merged.len(), 2);
        assert!(merged.iter().all(|u| u.source == UrlSource::Sitemap));
    }

    #[test]
    fn test_url_source_serialization() {
        let discovered = DiscoveredUrl {
            url: "https://example.com/page".to_string(),
            source: UrlSource::LlmsTxt,
        };
        let json = serde_json::to_string(&discovered).unwrap();
        assert!(json.contains("\"source\":\"llms_txt\""));

        let sitemap_discovered = DiscoveredUrl {
            url: "https://example.com/page".to_string(),
            source: UrlSource::Sitemap,
        };
        let json = serde_json::to_string(&sitemap_discovered).unwrap();
        assert!(json.contains("\"source\":\"sitemap\""));
    }
}
