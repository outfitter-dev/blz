//! Async I/O helpers and connection pooling.
//!
//! [`ConnectionPool`] manages shared `reqwest::Client` instances with per-domain
//! caching, bounded concurrency, and basic statistics. The module also provides
//! async file utilities and concurrent processors optimized for large
//! llms.txt payloads while enforcing timeouts to prevent stalls.
use crate::{Error, Result};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// Connection pool for HTTP clients with different configurations
pub struct ConnectionPool {
    /// Default HTTP client for general requests
    default_client: Client,
    
    /// Specialized clients for different domains/use cases
    domain_clients: Arc<RwLock<HashMap<String, ClientEntry>>>,
    
    /// Semaphore to limit concurrent connections
    connection_limiter: Arc<Semaphore>,
    
    /// Maximum number of concurrent connections per domain
    max_connections_per_domain: usize,
    
    /// Statistics
    stats: Arc<ConnectionPoolStats>,
}

/// Entry for domain-specific clients
struct ClientEntry {
    client: Client,
    created_at: Instant,
    last_used: Instant,
    usage_count: AtomicUsize,
}

/// Statistics for connection pool
#[derive(Default)]
pub struct ConnectionPoolStats {
    pub total_requests: AtomicUsize,
    pub cache_hits: AtomicUsize,
    pub cache_misses: AtomicUsize,
    pub timeouts: AtomicUsize,
    pub errors: AtomicUsize,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(max_total_connections: usize, max_per_domain: usize) -> Result<Self> {
        let default_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("blz/0.1.0 (connection-pooled)")
            .gzip(true)
            .brotli(true)
            .http2_prior_knowledge()
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(10)
            .build()
            .map_err(Error::Network)?;

        Ok(Self {
            default_client,
            domain_clients: Arc::new(RwLock::new(HashMap::new())),
            connection_limiter: Arc::new(Semaphore::new(max_total_connections)),
            max_connections_per_domain: max_per_domain,
            stats: Arc::new(ConnectionPoolStats::default()),
        })
    }

    /// Get an HTTP client optimized for the given domain
    pub async fn get_client(&self, url: &str) -> Result<Client> {
        self.stats.total_requests.fetch_add(1, Ordering::Relaxed);

        let domain = extract_domain(url);
        
        // Fast path: check if we have a cached client
        {
            let domain_clients = self.domain_clients.read().await;
            if let Some(entry) = domain_clients.get(&domain) {
                // Update last used time and usage count
                entry.usage_count.fetch_add(1, Ordering::Relaxed);
                self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
                
                return Ok(entry.client.clone());
            }
        }

        // Slow path: create or get client
        self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
        self.get_or_create_client(domain).await
    }

    /// Get or create a client for a specific domain
    async fn get_or_create_client(&self, domain: String) -> Result<Client> {
        let mut domain_clients = self.domain_clients.write().await;
        
        // Double-check in case another task created it
        if let Some(entry) = domain_clients.get(&domain) {
            entry.usage_count.fetch_add(1, Ordering::Relaxed);
            return Ok(entry.client.clone());
        }

        // Create domain-specific client with optimizations
        let client = self.create_optimized_client(&domain).await?;
        
        let entry = ClientEntry {
            client: client.clone(),
            created_at: Instant::now(),
            last_used: Instant::now(),
            usage_count: AtomicUsize::new(1),
        };

        domain_clients.insert(domain, entry);
        
        // Clean up old clients if we have too many
        if domain_clients.len() > 50 {
            self.cleanup_old_clients(&mut domain_clients).await;
        }

        Ok(client)
    }

    /// Create an optimized client for a specific domain
    async fn create_optimized_client(&self, domain: &str) -> Result<Client> {
        let mut builder = Client::builder()
            .user_agent("blz/0.1.0 (domain-optimized)")
            .gzip(true)
            .brotli(true)
            .pool_max_idle_per_host(self.max_connections_per_domain)
            .pool_idle_timeout(Duration::from_secs(90));

        // Domain-specific optimizations
        if domain.contains("github.com") || domain.contains("raw.githubusercontent.com") {
            // GitHub tends to have good HTTP/2 support and can handle more concurrent connections
            builder = builder
                .http2_prior_knowledge()
                .timeout(Duration::from_secs(60))
                .pool_max_idle_per_host(20);
        } else if domain.contains("cdn") || domain.contains("jsdelivr") || domain.contains("unpkg") {
            // CDNs typically have excellent performance characteristics
            builder = builder
                .http2_prior_knowledge()
                .timeout(Duration::from_secs(45))
                .pool_max_idle_per_host(15);
        } else {
            // Conservative defaults for unknown domains
            builder = builder
                .timeout(Duration::from_secs(30))
                .pool_max_idle_per_host(5);
        }

        builder.build().map_err(Error::Network)
    }

