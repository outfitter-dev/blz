//! URL probing for documentation source discovery.
//!
//! This module provides functionality to probe domains and discover
//! available documentation sources like llms.txt, llms-full.txt, and sitemap.xml.
//!
//! ## Smart URL Resolution
//!
//! When given a URL with a path (e.g., `https://code.claude.com/docs`), the probe
//! will search in multiple locations with this cascade:
//!
//! 1. **Link headers** - Check the provided URL for `Link` headers declaring llms.txt
//! 2. **Path-relative** - Probe for files relative to the provided path
//! 3. **Host root** - Probe the root of the same host
//! 4. **Domain root** - Suggest checking the parent domain (requires confirmation)
//!
//! This ensures we find native llms.txt files before falling back to generation.

use crate::{Error, Result};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use tracing::{debug, instrument};
use url::Url;

/// Default timeout for probe requests (5 seconds per URL).
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// How the documentation source was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiscoveryMethod {
    /// Found via Link header on the provided URL.
    LinkHeader,
    /// Found at the path-relative location (e.g., `/docs/llms-full.txt`).
    PathRelative,
    /// Found at the host root (e.g., `/llms-full.txt`).
    HostRoot,
    /// Found at a docs.* subdomain.
    DocsSubdomain,
    /// Found at the parent domain (requires user confirmation).
    ParentDomain,
    /// No documentation found through probing.
    #[default]
    NotFound,
}

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
    /// How the source was discovered.
    pub discovery_method: DiscoveryMethod,
    /// If discovery requires leaving the original scope, this contains the suggested URL.
    /// The CLI should confirm with the user before using this.
    pub requires_confirmation: bool,
    /// The original URL that was probed (if using smart resolution).
    pub original_url: Option<String>,
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
        let docs_result =
            probe_single_domain_with_method(&client, &docs_domain, DiscoveryMethod::DocsSubdomain)
                .await?;

        if docs_result.has_source() {
            result.llms_full_url = docs_result.llms_full_url;
            result.llms_url = docs_result.llms_url;
            result.sitemap_url = docs_result.sitemap_url;
            result.discovery_method = DiscoveryMethod::DocsSubdomain;
        }
        result.docs_subdomain_checked = true;
    }

    Ok(result)
}

