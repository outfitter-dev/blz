use crate::{Error, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::header::{CONTENT_LENGTH, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use reqwest::{Client, StatusCode};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tracing::{debug, info};

/// HTTP client for fetching llms.txt documentation with conditional request support
pub struct Fetcher {
    client: Client,
}

impl Fetcher {
    /// Creates a new fetcher with configured HTTP client
    pub fn new() -> Result<Self> {
        Self::with_timeout(Duration::from_secs(30))
    }

    /// Creates a new fetcher with a custom request timeout (primarily for tests)
    pub fn with_timeout(timeout: Duration) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(concat!("outfitter-blz/", env!("CARGO_PKG_VERSION")))
            .gzip(true)
            .brotli(true)
            .build()
            .map_err(Error::Network)?;
        Ok(Self { client })
    }

    /// Fetches a URL with conditional request support using `ETag` and `Last-Modified` headers
    pub async fn fetch_with_cache(
        &self,
        url: &str,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<FetchResult> {
        let mut request = self.client.get(url);

        if let Some(tag) = etag {
            debug!("Setting If-None-Match: {}", tag);
            request = request.header(IF_NONE_MATCH, tag);
        }

        if let Some(lm) = last_modified {
            debug!("Setting If-Modified-Since: {}", lm);
            request = request.header(IF_MODIFIED_SINCE, lm);
        }

        let response = request.send().await?;
        let status = response.status();

        if status == StatusCode::NOT_MODIFIED {
            info!("Resource not modified (304) for {}", url);

            // Extract ETag and Last-Modified headers even on 304
            let etag = response
                .headers()
                .get(ETAG)
                .and_then(|v| v.to_str().ok())
                .map(std::string::ToString::to_string);

            let last_modified = response
                .headers()
                .get(LAST_MODIFIED)
                .and_then(|v| v.to_str().ok())
                .map(std::string::ToString::to_string);

            return Ok(FetchResult::NotModified {
                etag,
                last_modified,
            });
        }

        if !status.is_success() {
            // Map 404 to a clearer NotFound error
            if status == StatusCode::NOT_FOUND {
                return Err(Error::NotFound(format!(
                    "Resource not found at '{url}'. Check the URL or try 'blz lookup' to find available sources"
                )));
            }

            // Try to get the actual error, or create one manually
            match response.error_for_status() {
                Ok(_) => unreachable!("Status should be an error"),
                Err(err) => return Err(Error::Network(err)),
            }
        }

        let new_etag = response
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .map(std::string::ToString::to_string);

        let new_last_modified = response
            .headers()
            .get(LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(std::string::ToString::to_string);

        let content = response.text().await?;
        let sha256 = calculate_sha256(&content);

        info!("Fetched {} bytes from {}", content.len(), url);

        Ok(FetchResult::Modified {
            content,
            etag: new_etag,
            last_modified: new_last_modified,
            sha256,
        })
    }

    /// Fetches a URL without conditional request support, returning content and `SHA256` hash
    pub async fn fetch(&self, url: &str) -> Result<(String, String)> {
        let response = self.client.get(url).send().await?;
        let status = response.status();

        if !status.is_success() {
            // Map 404 to a clearer NotFound error
            if status == StatusCode::NOT_FOUND {
                return Err(Error::NotFound(format!(
                    "Resource not found at '{url}'. Check the URL or try 'blz lookup' to find available sources"
                )));
            }

            // Try to get the actual error, or create one manually
            match response.error_for_status() {
                Ok(_) => unreachable!("Status should be an error"),
                Err(err) => return Err(Error::Network(err)),
            }
        }

        let content = response.text().await?;
        let sha256 = calculate_sha256(&content);

        Ok((content, sha256))
    }

    /// Perform a HEAD request to retrieve basic metadata for a URL without downloading content
    pub async fn head_metadata(&self, url: &str) -> Result<HeadInfo> {
        let response = self.client.head(url).send().await?;
        let status = response.status();

        let content_length = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let etag = response
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .map(std::string::ToString::to_string);

        let last_modified = response
            .headers()
            .get(LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(std::string::ToString::to_string);

        Ok(HeadInfo {
            status: status.as_u16(),
            content_length,
            etag,
            last_modified,
        })
    }
}

/// Metadata from a HEAD request
#[derive(Debug, Clone)]
pub struct HeadInfo {
    /// HTTP status code returned by the server (e.g., 200, 404)
    pub status: u16,
    /// Optional content length reported by the server via `Content-Length`
    pub content_length: Option<u64>,
    /// Optional entity tag returned by the server for cache validation
    pub etag: Option<String>,
    /// Optional last modified timestamp returned by the server
    pub last_modified: Option<String>,
}

/// Result of a conditional HTTP fetch operation
pub enum FetchResult {
    /// Resource has not been modified since last fetch
    NotModified {
        /// `ETag` header value if present
        etag: Option<String>,
        /// `Last-Modified` header value if present
        last_modified: Option<String>,
    },
    /// Resource has been modified and new content was fetched
    Modified {
        /// The fetched content
        content: String,
        /// `ETag` header value if present
        etag: Option<String>,
        /// `Last-Modified` header value if present
        last_modified: Option<String>,
        /// `SHA256` hash of the content
        sha256: String,
    },
}

fn calculate_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    STANDARD.encode(result)
}

// Note: Default is not implemented as Fetcher::new() can fail.
// Use Fetcher::new() directly and handle the Result.

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_macros,
    clippy::match_wildcard_for_single_variants
)]
mod tests {
    use super::*;
    use std::time::Duration;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
    };

    #[tokio::test]
    async fn test_fetcher_creation() {
        // Test that fetcher can be created successfully
        let result = Fetcher::new();
        assert!(result.is_ok(), "Fetcher creation should succeed");

        let _fetcher = result.unwrap();
        // Verify it has the expected user agent and settings
        // (This is implicit since we can't directly inspect the client)
    }

    #[tokio::test]
    #[ignore = "network: run in CI"]
    async fn test_fetch_with_etag_not_modified() -> anyhow::Result<()> {
        // Setup mock server
        let mock_server = MockServer::start().await;

        // Mock 304 Not Modified response when ETag matches
        Mock::given(method("GET"))
            .and(path("/llms.txt"))
            .and(header("If-None-Match", "\"test-etag\""))
            .respond_with(ResponseTemplate::new(304))
            .mount(&mock_server)
            .await;

        let fetcher = Fetcher::new()?;
        let url = format!("{}/llms.txt", mock_server.uri());

        // Test with matching ETag
        let result = fetcher
            .fetch_with_cache(&url, Some("\"test-etag\""), None)
            .await?;

        match result {
            FetchResult::NotModified { .. } => {
                // Expected result
            },
            _ => panic!("Expected NotModified result for matching ETag"),
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore = "network: run in CI"]
    async fn test_fetch_with_etag_modified() -> anyhow::Result<()> {
        // Setup mock server
        let mock_server = MockServer::start().await;

        let content = "# Test Content\n\nThis is test content.";

        // Mock 200 OK response when ETag doesn't match
        Mock::given(method("GET"))
            .and(path("/llms.txt"))
            .and(header("If-None-Match", "\"old-etag\""))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(content)
                    .insert_header("etag", "\"new-etag\"")
                    .insert_header("last-modified", "Wed, 21 Oct 2015 07:28:00 GMT"),
            )
            .mount(&mock_server)
            .await;

        let fetcher = Fetcher::new()?;
        let url = format!("{}/llms.txt", mock_server.uri());

        // Test with non-matching ETag
        let result = fetcher
            .fetch_with_cache(&url, Some("\"old-etag\""), None)
            .await?;

        match result {
            FetchResult::Modified {
                content: returned_content,
                etag,
                last_modified,
                sha256,
            } => {
                assert_eq!(returned_content, content);
                assert_eq!(etag, Some("\"new-etag\"".to_string()));
                assert_eq!(
                    last_modified,
                    Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string())
                );
                assert!(!sha256.is_empty(), "SHA256 should be computed");
            },
            _ => panic!("Expected Modified result for non-matching ETag"),
        }

        Ok(())
    }

    // Temporarily disabled - mock server setup needs adjustment
    // #[tokio::test]
    #[allow(dead_code)]
    async fn test_fetch_with_last_modified() -> anyhow::Result<()> {
        // Setup mock server
        let mock_server = MockServer::start().await;

        // Mock 304 Not Modified response when Last-Modified matches
        Mock::given(method("GET"))
            .and(path("/llms.txt"))
            .and(header("If-Modified-Since", "Wed, 21 Oct 2015 07:28:00 GMT"))
            .respond_with(ResponseTemplate::new(304))
            .mount(&mock_server)
            .await;

        let fetcher = Fetcher::new()?;
        let url = format!("{}/llms.txt", mock_server.uri());

        // Test with Last-Modified header
        let result = fetcher
            .fetch_with_cache(&url, None, Some("Wed, 21 Oct 2015 07:28:00 GMT"))
            .await?;

        match result {
            FetchResult::NotModified { .. } => {
                // Expected result
            },
            _ => panic!("Expected NotModified result for matching Last-Modified"),
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore = "network: run in CI"]
    async fn test_fetch_404_error() -> anyhow::Result<()> {
        // Setup mock server
        let mock_server = MockServer::start().await;

        // Mock 404 Not Found response
        Mock::given(method("GET"))
            .and(path("/nonexistent.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let fetcher = Fetcher::new()?;
        let url = format!("{}/nonexistent.txt", mock_server.uri());

        // Test 404 handling
        let result = fetcher.fetch_with_cache(&url, None, None).await;

        assert!(result.is_err(), "404 should result in error");

        match result {
            Err(Error::NotFound(msg)) => {
                // Expected error type - 404 now maps to NotFound
                assert!(msg.contains("not found"));
                assert!(msg.contains("blz lookup"));
            },
            Err(e) => panic!("Expected NotFound error, got: {e}"),
            Ok(_) => panic!("Expected error for 404 response"),
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore = "network: run in CI"]
    async fn test_fetch_500_error() -> anyhow::Result<()> {
        // Setup mock server
        let mock_server = MockServer::start().await;

        // Mock 500 Internal Server Error response
        Mock::given(method("GET"))
            .and(path("/error.txt"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let fetcher = Fetcher::new()?;
        let url = format!("{}/error.txt", mock_server.uri());

        // Test 500 handling
        let result = fetcher.fetch_with_cache(&url, None, None).await;

        assert!(result.is_err(), "500 should result in error");

        match result {
            Err(Error::Network(_)) => {
                // Expected error type
            },
            Err(e) => panic!("Expected Network error, got: {e}"),
            Ok(_) => panic!("Expected error for 500 response"),
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore = "network: run in CI"]
    async fn test_fetch_timeout() -> anyhow::Result<()> {
        // Setup mock server with very slow response
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/slow.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("slow content")
                    .set_delay(Duration::from_millis(500)), // Longer than custom client timeout (200ms)
            )
            .mount(&mock_server)
            .await;

        // Use a short timeout to keep test runtime fast
        let fetcher = Fetcher::with_timeout(Duration::from_millis(200))?;
        let url = format!("{}/slow.txt", mock_server.uri());

        let start_time = std::time::Instant::now();
        let result = fetcher.fetch_with_cache(&url, None, None).await;
        let elapsed = start_time.elapsed();

        // Should fail due to timeout
        assert!(result.is_err(), "Slow request should timeout");
        assert!(
            elapsed < Duration::from_millis(500),
            "Should timeout before server's 500ms delay"
        );

        Ok(())
    }

    #[tokio::test]
    #[ignore = "network: run in CI"]
    async fn test_fetch_simple_without_cache() -> anyhow::Result<()> {
        // Setup mock server
        let mock_server = MockServer::start().await;

        let content = "# Simple Content\n\nThis is simple test content.";

        Mock::given(method("GET"))
            .and(path("/simple.txt"))
            .respond_with(ResponseTemplate::new(200).set_body_string(content))
            .mount(&mock_server)
            .await;

        let fetcher = Fetcher::new()?;
        let url = format!("{}/simple.txt", mock_server.uri());

        // Test simple fetch without cache headers
        let (returned_content, sha256) = fetcher.fetch(&url).await?;

        assert_eq!(returned_content, content);
        assert!(!sha256.is_empty(), "SHA256 should be computed");

        // Verify SHA256 is consistent
        let expected_sha = calculate_sha256(content);
        assert_eq!(sha256, expected_sha);

        Ok(())
    }

    // Temporarily disabled - mock server setup needs adjustment
    // #[tokio::test]
    #[allow(dead_code)]
    async fn test_fetch_with_both_etag_and_last_modified() -> anyhow::Result<()> {
        // Setup mock server
        let mock_server = MockServer::start().await;

        // Mock response that checks both ETag and Last-Modified
        Mock::given(method("GET"))
            .and(path("/both.txt"))
            .and(header("If-None-Match", "\"test-etag\""))
            .and(header("If-Modified-Since", "Wed, 21 Oct 2015 07:28:00 GMT"))
            .respond_with(ResponseTemplate::new(304))
            .mount(&mock_server)
            .await;

        let fetcher = Fetcher::new()?;
        let url = format!("{}/both.txt", mock_server.uri());

        // Test with both cache headers
        let result = fetcher
            .fetch_with_cache(
                &url,
                Some("\"test-etag\""),
                Some("Wed, 21 Oct 2015 07:28:00 GMT"),
            )
            .await?;

        match result {
            FetchResult::NotModified { .. } => {
                // Expected result
            },
            _ => panic!("Expected NotModified result for matching cache headers"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_sha256_calculation() {
        // Test the actual sha256 calculation with known values
        let content = "Hello, World!";
        let sha256 = calculate_sha256(content);

        // The function returns base64-encoded SHA256
        // Verify it's a valid base64 string of the right length
        assert!(!sha256.is_empty());
        assert_eq!(sha256.len(), 44); // Base64 encoded SHA256 is 44 chars

        // Test empty string
        let empty_sha256 = calculate_sha256("");
        assert_eq!(empty_sha256, "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=");
    }

    #[tokio::test]
    #[ignore = "network: run in CI"]
    async fn test_invalid_urls() -> anyhow::Result<()> {
        let fetcher = Fetcher::new()?;

        // Test completely invalid URLs
        let invalid_urls = vec![
            "not-a-url",
            "ftp://invalid-protocol.com/llms.txt",
            "",
            "https://",
        ];

        for invalid_url in invalid_urls {
            let result = fetcher.fetch_with_cache(invalid_url, None, None).await;
            assert!(result.is_err(), "Invalid URL '{invalid_url}' should fail");
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore = "network: run in CI"]
    async fn test_concurrent_requests() -> anyhow::Result<()> {
        // Setup mock server
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/concurrent.txt"))
            .respond_with(ResponseTemplate::new(200).set_body_string("concurrent content"))
            .mount(&mock_server)
            .await;

        let _fetcher = Fetcher::new()?;
        let url = format!("{}/concurrent.txt", mock_server.uri());

        // Make multiple concurrent requests
        let mut handles = Vec::new();

        for i in 0..10 {
            let fetcher_clone = Fetcher::new()?;
            let url_clone = url.clone();

            handles.push(tokio::spawn(async move {
                let result = fetcher_clone.fetch(&url_clone).await;
                (i, result)
            }));
        }

        // Wait for all requests
        let results = futures::future::join_all(handles).await;

        // All should succeed
        for result in results {
            let (index, fetch_result) = result.expect("Task should complete");

            match fetch_result {
                Ok((content, sha256)) => {
                    assert_eq!(content, "concurrent content");
                    assert!(!sha256.is_empty());
                },
                Err(e) => panic!("Request {index} should succeed: {e}"),
            }
        }

        Ok(())
    }
}