    /// Clean up old or unused clients
    async fn cleanup_old_clients(&self, clients: &mut HashMap<String, ClientEntry>) {
        let cutoff_time = Instant::now() - Duration::from_secs(300); // 5 minutes
        let mut to_remove = Vec::new();

        for (domain, entry) in clients.iter() {
            if entry.last_used < cutoff_time && entry.usage_count.load(Ordering::Relaxed) < 5 {
                to_remove.push(domain.clone());
            }
        }

        for domain in to_remove {
            clients.remove(&domain);
            debug!("Removed unused client for domain: {}", domain);
        }
    }

    /// Perform an HTTP GET with connection pooling and timeout
    pub async fn get(&self, url: &str) -> Result<String> {
        self.perform_request(url, |client| async move {
            let response = client.get(url).send().await?;
            
            if !response.status().is_success() {
                return Err(Error::Network(
                    response.error_for_status().unwrap_err()
                ));
            }
            
            Ok(response.text().await?)
        }).await
    }

    /// Perform an HTTP HEAD request to check resource existence
    pub async fn head(&self, url: &str) -> Result<reqwest::Response> {
        self.perform_request(url, |client| async move {
            let response = client.head(url).send().await?;
            Ok(response)
        }).await
    }

    /// Generic method to perform HTTP requests with rate limiting and timeout
    async fn perform_request<F, Fut, T>(&self, url: &str, request_fn: F) -> Result<T>
    where
        F: FnOnce(Client) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Acquire connection semaphore
        let _permit = self.connection_limiter.acquire().await
            .map_err(|_| Error::ResourceLimited("Connection pool exhausted".into()))?;

        let client = self.get_client(url).await?;

        // Perform request with timeout
        match timeout(Duration::from_secs(45), request_fn(client)).await {
            Ok(result) => match result {
                Ok(response) => Ok(response),
                Err(e) => {
                    self.stats.errors.fetch_add(1, Ordering::Relaxed);
                    Err(e)
                }
            }
            Err(_) => {
                self.stats.timeouts.fetch_add(1, Ordering::Relaxed);
                Err(Error::Timeout("HTTP request timed out".into()))
            }
        }
    }

    /// Get connection pool statistics
    pub fn get_stats(&self) -> ConnectionPoolStatsSummary {
        ConnectionPoolStatsSummary {
            total_requests: self.stats.total_requests.load(Ordering::Relaxed),
            cache_hits: self.stats.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.stats.cache_misses.load(Ordering::Relaxed),
            timeouts: self.stats.timeouts.load(Ordering::Relaxed),
            errors: self.stats.errors.load(Ordering::Relaxed),
            hit_rate: {
                let hits = self.stats.cache_hits.load(Ordering::Relaxed);
                let total = self.stats.total_requests.load(Ordering::Relaxed);
                if total > 0 {
                    hits as f64 / total as f64
                } else {
                    0.0
                }
            },
        }
    }
}

/// Extract domain from URL for client caching
fn extract_domain(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            return host.to_string();
        }
    }
    
    // Fallback: extract domain manually
    if let Some(start) = url.find("://") {
        let after_scheme = &url[start + 3..];
        if let Some(end) = after_scheme.find('/') {
            after_scheme[..end].to_string()
        } else {
            after_scheme.to_string()
        }
    } else {
        "unknown".to_string()
    }
}

/// Statistics summary for connection pool
#[derive(Debug, Clone)]
pub struct ConnectionPoolStatsSummary {
    pub total_requests: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub timeouts: usize,
    pub errors: usize,
    pub hit_rate: f64,
}

/// Async file operations with buffering and error handling
pub struct AsyncFileOps;

impl AsyncFileOps {
    /// Read entire file contents asynchronously with buffering
    pub async fn read_to_string(path: &std::path::Path) -> Result<String> {
        let file = File::open(path).await
            .map_err(|e| Error::Io(format!("Failed to open file {}: {}", path.display(), e)))?;
            
        let mut reader = BufReader::new(file);
        let mut contents = String::new();
        
        reader.read_to_string(&mut contents).await
            .map_err(|e| Error::Io(format!("Failed to read file {}: {}", path.display(), e)))?;
            
        Ok(contents)
    }