/// Probe a URL with smart resolution to discover documentation sources.
///
/// This function implements a cascade of discovery methods:
///
/// 1. **Link headers** - Checks the provided URL for `Link` headers with
///    `rel="llms-txt"` or `rel="llms-full-txt"` relations (per llms.txt spec)
/// 2. **Path-relative** - If the URL has a path, probes for files relative to that path
///    (e.g., `https://example.com/docs` → `https://example.com/docs/llms-full.txt`)
/// 3. **Host root** - Probes the root of the same host
/// 4. **Docs subdomain** - Tries `docs.{domain}` if nothing found
/// 5. **Parent domain** - If the URL is on a subdomain, suggests checking the parent
///    domain (sets `requires_confirmation = true`)
///
/// # Arguments
///
/// * `url` - The URL to probe. Can be a full URL or just a domain.
///
/// # Returns
///
/// A [`ProbeResult`] with `requires_confirmation` set to `true` if the discovered
/// source is on a different scope than the original URL (CLI should confirm with user).
///
/// # Examples
///
/// ```no_run
/// use blz_core::discovery::probe_url;
///
/// # async fn example() -> blz_core::Result<()> {
/// // Smart resolution finds llms-full.txt via Link header
/// let result = probe_url("https://code.claude.com/docs").await?;
/// assert!(result.llms_full_url.is_some());
///
/// // If we had to leave the original scope, requires_confirmation is set
/// if result.requires_confirmation {
///     println!("Found at different scope - please confirm");
/// }
/// # Ok(())
/// # }
/// ```
#[instrument(skip_all, fields(url = %url))]
pub async fn probe_url(url: &str) -> Result<ProbeResult> {
    let client = build_probe_client()?;

    // Parse the URL to extract components
    let Ok(parsed) = Url::parse(url) else {
        // If it's not a valid URL, treat it as a domain
        debug!("Input is not a valid URL, treating as domain");
        return probe_domain(url).await;
    };

    // Get host with port (needed for mock servers and non-standard ports)
    let host = parsed.port().map_or_else(
        || parsed.host_str().unwrap_or("").to_string(),
        |port| format!("{}:{}", parsed.host_str().unwrap_or(""), port),
    );
    let host_without_port = parsed.host_str().unwrap_or("");
    let path = parsed.path();
    let original_url = url.to_string();

    debug!(host = %host, path = %path, "Parsed URL components");

    // Step 1: Check Link headers on the provided URL
    if let Some(result) = check_link_headers(&client, url, host_without_port).await? {
        debug!("Found documentation via Link header");
        return Ok(ProbeResult {
            original_url: Some(original_url),
            ..result
        });
    }

    // Step 2: If URL has a non-trivial path, probe relative to that path
    if path.len() > 1 && path != "/" {
        let path_base = path.trim_end_matches('/');
        if let Some(result) = probe_path_relative(&client, &parsed, path_base).await? {
            debug!(path = %path_base, "Found documentation at path-relative location");
            return Ok(ProbeResult {
                original_url: Some(original_url),
                ..result
            });
        }
    }

    // Step 3: Probe host root
    let host_result = probe_single_domain(&client, &host).await?;
    if host_result.has_source() {
        debug!("Found documentation at host root");
        return Ok(ProbeResult {
            original_url: Some(original_url),
            ..host_result
        });
    }

    // Step 4: Try docs.* subdomain (use host without port for subdomain construction)
    if !host_without_port.starts_with("docs.") {
        let docs_host = format!("docs.{host_without_port}");
        let docs_result =
            probe_single_domain_with_method(&client, &docs_host, DiscoveryMethod::DocsSubdomain)
                .await?;
        if docs_result.has_source() {
            debug!(docs_host = %docs_host, "Found documentation at docs subdomain");
            return Ok(ProbeResult {
                original_url: Some(original_url),
                docs_subdomain_checked: true,
                ..docs_result
            });
        }
    }

    // Step 5: If we're on a subdomain, suggest checking the parent domain
    // This requires user confirmation since we're leaving the original scope
    if let Some(parent_domain) = extract_parent_domain(host_without_port) {
        let parent_result =
            probe_single_domain_with_method(&client, &parent_domain, DiscoveryMethod::ParentDomain)
                .await?;
        if parent_result.has_source() {
            debug!(parent_domain = %parent_domain, "Found documentation at parent domain (requires confirmation)");
            return Ok(ProbeResult {
                original_url: Some(original_url),
                requires_confirmation: true,
                docs_subdomain_checked: true,
                ..parent_result
            });
        }
    }

    // Nothing found
    debug!("No documentation sources found");
    Ok(ProbeResult {
        domain: host_without_port.to_string(),
        llms_full_url: None,
        llms_url: None,
        sitemap_url: None,
        docs_subdomain_checked: true,
        discovery_method: DiscoveryMethod::NotFound,
        requires_confirmation: false,
        original_url: Some(original_url),
    })
}

