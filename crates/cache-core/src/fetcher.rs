use crate::{Error, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use reqwest::{Client, Response, StatusCode};
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
            return Err(Error::Network(
                reqwest::Error::from(response.error_for_status().unwrap_err())
            ));
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
            return Err(Error::Network(
                reqwest::Error::from(response.error_for_status().unwrap_err())
            ));
        }
        
        let content = response.text().await?;
        let sha256 = calculate_sha256(&content);
        
        Ok((content, sha256))
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

fn calculate_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    STANDARD.encode(result)
}

impl Default for Fetcher {
    fn default() -> Self {
        Self::new().expect("Failed to create fetcher")
    }
}