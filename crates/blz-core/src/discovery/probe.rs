//! URL probing for documentation source discovery.
//!
//! This module provides functionality to probe domains and discover
//! available documentation sources like llms.txt, llms-full.txt, and sitemap.xml.

use crate::{Error, Result};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use tracing::instrument;

/// Default timeout for probe requests (5 seconds per URL).
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Result of probing a domain for documentation sources.
///
/// Contains URLs for any discovered documentation files.
/// Use [`best_url`] to get the preferred documentation source.
///
/// [`best_url`]: ProbeResult::best_url
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeResult {
    /// The domain that was probed.
    pub domain: String,
    /// URL to llms-full.txt (preferred - complete documentation).
    pub llms_full_url: Option<String>,
    /// URL to llms.txt (fallback - index only).
    pub llms_url: Option<String>,
    /// URL to sitemap.xml (for URL discovery).
    pub sitemap_url: Option<String>,
    /// Whether a docs.* subdomain was checked.
    pub docs_subdomain_checked: bool,
}

impl ProbeResult {
    /// Returns the best available documentation URL.
    ///
    /// Preference order:
    /// 1. llms-full.txt (complete documentation)
    /// 2. llms.txt (index)
    /// 3. sitemap.xml (for URL discovery)
    #[must_use]
    pub fn best_url(&self) -> Option<&str> {
        self.llms_full_url
            .as_deref()
            .or(self.llms_url.as_deref())
            .or(self.sitemap_url.as_deref())
    }

    /// Returns true if any documentation source was found.
    #[must_use]
    pub const fn has_source(&self) -> bool {
        self.llms_full_url.is_some() || self.llms_url.is_some() || self.sitemap_url.is_some()
    }
}

/// Probe a domain to discover documentation sources.
///
/// Checks in order:
/// 1. `https://{domain}/llms-full.txt` (preferred - complete docs)
/// 2. `https://{domain}/llms.txt` (fallback - index)
/// 3. `https://{domain}/sitemap.xml` (for URL discovery)
/// 4. `https://docs.{domain}/*` if main domain has nothing
///
/// Uses HEAD requests for efficiency (doesn't download content during probe).
///
/// # Arguments
///
/// * `domain` - The domain to probe (e.g., `hono.dev` or `https://hono.dev`)
///
/// # Returns
///
/// A [`ProbeResult`] containing all discovered URLs.
///
/// # Errors
///
/// Returns an error if the HTTP client cannot be constructed.
///
/// # Examples
///
/// ```no_run
/// use blz_core::discovery::probe_domain;
///
/// # async fn example() -> blz_core::Result<()> {
/// let result = probe_domain("hono.dev").await?;
///
/// if let Some(url) = result.best_url() {
///     println!("Found documentation at: {}", url);
/// }
/// # Ok(())
/// # }
/// ```
#[instrument(skip_all, fields(domain = %domain))]
pub async fn probe_domain(domain: &str) -> Result<ProbeResult> {
    let normalized = normalize_domain(domain);
    let client = build_probe_client()?;

    // Probe main domain first
    let mut result = probe_single_domain(&client, &normalized).await?;

    // If nothing found on main domain, try docs.* subdomain
    if !result.has_source() && !normalized.starts_with("docs.") {
        let docs_domain = format!("docs.{normalized}");
        let docs_result = probe_single_domain(&client, &docs_domain).await?;

        if docs_result.has_source() {
            result.llms_full_url = docs_result.llms_full_url;
            result.llms_url = docs_result.llms_url;
            result.sitemap_url = docs_result.sitemap_url;
        }
        result.docs_subdomain_checked = true;
    }

    Ok(result)
}

/// Probe a single domain (without subdomain fallback).
async fn probe_single_domain(client: &Client, domain: &str) -> Result<ProbeResult> {
    // Use http for localhost/loopback (testing), https for everything else
    let protocol = if domain.starts_with("127.0.0.1") || domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    let base_url = format!("{protocol}://{domain}");

    // Probe llms-full.txt and llms.txt in parallel
    let llms_full_url = format!("{base_url}/llms-full.txt");
    let llms_url = format!("{base_url}/llms.txt");
    let sitemap_url = format!("{base_url}/sitemap.xml");

    let (llms_full_exists, llms_exists) = tokio::join!(
        probe_url_exists(client, &llms_full_url),
        probe_url_exists(client, &llms_url),
    );

    // Only probe sitemap if neither llms file exists
    let sitemap_exists = if !llms_full_exists && !llms_exists {
        probe_url_exists(client, &sitemap_url).await
    } else {
        false
    };

    Ok(ProbeResult {
        domain: domain.to_string(),
        llms_full_url: llms_full_exists.then_some(llms_full_url),
        llms_url: llms_exists.then_some(llms_url),
        sitemap_url: sitemap_exists.then_some(sitemap_url),
        docs_subdomain_checked: false,
    })
}