/// Check Link headers on a URL for llms.txt declarations.
///
/// Per the llms.txt spec, servers can declare llms.txt locations via Link headers:
/// - `Link: </llms.txt>; rel="llms-txt"`
/// - `Link: </llms-full.txt>; rel="llms-full-txt"`
///
/// The `domain` parameter should be the host without port for the result.
async fn check_link_headers(
    client: &Client,
    url: &str,
    domain: &str,
) -> Result<Option<ProbeResult>> {
    let response = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            debug!(error = %e, "Failed to fetch URL for Link header check");
            return Ok(None);
        },
    };

    if !response.status().is_success() {
        return Ok(None);
    }

    // Parse Link headers
    let mut llms_full_url: Option<String> = None;
    let mut llms_url: Option<String> = None;

    if let Some(link_header) = response.headers().get("link") {
        if let Ok(link_str) = link_header.to_str() {
            debug!(link_header = %link_str, "Found Link header");

            // Parse the Link header (can have multiple values separated by comma)
            for link in link_str.split(',') {
                let link = link.trim();

                // Extract URL from <...>
                if let Some(url_start) = link.find('<') {
                    if let Some(url_end) = link.find('>') {
                        let link_url = &link[url_start + 1..url_end];

                        // Check rel type
                        if link.contains("rel=\"llms-full-txt\"")
                            || link.contains("rel=llms-full-txt")
                        {
                            llms_full_url = Some(resolve_link_url(url, link_url));
                        } else if link.contains("rel=\"llms-txt\"") || link.contains("rel=llms-txt")
                        {
                            llms_url = Some(resolve_link_url(url, link_url));
                        }
                    }
                }
            }
        }
    }

    // If we found Link header declarations, verify they actually exist
    if llms_full_url.is_some() || llms_url.is_some() {
        let (full_exists, llms_exists) = tokio::join!(
            async {
                if let Some(ref u) = llms_full_url {
                    probe_url_exists(client, u).await
                } else {
                    false
                }
            },
            async {
                if let Some(ref u) = llms_url {
                    probe_url_exists(client, u).await
                } else {
                    false
                }
            }
        );

        if full_exists || llms_exists {
            return Ok(Some(ProbeResult {
                domain: domain.to_string(),
                llms_full_url: if full_exists { llms_full_url } else { None },
                llms_url: if llms_exists { llms_url } else { None },
                sitemap_url: None,
                docs_subdomain_checked: false,
                discovery_method: DiscoveryMethod::LinkHeader,
                requires_confirmation: false,
                original_url: None,
            }));
        }
    }

    Ok(None)
}

/// Resolve a Link header URL relative to the base URL.
fn resolve_link_url(base_url: &str, link_url: &str) -> String {
    // Absolute URLs are returned as-is
    if link_url.starts_with("http://") || link_url.starts_with("https://") {
        return link_url.to_string();
    }

    // Try to resolve relative URLs (both absolute path and relative path)
    if let Ok(base) = Url::parse(base_url) {
        if let Ok(resolved) = base.join(link_url) {
            return resolved.to_string();
        }
    }

    // Fallback to the original link URL if resolution fails
    link_url.to_string()
}

/// Probe for llms files relative to a path.
///
/// Returns the domain without port in the result.
async fn probe_path_relative(
    client: &Client,
    base: &Url,
    path: &str,
) -> Result<Option<ProbeResult>> {
    let domain = base.host_str().unwrap_or("");

    // Build URLs relative to the path
    let llms_full_path = format!("{path}/llms-full.txt");
    let llms_path = format!("{path}/llms.txt");

    let llms_full_url = base
        .join(&llms_full_path)
        .map(|u| u.to_string())
        .unwrap_or_default();
    let llms_url = base
        .join(&llms_path)
        .map(|u| u.to_string())
        .unwrap_or_default();

    debug!(llms_full = %llms_full_url, llms = %llms_url, "Probing path-relative locations");

    let (full_exists, llms_exists) = tokio::join!(
        probe_url_exists(client, &llms_full_url),
        probe_url_exists(client, &llms_url),
    );

    if full_exists || llms_exists {
        return Ok(Some(ProbeResult {
            domain: domain.to_string(),
            llms_full_url: full_exists.then_some(llms_full_url),
            llms_url: llms_exists.then_some(llms_url),
            sitemap_url: None,
            docs_subdomain_checked: false,
            discovery_method: DiscoveryMethod::PathRelative,
            requires_confirmation: false,
            original_url: None,
        }));
    }

    Ok(None)
}

/// Extract the parent domain from a subdomain.
///
/// Returns `None` if the host is not a subdomain (e.g., `example.com`).
/// Returns the parent domain for subdomains (e.g., `code.claude.com` → `claude.com`).
fn extract_parent_domain(host: &str) -> Option<String> {
    let parts: Vec<&str> = host.split('.').collect();

    // Need at least 3 parts for a subdomain (e.g., sub.example.com)
    if parts.len() < 3 {
        return None;
    }

    // Skip the first part (subdomain) and join the rest
    Some(parts[1..].join("."))
}

/// Probe a single domain (without subdomain fallback).
async fn probe_single_domain(client: &Client, domain: &str) -> Result<ProbeResult> {
    probe_single_domain_with_method(client, domain, DiscoveryMethod::HostRoot).await
}