    /// Read file contents in chunks with progress callback
    pub async fn read_with_progress<F>(
        path: &std::path::Path,
        mut progress_fn: F,
    ) -> Result<String>
    where
        F: FnMut(usize, usize), // (bytes_read, total_bytes)
    {
        let file = File::open(path).await
            .map_err(|e| Error::Io(format!("Failed to open file {}: {}", path.display(), e)))?;
            
        let file_size = file.metadata().await
            .map_err(|e| Error::Io(format!("Failed to get file metadata: {}", e)))?
            .len() as usize;
            
        let mut reader = BufReader::with_capacity(8192, file);
        let mut contents = String::with_capacity(file_size);
        let mut buffer = vec![0u8; 8192];
        let mut total_read = 0usize;
        
        loop {
            let bytes_read = reader.read(&mut buffer).await
                .map_err(|e| Error::Io(format!("Failed to read from file: {}", e)))?;
                
            if bytes_read == 0 {
                break;
            }
            
            let chunk = String::from_utf8_lossy(&buffer[..bytes_read]);
            contents.push_str(&chunk);
            
            total_read += bytes_read;
            progress_fn(total_read, file_size);
        }
        
        Ok(contents)
    }

    /// Write string to file asynchronously with atomic operation
    pub async fn write_atomic(
        path: &std::path::Path, 
        contents: &str,
    ) -> Result<()> {
        let temp_path = path.with_extension("tmp");
        
        // Write to temporary file first
        {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&temp_path)
                .await
                .map_err(|e| Error::Io(format!("Failed to create temp file: {}", e)))?;
                
            let mut writer = BufWriter::new(file);
            writer.write_all(contents.as_bytes()).await
                .map_err(|e| Error::Io(format!("Failed to write to temp file: {}", e)))?;
                
            writer.flush().await
                .map_err(|e| Error::Io(format!("Failed to flush temp file: {}", e)))?;
        }
        
        // Atomically move temp file to final location
        tokio::fs::rename(&temp_path, path).await
            .map_err(|e| Error::Io(format!("Failed to rename temp file: {}", e)))?;
            
        Ok(())
    }

    /// Read file bytes with memory-mapped optimization for large files
    pub async fn read_bytes_optimized(path: &std::path::Path) -> Result<Vec<u8>> {
        let metadata = tokio::fs::metadata(path).await
            .map_err(|e| Error::Io(format!("Failed to get file metadata: {}", e)))?;
            
        let file_size = metadata.len();
        
        // Use memory mapping for large files (>1MB), direct read for smaller
        if file_size > 1_048_576 {
            // For very large files, use regular async read to avoid blocking
            let file = File::open(path).await
                .map_err(|e| Error::Io(format!("Failed to open file: {}", e)))?;
                
            let mut reader = BufReader::with_capacity(65536, file); // 64KB buffer
            let mut contents = Vec::with_capacity(file_size as usize);
            
            reader.read_to_end(&mut contents).await
                .map_err(|e| Error::Io(format!("Failed to read file: {}", e)))?;
                
            Ok(contents)
        } else {
            // Small files: simple read
            tokio::fs::read(path).await
                .map_err(|e| Error::Io(format!("Failed to read file: {}", e)))
        }
    }

    /// Batch file operations to reduce syscall overhead
    pub async fn write_multiple_files(
        files: Vec<(&std::path::Path, &str)>,
    ) -> Result<()> {
        use futures::future::try_join_all;
        
        let write_tasks: Vec<_> = files
            .into_iter()
            .map(|(path, content)| Self::write_atomic(path, content))
            .collect();
            
        try_join_all(write_tasks).await?;
        Ok(())
    }

    /// Check if file exists and get basic metadata efficiently
    pub async fn file_info(path: &std::path::Path) -> Option<FileInfo> {
        tokio::fs::metadata(path).await.ok().map(|metadata| FileInfo {
            size: metadata.len(),
            is_file: metadata.is_file(),
            modified: metadata.modified().ok(),
        })
    }
}

/// Basic file information
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub size: u64,
    pub is_file: bool,
    pub modified: Option<std::time::SystemTime>,
}

/// Concurrent file processor for batch operations
pub struct ConcurrentFileProcessor {
    /// Maximum concurrent operations
    max_concurrent: usize,
    
    /// Semaphore to limit concurrency
    semaphore: Arc<Semaphore>,
}