/// Check if a URL exists using a HEAD request.
///
/// Follows redirects to determine the final status.
async fn probe_url_exists(client: &Client, url: &str) -> bool {
    client.head(url).send().await.is_ok_and(|response| {
        let status = response.status();
        // Accept 2xx status codes as "exists"
        status.is_success()
            || status == StatusCode::MOVED_PERMANENTLY
            || status == StatusCode::FOUND
            || status == StatusCode::TEMPORARY_REDIRECT
            || status == StatusCode::PERMANENT_REDIRECT
    })
}

/// Normalize a domain string by removing protocol and trailing slashes.
fn normalize_domain(domain: &str) -> String {
    let mut normalized = domain.trim();

    // Remove protocol prefix
    if let Some(stripped) = normalized.strip_prefix("https://") {
        normalized = stripped;
    } else if let Some(stripped) = normalized.strip_prefix("http://") {
        normalized = stripped;
    }

    // Remove trailing slash
    normalized = normalized.trim_end_matches('/');

    // Remove any path component (keep only the domain)
    if let Some(slash_pos) = normalized.find('/') {
        normalized = &normalized[..slash_pos];
    }

    normalized.to_string()
}

/// Build an HTTP client configured for probing.
fn build_probe_client() -> Result<Client> {
    Client::builder()
        .timeout(PROBE_TIMEOUT)
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

    #[tokio::test]
    async fn test_probe_finds_llms_full_txt() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(path("/llms-full.txt"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        Mock::given(method("HEAD"))
            .and(path("/llms.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        // Extract host from mock server URL (format: http://127.0.0.1:port)
        let uri = mock_server.uri();
        let domain = uri.trim_start_matches("http://");

        let result = probe_domain(domain).await.unwrap();

        assert!(
            result.llms_full_url.is_some(),
            "Expected llms_full_url to be Some"
        );
        assert!(
            result
                .llms_full_url
                .as_ref()
                .unwrap()
                .contains("llms-full.txt"),
            "llms_full_url should contain llms-full.txt"
        );
    }

    #[tokio::test]
    async fn test_probe_falls_back_to_llms_txt() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(path("/llms-full.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        Mock::given(method("HEAD"))
            .and(path("/llms.txt"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let uri = mock_server.uri();
        let domain = uri.trim_start_matches("http://");

        let result = probe_domain(domain).await.unwrap();

        assert!(
            result.llms_full_url.is_none(),
            "Expected llms_full_url to be None when 404"
        );
        assert!(
            result.llms_url.is_some(),
            "Expected llms_url to be Some as fallback"
        );
    }

    #[tokio::test]
    async fn test_probe_finds_sitemap() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(path("/llms-full.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        Mock::given(method("HEAD"))
            .and(path("/llms.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        Mock::given(method("HEAD"))
            .and(path("/sitemap.xml"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let uri = mock_server.uri();
        let domain = uri.trim_start_matches("http://");

        let result = probe_domain(domain).await.unwrap();

        assert!(
            result.sitemap_url.is_some(),
            "Expected sitemap_url to be Some"
        );
    }

    #[tokio::test]
    async fn test_probe_handles_redirects() {
        let mock_server = MockServer::start().await;

        // 301 redirect that leads to 200 OK - reqwest follows redirects
        Mock::given(method("HEAD"))
            .and(path("/llms-full.txt"))
            .respond_with(
                ResponseTemplate::new(301).insert_header("Location", "/docs/llms-full.txt"),
            )
            .mount(&mock_server)
            .await;

        // The redirect destination returns 200
        Mock::given(method("HEAD"))
            .and(path("/docs/llms-full.txt"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        Mock::given(method("HEAD"))
            .and(path("/llms.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let uri = mock_server.uri();
        let domain = uri.trim_start_matches("http://");

        let result = probe_domain(domain).await.unwrap();

        assert!(
            result.llms_full_url.is_some(),
            "Redirect chain ending in 200 should be treated as resource exists"
        );
    }

    #[test]
    fn test_best_url_prefers_llms_full() {
        let result = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: Some("https://example.com/llms-full.txt".to_string()),
            llms_url: Some("https://example.com/llms.txt".to_string()),
            sitemap_url: Some("https://example.com/sitemap.xml".to_string()),
            docs_subdomain_checked: false,
        };

        assert_eq!(result.best_url(), Some("https://example.com/llms-full.txt"));
    }

    #[test]
    fn test_best_url_falls_back_to_llms() {
        let result = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: None,
            llms_url: Some("https://example.com/llms.txt".to_string()),
            sitemap_url: Some("https://example.com/sitemap.xml".to_string()),
            docs_subdomain_checked: false,
        };

        assert_eq!(result.best_url(), Some("https://example.com/llms.txt"));
    }

    #[test]
    fn test_best_url_falls_back_to_sitemap() {
        let result = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: None,
            llms_url: None,
            sitemap_url: Some("https://example.com/sitemap.xml".to_string()),
            docs_subdomain_checked: false,
        };

        assert_eq!(result.best_url(), Some("https://example.com/sitemap.xml"));
    }

    #[test]
    fn test_best_url_returns_none_when_empty() {
        let result = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: None,
            llms_url: None,
            sitemap_url: None,
            docs_subdomain_checked: true,
        };

        assert_eq!(result.best_url(), None);
    }

    #[test]
    fn test_has_source_with_llms_full() {
        let result = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: Some("https://example.com/llms-full.txt".to_string()),
            llms_url: None,
            sitemap_url: None,
            docs_subdomain_checked: false,
        };

        assert!(result.has_source());
    }

    #[test]
    fn test_has_source_with_llms() {
        let result = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: None,
            llms_url: Some("https://example.com/llms.txt".to_string()),
            sitemap_url: None,
            docs_subdomain_checked: false,
        };

        assert!(result.has_source());
    }

    #[test]
    fn test_has_source_with_sitemap() {
        let result = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: None,
            llms_url: None,
            sitemap_url: Some("https://example.com/sitemap.xml".to_string()),
            docs_subdomain_checked: false,
        };

        assert!(result.has_source());
    }

    #[test]
    fn test_has_source_empty() {
        let result = ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: None,
            llms_url: None,
            sitemap_url: None,
            docs_subdomain_checked: true,
        };

        assert!(!result.has_source());
    }

    #[test]
    fn test_normalize_domain_strips_https() {
        assert_eq!(normalize_domain("https://example.com"), "example.com");
    }

    #[test]
    fn test_normalize_domain_strips_http() {
        assert_eq!(normalize_domain("http://example.com"), "example.com");
    }

    #[test]
    fn test_normalize_domain_strips_trailing_slash() {
        assert_eq!(normalize_domain("example.com/"), "example.com");
    }

    #[test]
    fn test_normalize_domain_strips_path() {
        assert_eq!(normalize_domain("example.com/path/to/page"), "example.com");
    }

    #[test]
    fn test_normalize_domain_strips_all() {
        assert_eq!(
            normalize_domain("https://example.com/path/to/page/"),
            "example.com"
        );
    }

    #[test]
    fn test_normalize_domain_trims_whitespace() {
        assert_eq!(normalize_domain("  example.com  "), "example.com");
    }

    #[test]
    fn test_normalize_domain_preserves_subdomain() {
        assert_eq!(normalize_domain("docs.example.com"), "docs.example.com");
    }

    #[tokio::test]
    async fn test_probe_no_sources_found() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(path("/llms-full.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        Mock::given(method("HEAD"))
            .and(path("/llms.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        Mock::given(method("HEAD"))
            .and(path("/sitemap.xml"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let uri = mock_server.uri();
        let domain = uri.trim_start_matches("http://");

        let result = probe_domain(domain).await.unwrap();

        assert!(!result.has_source());
        assert!(result.best_url().is_none());
    }

    #[tokio::test]
    async fn test_probe_finds_both_llms_files() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(path("/llms-full.txt"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        Mock::given(method("HEAD"))
            .and(path("/llms.txt"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let uri = mock_server.uri();
        let domain = uri.trim_start_matches("http://");

        let result = probe_domain(domain).await.unwrap();

        assert!(result.llms_full_url.is_some());
        assert!(result.llms_url.is_some());
        // When both exist, sitemap is not probed
        assert!(result.sitemap_url.is_none());
        // best_url should prefer llms-full
        assert!(result.best_url().unwrap().contains("llms-full.txt"));
    }

    #[tokio::test]
    async fn test_probe_handles_500_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(path("/llms-full.txt"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        Mock::given(method("HEAD"))
            .and(path("/llms.txt"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        Mock::given(method("HEAD"))
            .and(path("/sitemap.xml"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let uri = mock_server.uri();
        let domain = uri.trim_start_matches("http://");

        let result = probe_domain(domain).await.unwrap();

        // 500 errors should be treated as "not found"
        assert!(!result.has_source());
    }
}