/// Probe a single domain with a specific discovery method.
async fn probe_single_domain_with_method(
    client: &Client,
    domain: &str,
    method: DiscoveryMethod,
) -> Result<ProbeResult> {
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

    let has_source = llms_full_exists || llms_exists || sitemap_exists;

    Ok(ProbeResult {
        domain: domain.to_string(),
        llms_full_url: llms_full_exists.then_some(llms_full_url),
        llms_url: llms_exists.then_some(llms_url),
        sitemap_url: sitemap_exists.then_some(sitemap_url),
        docs_subdomain_checked: false,
        discovery_method: if has_source {
            method
        } else {
            DiscoveryMethod::NotFound
        },
        requires_confirmation: false,
        original_url: None,
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

    fn make_probe_result(
        llms_full_url: Option<&str>,
        llms_url: Option<&str>,
        sitemap_url: Option<&str>,
    ) -> ProbeResult {
        ProbeResult {
            domain: "example.com".to_string(),
            llms_full_url: llms_full_url.map(String::from),
            llms_url: llms_url.map(String::from),
            sitemap_url: sitemap_url.map(String::from),
            docs_subdomain_checked: false,
            discovery_method: DiscoveryMethod::HostRoot,
            requires_confirmation: false,
            original_url: None,
        }
    }

    #[test]
    fn test_best_url_prefers_llms_full() {
        let result = make_probe_result(
            Some("https://example.com/llms-full.txt"),
            Some("https://example.com/llms.txt"),
            Some("https://example.com/sitemap.xml"),
        );

        assert_eq!(result.best_url(), Some("https://example.com/llms-full.txt"));
    }

    #[test]
    fn test_best_url_falls_back_to_llms() {
        let result = make_probe_result(
            None,
            Some("https://example.com/llms.txt"),
            Some("https://example.com/sitemap.xml"),
        );

        assert_eq!(result.best_url(), Some("https://example.com/llms.txt"));
    }

    #[test]
    fn test_best_url_falls_back_to_sitemap() {
        let result = make_probe_result(None, None, Some("https://example.com/sitemap.xml"));

        assert_eq!(result.best_url(), Some("https://example.com/sitemap.xml"));
    }

    #[test]
    fn test_best_url_returns_none_when_empty() {
        let mut result = make_probe_result(None, None, None);
        result.docs_subdomain_checked = true;
        result.discovery_method = DiscoveryMethod::NotFound;

        assert_eq!(result.best_url(), None);
    }

    #[test]
    fn test_has_source_with_llms_full() {
        let result = make_probe_result(Some("https://example.com/llms-full.txt"), None, None);

        assert!(result.has_source());
    }

    #[test]
    fn test_has_source_with_llms() {
        let result = make_probe_result(None, Some("https://example.com/llms.txt"), None);

        assert!(result.has_source());
    }

    #[test]
    fn test_has_source_with_sitemap() {
        let result = make_probe_result(None, None, Some("https://example.com/sitemap.xml"));

        assert!(result.has_source());
    }

    #[test]
    fn test_has_source_empty() {
        let mut result = make_probe_result(None, None, None);
        result.docs_subdomain_checked = true;

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

    // Tests for probe_url smart resolution

    #[tokio::test]
    async fn test_probe_url_finds_via_link_header() {
        let mock_server = MockServer::start().await;

        // Main page returns Link header pointing to llms-full.txt
        Mock::given(method("GET"))
            .and(path("/docs"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Link", "</docs/llms-full.txt>; rel=\"llms-full-txt\""),
            )
            .mount(&mock_server)
            .await;

        // The linked file exists
        Mock::given(method("HEAD"))
            .and(path("/docs/llms-full.txt"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let url = format!("{}/docs", mock_server.uri());
        let result = probe_url(&url).await.unwrap();

        assert!(result.llms_full_url.is_some());
        assert!(
            result
                .llms_full_url
                .as_ref()
                .unwrap()
                .contains("/docs/llms-full.txt")
        );
        assert_eq!(result.discovery_method, DiscoveryMethod::LinkHeader);
        assert!(!result.requires_confirmation);
    }

    #[tokio::test]
    async fn test_probe_url_finds_path_relative() {
        let mock_server = MockServer::start().await;

        // Main page exists but no Link header
        Mock::given(method("GET"))
            .and(path("/docs"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // Path-relative llms-full.txt exists
        Mock::given(method("HEAD"))
            .and(path("/docs/llms-full.txt"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        Mock::given(method("HEAD"))
            .and(path("/docs/llms.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let url = format!("{}/docs", mock_server.uri());
        let result = probe_url(&url).await.unwrap();

        assert!(result.llms_full_url.is_some());
        assert!(
            result
                .llms_full_url
                .as_ref()
                .unwrap()
                .contains("/docs/llms-full.txt")
        );
        assert_eq!(result.discovery_method, DiscoveryMethod::PathRelative);
    }

    #[tokio::test]
    async fn test_probe_url_falls_back_to_host_root() {
        let mock_server = MockServer::start().await;

        // Main page exists but no Link header
        Mock::given(method("GET"))
            .and(path("/docs"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // Nothing at path-relative location
        Mock::given(method("HEAD"))
            .and(path("/docs/llms-full.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        Mock::given(method("HEAD"))
            .and(path("/docs/llms.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        // But exists at host root
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

        let url = format!("{}/docs", mock_server.uri());
        let result = probe_url(&url).await.unwrap();

        assert!(result.llms_full_url.is_some());
        assert!(
            result
                .llms_full_url
                .as_ref()
                .unwrap()
                .contains("/llms-full.txt")
        );
        assert!(
            !result
                .llms_full_url
                .as_ref()
                .unwrap()
                .contains("/docs/llms-full.txt")
        );
        assert_eq!(result.discovery_method, DiscoveryMethod::HostRoot);
    }

    #[tokio::test]
    async fn test_probe_url_with_domain_only() {
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

        // Just pass the domain, no path
        let uri = mock_server.uri();
        let domain = uri.trim_start_matches("http://");

        let result = probe_url(domain).await.unwrap();

        // Should fall back to probe_domain behavior
        assert!(result.llms_full_url.is_some());
    }

    #[test]
    fn test_extract_parent_domain() {
        assert_eq!(
            extract_parent_domain("code.claude.com"),
            Some("claude.com".to_string())
        );
        assert_eq!(
            extract_parent_domain("docs.api.example.com"),
            Some("api.example.com".to_string())
        );
        assert_eq!(extract_parent_domain("example.com"), None);
        assert_eq!(extract_parent_domain("localhost"), None);
    }

    #[test]
    fn test_resolve_link_url_absolute() {
        let result = resolve_link_url("https://example.com/docs", "https://other.com/llms.txt");
        assert_eq!(result, "https://other.com/llms.txt");
    }

    #[test]
    fn test_resolve_link_url_absolute_path() {
        let result = resolve_link_url("https://example.com/docs", "/llms-full.txt");
        assert_eq!(result, "https://example.com/llms-full.txt");
    }

    #[test]
    fn test_resolve_link_url_relative() {
        let result = resolve_link_url("https://example.com/docs/", "llms-full.txt");
        assert_eq!(result, "https://example.com/docs/llms-full.txt");
    }

    #[test]
    fn test_discovery_method_default() {
        assert_eq!(DiscoveryMethod::default(), DiscoveryMethod::NotFound);
    }

    #[tokio::test]
    async fn test_probe_url_link_header_both_relations() {
        let mock_server = MockServer::start().await;

        // Page returns Link header with both llms.txt and llms-full.txt
        Mock::given(method("GET"))
            .and(path("/docs"))
            .respond_with(ResponseTemplate::new(200).insert_header(
                "Link",
                "</llms.txt>; rel=\"llms-txt\", </llms-full.txt>; rel=\"llms-full-txt\"",
            ))
            .mount(&mock_server)
            .await;

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

        let url = format!("{}/docs", mock_server.uri());
        let result = probe_url(&url).await.unwrap();

        assert!(result.llms_full_url.is_some());
        assert!(result.llms_url.is_some());
        assert_eq!(result.discovery_method, DiscoveryMethod::LinkHeader);
    }
}