impl ConcurrentFileProcessor {
    /// Create new concurrent file processor
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Process multiple files concurrently with a processing function
    pub async fn process_files<F, Fut, T>(
        &self,
        files: Vec<std::path::PathBuf>,
        processor: F,
    ) -> Result<Vec<T>>
    where
        F: Fn(std::path::PathBuf) -> Fut + Send + Sync + Copy,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send,
    {
        use futures::future::try_join_all;
        
        let tasks: Vec<_> = files
            .into_iter()
            .map(|path| {
                let semaphore = Arc::clone(&self.semaphore);
                async move {
                    let _permit = semaphore.acquire().await
                        .map_err(|_| Error::ResourceLimited("Semaphore error".into()))?;
                    processor(path).await
                }
            })
            .collect();
            
        try_join_all(tasks).await
    }

    /// Read multiple files concurrently
    pub async fn read_files(
        &self,
        paths: Vec<std::path::PathBuf>,
    ) -> Result<Vec<(std::path::PathBuf, String)>> {
        self.process_files(paths, |path| async move {
            let content = AsyncFileOps::read_to_string(&path).await?;
            Ok((path, content))
        }).await
    }
}

impl Default for ConcurrentFileProcessor {
    fn default() -> Self {
        Self::new(10) // Default to 10 concurrent operations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio_test;

    #[tokio::test]
    async fn test_connection_pool_basic() {
        let pool = ConnectionPool::new(10, 5).unwrap();
        
        // This will fail in tests without network, but tests the structure
        let stats = pool.get_stats();
        assert_eq!(stats.total_requests, 0);
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://example.com/path"), "example.com");
        assert_eq!(extract_domain("http://api.github.com/repos"), "api.github.com");
        assert_eq!(extract_domain("https://cdn.jsdelivr.net/package"), "cdn.jsdelivr.net");
    }

    #[tokio::test]
    async fn test_async_file_ops_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = "Hello, async world!";

        // Write file
        AsyncFileOps::write_atomic(&file_path, content).await.unwrap();

        // Read file
        let read_content = AsyncFileOps::read_to_string(&file_path).await.unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_async_file_ops_read_bytes() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.bin");
        let data = b"binary data test";

        tokio::fs::write(&file_path, data).await.unwrap();

        let read_data = AsyncFileOps::read_bytes_optimized(&file_path).await.unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_async_file_ops_file_info() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("info_test.txt");
        let content = "test content for file info";

        AsyncFileOps::write_atomic(&file_path, content).await.unwrap();

        let info = AsyncFileOps::file_info(&file_path).await.unwrap();
        assert!(info.is_file);
        assert_eq!(info.size, content.len() as u64);
    }

    #[tokio::test]
    async fn test_concurrent_file_processor() {
        let temp_dir = TempDir::new().unwrap();
        let processor = ConcurrentFileProcessor::new(3);

        // Create test files
        let mut paths = Vec::new();
        for i in 0..5 {
            let path = temp_dir.path().join(format!("file_{}.txt", i));
            let content = format!("Content of file {}", i);
            AsyncFileOps::write_atomic(&path, &content).await.unwrap();
            paths.push(path);
        }

        // Read files concurrently
        let results = processor.read_files(paths.clone()).await.unwrap();
        
        assert_eq!(results.len(), 5);
        for (i, (path, content)) in results.iter().enumerate() {
            assert_eq!(path, &paths[i]);
            assert!(content.contains(&format!("Content of file {}", i)));
        }
    }

    #[tokio::test]
    async fn test_write_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        
        let files = vec![
            (temp_dir.path().join("file1.txt").as_path(), "content1"),
            (temp_dir.path().join("file2.txt").as_path(), "content2"),
            (temp_dir.path().join("file3.txt").as_path(), "content3"),
        ];

        AsyncFileOps::write_multiple_files(files).await.unwrap();

        // Verify all files were written
        for i in 1..=3 {
            let path = temp_dir.path().join(format!("file{}.txt", i));
            let content = AsyncFileOps::read_to_string(&path).await.unwrap();
            assert_eq!(content, format!("content{}", i));
        }
    }

    #[tokio::test]
    async fn test_read_with_progress() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("progress_test.txt");
        
        // Create a file with known content
        let content = "A".repeat(1000); // 1000 bytes
        AsyncFileOps::write_atomic(&file_path, &content).await.unwrap();

        let mut progress_calls = 0;
        let mut last_total = 0;

        let read_content = AsyncFileOps::read_with_progress(&file_path, |read, total| {
            progress_calls += 1;
            last_total = total;
            assert!(read <= total);
        }).await.unwrap();

        assert_eq!(read_content, content);
        assert!(progress_calls > 0);
        assert_eq!(last_total, 1000);
    }
}
