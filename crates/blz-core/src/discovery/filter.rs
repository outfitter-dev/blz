//! URL filtering for documentation discovery.
//!
//! This module provides functionality to filter URLs to documentation-relevant
//! pages, helping to focus crawling and indexing on useful content.
//!
//! ## Quick Start
//!
//! ```rust
//! use blz_core::discovery::filter::{filter_to_domain, filter_to_docs, is_likely_docs_path};
//!
//! // Filter URLs to a specific domain
//! let urls = ["https://example.com/page", "https://other.com/page"];
//! let filtered = filter_to_domain(&urls, "example.com");
//! assert_eq!(filtered.len(), 1);
//!
//! // Check if a path is likely documentation
//! assert!(is_likely_docs_path("/docs/getting-started"));
//! assert!(!is_likely_docs_path("/blog/post-1"));
//!
//! // Filter to likely documentation URLs
//! let urls = [
//!     "https://example.com/docs/intro",
//!     "https://example.com/blog/news",
//! ];
//! let docs_only = filter_to_docs(&urls);
//! assert_eq!(docs_only.len(), 1);
//! ```

use url::Url;

/// Path segments that indicate documentation content.
const DOCS_PATH_INDICATORS: &[&str] = &[
    "/docs/",
    "/docs",
    "/guide/",
    "/guide",
    "/api/",
    "/api",
    "/reference/",
    "/reference",
    "/learn/",
    "/learn",
    "/tutorial/",
    "/tutorial",
    "/tutorials/",
    "/tutorials",
    "/manual/",
    "/manual",
    "/handbook/",
    "/handbook",
    "/getting-started",
    "/quickstart",
    "/examples/",
    "/examples",
];

/// Path segments that indicate non-documentation content.
const NON_DOCS_PATH_INDICATORS: &[&str] = &[
    "/blog/",
    "/blog",
    "/about",
    "/careers",
    "/pricing",
    "/login",
    "/signup",
    "/sign-up",
    "/signin",
    "/sign-in",
    "/register",
    "/assets/",
    "/static/",
    "/_next/",
    "/_nuxt/",
    "/cdn-cgi/",
    "/wp-content/",
    "/wp-admin/",
    "/feed/",
    "/rss",
    "/atom",
    "/sitemap",
    "/robots.txt",
    "/favicon",
    "/contact",
    "/privacy",
    "/terms",
    "/legal",
    "/jobs",
    "/team",
    "/press",
    "/news/",
    "/news",
    "/changelog",
    "/releases",
];

/// File extensions that indicate non-documentation content.
const NON_DOCS_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".ico", ".bmp", ".tiff", ".css", ".js",
    ".mjs", ".cjs", ".ts", ".tsx", ".jsx", ".woff", ".woff2", ".ttf", ".eot", ".otf", ".pdf",
    ".zip", ".tar", ".gz", ".rar", ".mp3", ".mp4", ".webm", ".ogg", ".wav", ".json", ".xml",
    ".yaml", ".yml", ".toml", ".map", ".min.js", ".min.css",
];

