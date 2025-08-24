use crate::{Error, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::header::{CONTENT_LENGTH, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use reqwest::{Client, StatusCode};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tracing::{debug, info};

pub struct Fetcher {
    client: Client,
}

impl Fetcher {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("outfitter-cache/0.1.0")
            .gzip(true)
            .brotli(true)
            .build()
            .map_err(|e| Error::Network(e))?;

        Ok(Self { client })
    }

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
            return Ok(FetchResult::NotModified);
        }

        if !status.is_success() {
            return Err(Error::Network(reqwest::Error::from(
                response.error_for_status().unwrap_err(),
            )));
        }

        let new_etag = response
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let new_last_modified = response
            .headers()
            .get(LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

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

    pub async fn fetch(&self, url: &str) -> Result<(String, String)> {
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(Error::Network(reqwest::Error::from(
                response.error_for_status().unwrap_err(),
            )));
        }

        let content = response.text().await?;
        let sha256 = calculate_sha256(&content);

        Ok((content, sha256))
    }

    /// Check for available llms.txt flavors
    pub async fn check_flavors(&self, url: &str) -> Result<Vec<FlavorInfo>> {
        let mut flavors = Vec::new();
        let base_url = extract_base_url(url);

        // List of possible flavors to check
        let flavor_names = vec![
            "llms-full.txt",
            "llms.txt",
            "llms-mini.txt",
            "llms-base.txt",
        ];

        for flavor_name in flavor_names {
            let flavor_url = format!("{}/{}", base_url, flavor_name);

            // Make HEAD request to check if file exists and get size
            match self.client.head(&flavor_url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let size = response
                            .headers()
                            .get(CONTENT_LENGTH)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok());

                        flavors.push(FlavorInfo {
                            name: flavor_name.to_string(),
                            size,
                            url: flavor_url,
                        });
                    }
                },
                Err(e) => {
                    debug!("Failed to check flavor {}: {}", flavor_name, e);
                    // If it's the original URL provided by user, still add it even if HEAD fails
                    if url.ends_with(flavor_name) {
                        flavors.push(FlavorInfo {
                            name: flavor_name.to_string(),
                            size: None,
                            url: url.to_string(),
                        });
                    }
                },
            }
        }

        // If the user provided a specific llms.txt variant, make sure it's in the list
        if let Some(filename) = url.split('/').last() {
            if filename.starts_with("llms")
                && filename.ends_with(".txt")
                && !flavors.iter().any(|f| f.name == filename)
            {
                flavors.push(FlavorInfo {
                    name: filename.to_string(),
                    size: None,
                    url: url.to_string(),
                });
            }
        }

        // Sort flavors by preference: llms-full.txt > llms.txt > others
        flavors.sort_by(|a, b| {
            let order_a = match a.name.as_str() {
                "llms-full.txt" => 0,
                "llms.txt" => 1,
                "llms-mini.txt" => 2,
                "llms-base.txt" => 3,
                _ => 4,
            };
            let order_b = match b.name.as_str() {
                "llms-full.txt" => 0,
                "llms.txt" => 1,
                "llms-mini.txt" => 2,
                "llms-base.txt" => 3,
                _ => 4,
            };
            order_a.cmp(&order_b)
        });

        Ok(flavors)
    }
}

pub enum FetchResult {
    NotModified,
    Modified {
        content: String,
        etag: Option<String>,
        last_modified: Option<String>,
        sha256: String,
    },
}

#[derive(Debug, Clone)]
pub struct FlavorInfo {
    pub name: String,
    pub size: Option<u64>,
    pub url: String,
}

impl std::fmt::Display for FlavorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(size) = self.size {
            write!(f, "{} ({})", self.name, format_size(size))
        } else {
            write!(f, "{}", self.name)
        }
    }
}

fn calculate_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    STANDARD.encode(result)
}

fn extract_base_url(url: &str) -> String {
    // Simply remove the filename from the URL
    if let Some(last_slash) = url.rfind('/') {
        // Special case: if this is just the scheme separator, keep the full URL
        if url.len() > 3 && &url[last_slash - 2..last_slash + 1] == "://" {
            url.to_string()
        } else {
            url[..last_slash].to_string()
        }
    } else {
        url.to_string()
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

impl Default for Fetcher {
    fn default() -> Self {
        Self::new().expect("Failed to create fetcher")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_base_url() {
        assert_eq!(
            extract_base_url("https://example.com/llms.txt"),
            "https://example.com"
        );
        assert_eq!(
            extract_base_url("https://api.example.com/v1/docs/llms.txt"),
            "https://api.example.com/v1/docs"
        );
        assert_eq!(
            extract_base_url("https://example.com/"),
            "https://example.com"
        );
        assert_eq!(
            extract_base_url("https://example.com"),
            "https://example.com"
        );
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1_048_576), "1.0 MB");
        assert_eq!(format_size(1_572_864), "1.5 MB");
        assert_eq!(format_size(1_073_741_824), "1.0 GB");
        assert_eq!(format_size(2_147_483_648), "2.0 GB");
    }

    #[test]
    fn test_flavor_info_display() {
        let flavor_with_size = FlavorInfo {
            name: "llms-full.txt".to_string(),
            size: Some(892_000),
            url: "https://example.com/llms-full.txt".to_string(),
        };
        assert_eq!(format!("{}", flavor_with_size), "llms-full.txt (871.1 KB)");

        let flavor_no_size = FlavorInfo {
            name: "llms.txt".to_string(),
            size: None,
            url: "https://example.com/llms.txt".to_string(),
        };
        assert_eq!(format!("{}", flavor_no_size), "llms.txt");
    }
}