/// Filter URLs to only those on the specified domain.
///
/// Includes both the exact domain and subdomains. For example, filtering
/// to "example.com" will include both `https://example.com/page` and
/// `https://docs.example.com/page`.
///
/// # Arguments
///
/// * `urls` - URLs to filter.
/// * `domain` - The domain to filter to (without scheme).
///
/// # Returns
///
/// URLs that belong to the specified domain or its subdomains.
///
/// # Examples
///
/// ```rust
/// use blz_core::discovery::filter::filter_to_domain;
///
/// let urls = [
///     "https://example.com/page1",
///     "https://docs.example.com/page2",
///     "https://other.com/page3",
/// ];
///
/// let filtered = filter_to_domain(&urls, "example.com");
/// assert_eq!(filtered.len(), 2);
/// ```
#[must_use]
pub fn filter_to_domain(urls: &[&str], domain: &str) -> Vec<String> {
    let domain_lower = domain.to_lowercase();

    urls.iter()
        .filter_map(|url_str| {
            let url = Url::parse(url_str).ok()?;
            let host = url.host_str()?;
            let host_lower = host.to_lowercase();

            // Check if the host matches the domain or is a subdomain
            if host_lower == domain_lower || host_lower.ends_with(&format!(".{domain_lower}")) {
                Some((*url_str).to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Check if a URL path is likely documentation.
///
/// Uses heuristics based on common documentation URL patterns. A path is
/// considered likely documentation if it:
/// - Contains documentation-related segments like `/docs/`, `/guide/`, `/api/`
/// - Does not contain non-documentation segments like `/blog/`, `/about`
/// - Does not have file extensions for static assets
///
/// # Arguments
///
/// * `path` - The URL path to check (including leading slash).
///
/// # Returns
///
/// `true` if the path appears to be documentation, `false` otherwise.
///
/// # Examples
///
/// ```rust
/// use blz_core::discovery::filter::is_likely_docs_path;
///
/// // Documentation paths
/// assert!(is_likely_docs_path("/docs/getting-started"));
/// assert!(is_likely_docs_path("/guide/introduction"));
/// assert!(is_likely_docs_path("/api/reference"));
///
/// // Non-documentation paths
/// assert!(!is_likely_docs_path("/blog/post-1"));
/// assert!(!is_likely_docs_path("/about"));
/// assert!(!is_likely_docs_path("/assets/logo.png"));
/// ```
#[must_use]
pub fn is_likely_docs_path(path: &str) -> bool {
    let path_lower = path.to_lowercase();

    // Check for non-docs extensions first
    for ext in NON_DOCS_EXTENSIONS {
        if path_lower.ends_with(ext) {
            return false;
        }
    }

    // Check for non-docs path indicators
    for indicator in NON_DOCS_PATH_INDICATORS {
        if contains_path_segment(&path_lower, indicator) {
            return false;
        }
    }

    // Check for docs path indicators
    for indicator in DOCS_PATH_INDICATORS {
        if contains_path_segment(&path_lower, indicator) {
            return true;
        }
    }

    // Default: not clearly documentation
    false
}

/// Check if a path contains a segment indicator with proper boundaries.
///
/// For indicators ending with `/` (like `/docs/`), uses simple contains.
/// For indicators without trailing `/` (like `/docs`), ensures the match
/// is followed by `/`, `?`, `#`, or end of string to avoid false positives
/// like `/doc-builder/` matching `/doc`.
fn contains_path_segment(path: &str, indicator: &str) -> bool {
    // Indicators with trailing slash are already bounded
    if indicator.ends_with('/') {
        return path.starts_with(indicator) || path.contains(indicator);
    }

    // For indicators without trailing slash, check segment boundaries
    if let Some(rest) = path.strip_prefix(indicator) {
        // Check what follows the indicator at the start
        return rest.is_empty()
            || rest.starts_with('/')
            || rest.starts_with('?')
            || rest.starts_with('#');
    }

    // Check for indicator in the middle of the path
    if let Some(pos) = path.find(indicator) {
        let rest = &path[pos + indicator.len()..];
        return rest.is_empty()
            || rest.starts_with('/')
            || rest.starts_with('?')
            || rest.starts_with('#');
    }

    false
}

/// Filter URLs to likely documentation pages.
///
/// Combines domain filtering with documentation path detection to identify
/// URLs that are most likely to contain useful documentation content.
///
/// # Arguments
///
/// * `urls` - URLs to filter.
///
/// # Returns
///
/// URLs that appear to be documentation pages.
///
/// # Examples
///
/// ```rust
/// use blz_core::discovery::filter::filter_to_docs;
///
/// let urls = [
///     "https://example.com/docs/intro",
///     "https://example.com/blog/news",
///     "https://example.com/assets/style.css",
/// ];
///
/// let docs = filter_to_docs(&urls);
/// assert_eq!(docs.len(), 1);
/// assert!(docs[0].contains("/docs/"));
/// ```
#[must_use]
pub fn filter_to_docs(urls: &[&str]) -> Vec<String> {
    urls.iter()
        .filter_map(|url_str| {
            let url = Url::parse(url_str).ok()?;
            let path = url.path();

            if is_likely_docs_path(path) {
                Some((*url_str).to_string())
            } else {
                None
            }
        })
        .collect()
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

    // filter_to_domain tests

    #[test]
    fn test_filters_to_same_domain() {
        let urls = [
            "https://example.com/page1",
            "https://other.com/page2",
            "https://example.com/page3",
        ];
        let filtered = filter_to_domain(&urls, "example.com");
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|u| u.contains("example.com")));
    }

    #[test]
    fn test_includes_subdomains() {
        let urls = [
            "https://docs.example.com/page",
            "https://api.example.com/page",
        ];
        let filtered = filter_to_domain(&urls, "example.com");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_domain_case_insensitive() {
        let urls = ["https://EXAMPLE.COM/page", "https://Example.Com/page2"];
        let filtered = filter_to_domain(&urls, "example.com");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_excludes_similar_domains() {
        let urls = [
            "https://example.com/page",
            "https://notexample.com/page",
            "https://example.com.evil.com/page",
        ];
        let filtered = filter_to_domain(&urls, "example.com");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0], "https://example.com/page");
    }

    #[test]
    fn test_empty_input() {
        let urls: [&str; 0] = [];
        let filtered = filter_to_domain(&urls, "example.com");
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_invalid_urls_skipped() {
        let urls = [
            "https://example.com/page",
            "not-a-url",
            "https://example.com/page2",
        ];
        let filtered = filter_to_domain(&urls, "example.com");
        assert_eq!(filtered.len(), 2);
    }

    // is_likely_docs_path tests

    #[test]
    fn test_docs_paths() {
        assert!(is_likely_docs_path("/docs/getting-started"));
        assert!(is_likely_docs_path("/guide/introduction"));
        assert!(is_likely_docs_path("/api/reference"));
        assert!(is_likely_docs_path("/reference/types"));
        assert!(is_likely_docs_path("/learn/basics"));
        assert!(is_likely_docs_path("/tutorial/step-1"));
    }

    #[test]
    fn test_nested_docs_paths() {
        assert!(is_likely_docs_path("/v2/docs/getting-started"));
        assert!(is_likely_docs_path("/en/guide/introduction"));
        assert!(is_likely_docs_path("/latest/api/reference"));
    }

    #[test]
    fn test_non_docs_paths() {
        assert!(!is_likely_docs_path("/blog/post-1"));
        assert!(!is_likely_docs_path("/about"));
        assert!(!is_likely_docs_path("/careers"));
        assert!(!is_likely_docs_path("/pricing"));
        assert!(!is_likely_docs_path("/login"));
        assert!(!is_likely_docs_path("/assets/logo.png"));
    }

    #[test]
    fn test_static_asset_extensions() {
        assert!(!is_likely_docs_path("/docs/image.png"));
        assert!(!is_likely_docs_path("/api/schema.json"));
        assert!(!is_likely_docs_path("/guide/style.css"));
        assert!(!is_likely_docs_path("/tutorial/script.js"));
    }

    #[test]
    fn test_path_case_insensitive() {
        assert!(is_likely_docs_path("/DOCS/Getting-Started"));
        assert!(is_likely_docs_path("/Guide/INTRODUCTION"));
        assert!(!is_likely_docs_path("/BLOG/Post-1"));
    }

    #[test]
    fn test_empty_path() {
        assert!(!is_likely_docs_path(""));
        assert!(!is_likely_docs_path("/"));
    }

    #[test]
    fn test_root_docs_path() {
        assert!(is_likely_docs_path("/docs"));
        assert!(is_likely_docs_path("/guide"));
        assert!(is_likely_docs_path("/api"));
    }

    #[test]
    fn test_ambiguous_paths() {
        // These don't have clear docs indicators, should return false
        assert!(!is_likely_docs_path("/page"));
        assert!(!is_likely_docs_path("/product/feature"));
        assert!(!is_likely_docs_path("/"));
    }

    // filter_to_docs tests

    #[test]
    fn test_filters_docs_urls() {
        let urls = [
            "https://example.com/docs/intro",
            "https://example.com/blog/news",
            "https://example.com/assets/style.css",
        ];
        let filtered = filter_to_docs(&urls);
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].contains("/docs/"));
    }

    #[test]
    fn test_filters_multiple_docs_urls() {
        let urls = [
            "https://example.com/docs/intro",
            "https://example.com/guide/getting-started",
            "https://example.com/api/reference",
            "https://example.com/blog/news",
        ];
        let filtered = filter_to_docs(&urls);
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_filter_docs_empty_input() {
        let urls: [&str; 0] = [];
        let filtered = filter_to_docs(&urls);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_docs_invalid_urls_skipped() {
        let urls = [
            "https://example.com/docs/intro",
            "not-a-url",
            "https://example.com/guide/start",
        ];
        let filtered = filter_to_docs(&urls);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_docs_all_non_docs() {
        let urls = [
            "https://example.com/blog/post",
            "https://example.com/about",
            "https://example.com/pricing",
        ];
        let filtered = filter_to_docs(&urls);
        assert!(filtered.is_empty());
    }

    // Additional edge case tests

    #[test]
    fn test_handbook_and_manual_paths() {
        assert!(is_likely_docs_path("/handbook/chapter-1"));
        assert!(is_likely_docs_path("/manual/installation"));
    }

    #[test]
    fn test_examples_path() {
        assert!(is_likely_docs_path("/examples/basic"));
        assert!(is_likely_docs_path("/examples"));
    }

    #[test]
    fn test_quickstart_path() {
        assert!(is_likely_docs_path("/quickstart"));
        assert!(is_likely_docs_path("/getting-started"));
    }

    #[test]
    fn test_framework_specific_paths() {
        // Next.js, Nuxt, etc. asset paths should be excluded
        assert!(!is_likely_docs_path("/_next/static/chunks/main.js"));
        assert!(!is_likely_docs_path("/_nuxt/something.js"));
    }

    #[test]
    fn test_feed_and_rss_paths() {
        assert!(!is_likely_docs_path("/feed/"));
        assert!(!is_likely_docs_path("/rss"));
        assert!(!is_likely_docs_path("/atom.xml"));
    }

    #[test]
    fn test_wordpress_paths() {
        assert!(!is_likely_docs_path("/wp-content/uploads/image.png"));
        assert!(!is_likely_docs_path("/wp-admin/"));
    }

    #[test]
    fn test_legal_paths() {
        assert!(!is_likely_docs_path("/privacy"));
        assert!(!is_likely_docs_path("/terms"));
        assert!(!is_likely_docs_path("/legal"));
    }

    #[test]
    fn test_path_segment_boundaries() {
        // /doc-builder/ should NOT match /docs (false positive from simple contains)
        assert!(!is_likely_docs_path("/doc-builder/something"));
        assert!(!is_likely_docs_path("/documentary/film"));
        assert!(!is_likely_docs_path("/guidance-system/config"));
        assert!(!is_likely_docs_path("/api-client/utils"));

        // These SHOULD match (proper segment boundaries)
        assert!(is_likely_docs_path("/docs/builder"));
        assert!(is_likely_docs_path("/api/client"));
        assert!(is_likely_docs_path("/guide/system"));

        // With query strings and fragments
        assert!(is_likely_docs_path("/docs?version=2"));
        assert!(is_likely_docs_path("/api#section"));
        assert!(!is_likely_docs_path("/doc-viewer?page=1"));
    }
}
