# Security Practices

## Security Philosophy

Security in Rust leverages the language's memory safety guarantees while adding defense-in-depth practices for input validation, resource limits, and access control.

## Memory Safety

### Safe Rust by Default

**Forbid Unsafe Code**

```toml
# Cargo.toml workspace configuration
[workspace.lints.rust]
unsafe_code = "forbid"
```

**Justified Unsafe Code**

```rust
// Only use unsafe when absolutely necessary and document safety requirements
/// # Safety
///
/// This function is safe to call when:
/// 1. `ptr` is valid and points to initialized memory
/// 2. The memory region is at least `len` bytes long
/// 3. No other code is accessing this memory concurrently
/// 4. The memory remains valid for the lifetime 'a
#[allow(unsafe_code)]
unsafe fn read_raw_bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    // Safety: Caller guarantees ptr is valid and len is correct
    std::slice::from_raw_parts(ptr, len)
}

// Better: Use safe alternatives when possible
fn read_bytes_safe(buffer: &[u8], offset: usize, len: usize) -> Result<&[u8], SecurityError> {
    buffer
        .get(offset..offset + len)
        .ok_or(SecurityError::BufferOverflow {
            offset,
            len,
            buffer_size: buffer.len()
        })
}
```

### Memory Management

**Resource Limits**

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Memory usage tracker to prevent memory exhaustion
pub struct MemoryTracker {
    current_usage: AtomicUsize,
    max_allowed: usize,
}

impl MemoryTracker {
    pub fn new(max_memory_mb: usize) -> Self {
        Self {
            current_usage: AtomicUsize::new(0),
            max_allowed: max_memory_mb * 1024 * 1024,
        }
    }

    pub fn allocate(&self, size: usize) -> Result<MemoryGuard, SecurityError> {
        let current = self.current_usage.load(Ordering::Relaxed);

        if current + size > self.max_allowed {
            return Err(SecurityError::MemoryLimit {
                requested: size,
                current: current,
                max: self.max_allowed,
            });
        }

        // Atomic check-and-update to prevent race conditions
        loop {
            let current = self.current_usage.load(Ordering::Acquire);
            if current + size > self.max_allowed {
                return Err(SecurityError::MemoryLimit {
                    requested: size,
                    current: current,
                    max: self.max_allowed,
                });
            }

            if self.current_usage
                .compare_exchange_weak(current, current + size, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }

        Ok(MemoryGuard {
            tracker: self,
            size,
        })
    }

    fn deallocate(&self, size: usize) {
        self.current_usage.fetch_sub(size, Ordering::Release);
    }
}

/// RAII guard for memory allocations
pub struct MemoryGuard<'a> {
    tracker: &'a MemoryTracker,
    size: usize,
}

impl Drop for MemoryGuard<'_> {
    fn drop(&mut self) {
        self.tracker.deallocate(self.size);
    }
}
```

## Input Validation

### Query Sanitization

**Comprehensive Input Validation**

```rust
use regex::Regex;
use once_cell::sync::Lazy;

/// Security error types
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Input validation failed: {field} - {reason}")]
    InputValidation { field: String, reason: String },

    #[error("Resource limit exceeded: {resource}")]
    ResourceLimit { resource: String },

    #[error("Memory limit exceeded: requested {requested}, current {current}, max {max}")]
    MemoryLimit { requested: usize, current: usize, max: usize },

    #[error("Rate limit exceeded: {requests} requests in {window:?}")]
    RateLimit { requests: u32, window: std::time::Duration },

    #[error("Access denied: {reason}")]
    AccessDenied { reason: String },

    #[error("Buffer overflow: offset {offset}, len {len}, buffer size {buffer_size}")]
    BufferOverflow { offset: usize, len: usize, buffer_size: usize },
}

// Compile regex patterns once at startup
static SAFE_QUERY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9\s\-_:()\"*+]+$").unwrap()
});

static SAFE_FIELD_NAME: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]{0,63}$").unwrap()
});

/// Query validator with security checks
pub struct QueryValidator {
    max_query_length: usize,
    max_terms: usize,
    max_nesting_depth: usize,
    allowed_fields: std::collections::HashSet<String>,
}

impl QueryValidator {
    pub fn new() -> Self {
        let mut allowed_fields = std::collections::HashSet::new();
        allowed_fields.insert("title".to_string());
        allowed_fields.insert("body".to_string());
        allowed_fields.insert("tags".to_string());
        allowed_fields.insert("url".to_string());
        allowed_fields.insert("author".to_string());

        Self {
            max_query_length: 1000,
            max_terms: 50,
            max_nesting_depth: 10,
            allowed_fields,
        }
    }

    /// Validate and sanitize a search query
    pub fn validate_query(&self, query: &str) -> Result<ValidatedQuery, SecurityError> {
        // Length check
        if query.len() > self.max_query_length {
            return Err(SecurityError::InputValidation {
                field: "query".to_string(),
                reason: format!(
                    "Query too long: {} chars, max {}",
                    query.len(),
                    self.max_query_length
                ),
            });
        }

        // Empty query check
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(SecurityError::InputValidation {
                field: "query".to_string(),
                reason: "Query cannot be empty".to_string(),
            });
        }

        // Character allowlist check
        if !SAFE_QUERY_PATTERN.is_match(trimmed) {
            return Err(SecurityError::InputValidation {
                field: "query".to_string(),
                reason: "Query contains unsafe characters".to_string(),
            });
        }

        // Parse and validate structure
        let parsed = self.parse_query_safely(trimmed)?;

        // Check complexity limits
        self.check_query_complexity(&parsed)?;

        Ok(ValidatedQuery {
            original: query.to_string(),
            sanitized: trimmed.to_string(),
            parsed,
        })
    }

    fn parse_query_safely(&self, query: &str) -> Result<ParsedQuery, SecurityError> {
        let mut term_count = 0;
        let mut nesting_depth = 0;
        let mut max_depth = 0;

        // Simple recursive descent parser with limits
        self.parse_expression(query, &mut term_count, &mut nesting_depth, &mut max_depth)?;

        if term_count > self.max_terms {
            return Err(SecurityError::InputValidation {
                field: "query".to_string(),
                reason: format!("Too many terms: {}, max {}", term_count, self.max_terms),
            });
        }

        if max_depth > self.max_nesting_depth {
            return Err(SecurityError::InputValidation {
                field: "query".to_string(),
                reason: format!(
                    "Query too complex: depth {}, max {}",
                    max_depth,
                    self.max_nesting_depth
                ),
            });
        }

        // Use a real parser here (this is simplified)
        Ok(ParsedQuery::Term(query.to_string()))
    }

    fn parse_expression(
        &self,
        expr: &str,
        term_count: &mut usize,
        current_depth: &mut usize,
        max_depth: &mut usize,
    ) -> Result<(), SecurityError> {
        *current_depth += 1;
        *max_depth = (*max_depth).max(*current_depth);

        // Check for field queries and validate field names
        if let Some(colon_pos) = expr.find(':') {
            let field_name = &expr[..colon_pos].trim();
            if !SAFE_FIELD_NAME.is_match(field_name) {
                return Err(SecurityError::InputValidation {
                    field: "field_name".to_string(),
                    reason: format!("Invalid field name: '{}'", field_name),
                });
            }

            if !self.allowed_fields.contains(*field_name) {
                return Err(SecurityError::InputValidation {
                    field: "field_name".to_string(),
                    reason: format!("Field not allowed: '{}'", field_name),
                });
            }
        }

        *term_count += 1;
        *current_depth -= 1;
        Ok(())
    }
}

/// Validated query that has passed security checks
#[derive(Debug, Clone)]
pub struct ValidatedQuery {
    pub original: String,
    pub sanitized: String,
    pub parsed: ParsedQuery,
}
```

### File System Security

**Safe Path Handling**

```rust
use std::path::{Path, PathBuf};

/// Secure path operations
pub struct SecurePath;

impl SecurePath {
    /// Validate and canonicalize a path to prevent directory traversal
    pub fn validate_path(path: &Path, allowed_base: &Path) -> Result<PathBuf, SecurityError> {
        // Canonicalize paths to resolve .. and . components
        let canonical_path = path.canonicalize()
            .map_err(|e| SecurityError::InputValidation {
                field: "path".to_string(),
                reason: format!("Invalid path: {}", e),
            })?;

        let canonical_base = allowed_base.canonicalize()
            .map_err(|e| SecurityError::InputValidation {
                field: "base_path".to_string(),
                reason: format!("Invalid base path: {}", e),
            })?;

        // Ensure the path is within the allowed base directory
        if !canonical_path.starts_with(&canonical_base) {
            return Err(SecurityError::AccessDenied {
                reason: format!(
                    "Path '{}' is outside allowed directory '{}'",
                    canonical_path.display(),
                    canonical_base.display()
                ),
            });
        }

        Ok(canonical_path)
    }

    /// Safe file reading with size limits
    pub async fn read_file_safely(
        path: &Path,
        max_size: usize,
    ) -> Result<String, SecurityError> {
        // Check file size before reading
        let metadata = tokio::fs::metadata(path).await
            .map_err(|e| SecurityError::InputValidation {
                field: "file_path".to_string(),
                reason: format!("Cannot access file: {}", e),
            })?;

        if metadata.len() > max_size as u64 {
            return Err(SecurityError::ResourceLimit {
                resource: format!(
                    "File '{}' is {} bytes, max allowed {}",
                    path.display(),
                    metadata.len(),
                    max_size
                ),
            });
        }

        // Read file with timeout
        let content = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            tokio::fs::read_to_string(path)
        ).await
        .map_err(|_| SecurityError::ResourceLimit {
            resource: "File read timeout".to_string(),
        })?
        .map_err(|e| SecurityError::InputValidation {
            field: "file_content".to_string(),
            reason: format!("Cannot read file: {}", e),
        })?;

        Ok(content)
    }
}
```

## Resource Protection

### Rate Limiting

**Token Bucket Rate Limiter**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Token bucket for rate limiting
#[derive(Debug)]
pub struct TokenBucket {
    capacity: u32,
    tokens: u32,
    refill_rate: u32, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(capacity: u32, refill_rate: u32) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    pub fn try_consume(&mut self, tokens: u32) -> bool {
        self.refill();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let time_passed = now.duration_since(self.last_refill);
        let tokens_to_add = (time_passed.as_secs() as u32 * self.refill_rate)
            + ((time_passed.subsec_millis() * self.refill_rate) / 1000);

        if tokens_to_add > 0 {
            self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
            self.last_refill = now;
        }
    }
}

/// Rate limiter for API requests
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    default_capacity: u32,
    default_refill_rate: u32,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            default_capacity: requests_per_second * 10, // 10 second burst
            default_refill_rate: requests_per_second,
        }
    }

    pub async fn check_rate_limit(&self, client_id: &str) -> Result<(), SecurityError> {
        let mut buckets = self.buckets.write().await;

        let bucket = buckets
            .entry(client_id.to_string())
            .or_insert_with(|| {
                TokenBucket::new(self.default_capacity, self.default_refill_rate)
            });

        if bucket.try_consume(1) {
            Ok(())
        } else {
            Err(SecurityError::RateLimit {
                requests: self.default_capacity,
                window: Duration::from_secs(1),
            })
        }
    }
}
```

### Resource Limits

**Query Execution Limits**

```rust
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

/// Resource manager for query execution
pub struct QueryResourceManager {
    max_concurrent_queries: u32,
    current_queries: AtomicU32,
    max_query_duration: Duration,
    max_memory_per_query: usize,
}

impl QueryResourceManager {
    pub fn new() -> Self {
        Self {
            max_concurrent_queries: 100,
            current_queries: AtomicU32::new(0),
            max_query_duration: Duration::from_secs(30),
            max_memory_per_query: 100 * 1024 * 1024, // 100MB
        }
    }

    /// Execute a query with resource limits
    pub async fn execute_with_limits<F, T>(&self, query_fn: F) -> Result<T, SecurityError>
    where
        F: std::future::Future<Output = Result<T, CacheError>>,
    {
        // Check concurrent query limit
        let current = self.current_queries.fetch_add(1, Ordering::Relaxed);
        if current >= self.max_concurrent_queries {
            self.current_queries.fetch_sub(1, Ordering::Relaxed);
            return Err(SecurityError::ResourceLimit {
                resource: format!(
                    "Too many concurrent queries: {}/{}",
                    current + 1,
                    self.max_concurrent_queries
                ),
            });
        }

        // Execute with timeout
        let result = timeout(self.max_query_duration, query_fn).await;

        // Decrement counter
        self.current_queries.fetch_sub(1, Ordering::Relaxed);

        match result {
            Ok(query_result) => query_result.map_err(|e| SecurityError::InputValidation {
                field: "query_execution".to_string(),
                reason: e.to_string(),
            }),
            Err(_) => Err(SecurityError::ResourceLimit {
                resource: format!(
                    "Query timeout: exceeded {} seconds",
                    self.max_query_duration.as_secs()
                ),
            }),
        }
    }
}

/// RAII guard for query execution
pub struct QueryExecutionGuard<'a> {
    manager: &'a QueryResourceManager,
}

impl Drop for QueryExecutionGuard<'_> {
    fn drop(&mut self) {
        self.manager.current_queries.fetch_sub(1, Ordering::Relaxed);
    }
}
```

## Access Control

### File System Permissions

**Secure File Operations**

```rust
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Secure file operations with permission checks
pub struct SecureFileSystem;

impl SecureFileSystem {
    /// Create directory with secure permissions
    pub async fn create_secure_directory(path: &Path) -> Result<(), SecurityError> {
        tokio::fs::create_dir_all(path).await
            .map_err(|e| SecurityError::AccessDenied {
                reason: format!("Cannot create directory '{}': {}", path.display(), e),
            })?;

        // Set restrictive permissions (owner read/write/execute only)
        #[cfg(unix)]
        {
            let mut permissions = tokio::fs::metadata(path).await
                .map_err(|e| SecurityError::AccessDenied {
                    reason: format!("Cannot read directory permissions: {}", e),
                })?
                .permissions();

            permissions.set_mode(0o700); // rwx------

            tokio::fs::set_permissions(path, permissions).await
                .map_err(|e| SecurityError::AccessDenied {
                    reason: format!("Cannot set directory permissions: {}", e),
                })?;
        }

        Ok(())
    }

    /// Verify file permissions are secure
    pub async fn verify_secure_permissions(path: &Path) -> Result<(), SecurityError> {
        let metadata = tokio::fs::metadata(path).await
            .map_err(|e| SecurityError::AccessDenied {
                reason: format!("Cannot access file '{}': {}", path.display(), e),
            })?;

        #[cfg(unix)]
        {
            let permissions = metadata.permissions();
            let mode = permissions.mode();

            // Check that file is not world-readable or group-readable
            if mode & 0o044 != 0 {
                return Err(SecurityError::AccessDenied {
                    reason: format!(
                        "File '{}' has insecure permissions: {:o}",
                        path.display(),
                        mode
                    ),
                });
            }
        }

        Ok(())
    }
}
```

### Configuration Security

**Secure Configuration Loading**

```rust
use serde::Deserialize;
use std::env;
use std::path::PathBuf;

/// Secure configuration with environment variable support
#[derive(Debug, Deserialize)]
pub struct SecurityConfig {
    pub max_query_length: usize,
    pub max_concurrent_queries: u32,
    pub rate_limit_requests_per_second: u32,
    pub max_file_size_mb: usize,
    pub allowed_index_paths: Vec<PathBuf>,
    pub enable_query_logging: bool,
}

impl SecurityConfig {
    /// Load configuration from environment variables and config file
    pub fn load() -> Result<Self, SecurityError> {
        let mut config = Self::default();

        // Override with environment variables (prefixed with CACHE_)
        if let Ok(val) = env::var("CACHE_MAX_QUERY_LENGTH") {
            config.max_query_length = val.parse()
                .map_err(|_| SecurityError::InputValidation {
                    field: "CACHE_MAX_QUERY_LENGTH".to_string(),
                    reason: "Must be a valid number".to_string(),
                })?;
        }

        if let Ok(val) = env::var("CACHE_MAX_CONCURRENT_QUERIES") {
            config.max_concurrent_queries = val.parse()
                .map_err(|_| SecurityError::InputValidation {
                    field: "CACHE_MAX_CONCURRENT_QUERIES".to_string(),
                    reason: "Must be a valid number".to_string(),
                })?;
        }

        // Validate configuration values
        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> Result<(), SecurityError> {
        if self.max_query_length > 10_000 {
            return Err(SecurityError::InputValidation {
                field: "max_query_length".to_string(),
                reason: "Cannot exceed 10,000 characters".to_string(),
            });
        }

        if self.max_concurrent_queries > 1000 {
            return Err(SecurityError::InputValidation {
                field: "max_concurrent_queries".to_string(),
                reason: "Cannot exceed 1,000 concurrent queries".to_string(),
            });
        }

        if self.rate_limit_requests_per_second > 1000 {
            return Err(SecurityError::InputValidation {
                field: "rate_limit_requests_per_second".to_string(),
                reason: "Cannot exceed 1,000 requests per second".to_string(),
            });
        }

        // Validate allowed paths exist and are directories
        for path in &self.allowed_index_paths {
            if !path.exists() {
                return Err(SecurityError::InputValidation {
                    field: "allowed_index_paths".to_string(),
                    reason: format!("Path does not exist: {}", path.display()),
                });
            }

            if !path.is_dir() {
                return Err(SecurityError::InputValidation {
                    field: "allowed_index_paths".to_string(),
                    reason: format!("Path is not a directory: {}", path.display()),
                });
            }
        }

        Ok(())
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_query_length: 1000,
            max_concurrent_queries: 100,
            rate_limit_requests_per_second: 10,
            max_file_size_mb: 10,
            allowed_index_paths: vec![
                PathBuf::from("/tmp/cache-indices"),
                PathBuf::from("./indices"),
            ],
            enable_query_logging: false,
        }
    }
}
```

## Dependency Security

### Cargo Audit Integration

**Security Scanning**

```toml
# .github/workflows/security.yml
name: Security Audit

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]
  schedule:
    - cron: '0 0 * * *' # Daily

jobs:
  security_audit:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable

    - name: Install cargo-audit
      run: cargo install --force cargo-audit

    - name: Run cargo audit
      run: cargo audit

    - name: Run cargo deny
      run: |
        cargo install --force cargo-deny
        cargo deny check
```

**Cargo Deny Configuration**

```toml
# deny.toml
[licenses]
allow = ["MIT", "Apache-2.0", "BSD-3-Clause", "ISC"]
deny = ["GPL-2.0", "GPL-3.0", "AGPL-1.0", "AGPL-3.0"]

[bans]
multiple-versions = "warn"
wildcards = "deny"
deny = [
    # Deny specific crates known to have issues
    { name = "openssl-sys", use-instead = "rustls" },
    # Add known problematic crates here
]

[advisories]
vulnerability = "deny"
unmaintained = "warn"
unsound = "warn"
yanked = "deny"
notice = "warn"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

### Secure Dependency Patterns

**Minimal Dependencies**

```toml
# Prefer established, well-maintained crates
[dependencies]
# Prefer std library when possible
# Use serde for serialization (well-established)
serde = { version = "1.0", features = ["derive"] }

# Use thiserror for error handling (simple, focused)
thiserror = "1.0"

# Use tokio for async (industry standard)
tokio = { version = "1.0", features = ["rt-multi-thread", "fs", "net"] }

# Use tantivy for search (specialized, well-maintained)
tantivy = "0.21"

# Avoid unnecessary dependencies
# Don't include "uuid" if you can use simpler ID generation
# Don't include "chrono" if std::time suffices
# Don't include "reqwest" for simple HTTP if you can avoid it
```

## Security Testing

### Security Test Suite

**Fuzzing and Property Testing**

```rust
#[cfg(test)]
mod security_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn query_validation_never_panics(query in any::<String>()) {
            let validator = QueryValidator::new();
            // Should never panic, even with malicious input
            let _ = validator.validate_query(&query);
        }

        #[test]
        fn path_validation_prevents_traversal(
            path in r"[./\\]+[a-zA-Z0-9_-]*"
        ) {
            let base = std::path::Path::new("/safe/directory");
            let test_path = std::path::Path::new(&path);

            // Should never allow access outside base directory
            let result = SecurePath::validate_path(test_path, base);
            if let Ok(validated_path) = result {
                assert!(validated_path.starts_with(base));
            }
        }

        #[test]
        fn memory_tracker_prevents_overflow(
            allocations in prop::collection::vec(1usize..1000000, 1..100)
        ) {
            let tracker = MemoryTracker::new(10); // 10MB limit
            let mut guards = Vec::new();
            let mut total_allocated = 0;

            for size in allocations {
                match tracker.allocate(size) {
                    Ok(guard) => {
                        total_allocated += size;
                        guards.push(guard);

                        // Should never exceed limit
                        assert!(total_allocated <= 10 * 1024 * 1024);
                    }
                    Err(_) => {
                        // Allocation rejected - that's fine
                        break;
                    }
                }
            }
        }
    }

    #[test]
    fn test_malicious_query_inputs() {
        let validator = QueryValidator::new();

        let malicious_inputs = vec![
            // SQL injection attempts
            "'; DROP TABLE users; --",
            "' OR 1=1 --",

            // Directory traversal attempts
            "../../../etc/passwd",
            "..\\..\\..\\windows\\system32",

            // Script injection attempts
            "<script>alert('xss')</script>",
            "javascript:alert('xss')",

            // Buffer overflow attempts
            &"A".repeat(100_000),

            // Null byte injection
            "query\0/etc/passwd",

            // Unicode normalization attacks
            "query\u{202e}gnirts",
        ];

        for input in malicious_inputs {
            let result = validator.validate_query(input);
            assert!(result.is_err(), "Should reject malicious input: {}", input);
        }
    }

    #[test]
    fn test_resource_exhaustion_protection() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let manager = QueryResourceManager::new();

        rt.block_on(async {
            // Try to launch more queries than allowed
            let mut handles = Vec::new();

            for i in 0..150 { // More than max_concurrent_queries (100)
                handles.push(tokio::spawn({
                    let manager = &manager;
                    async move {
                        manager.execute_with_limits(async {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            Ok::<_, CacheError>(i)
                        }).await
                    }
                }));
            }

            let results = futures::future::join_all(handles).await;

            // Some requests should be rejected due to limits
            let rejected_count = results
                .into_iter()
                .filter_map(|r| r.ok())
                .filter(|r| r.is_err())
                .count();

            assert!(rejected_count > 0, "Should reject some requests due to limits");
        });
    }
}
```

## Security Anti-Patterns

### Avoid These Patterns

**Common Security Mistakes**

```rust
// ❌ Trusting user input without validation
pub fn search(query: String) -> SearchResults {
    // Direct use of user input - dangerous!
    execute_raw_query(&query)
}

// ✅ Always validate input
pub fn search(query: String) -> Result<SearchResults, SecurityError> {
    let validated = QueryValidator::new().validate_query(&query)?;
    execute_safe_query(validated)
}

// ❌ Unbounded resource usage
pub async fn process_file(path: &Path) -> Result<String, Error> {
    // Could read gigabyte files and exhaust memory
    tokio::fs::read_to_string(path).await
}

// ✅ Bounded resource usage
pub async fn process_file(path: &Path) -> Result<String, SecurityError> {
    SecurePath::read_file_safely(path, 10 * 1024 * 1024).await // 10MB limit
}

// ❌ Logging sensitive information
pub async fn authenticate(password: &str) -> Result<User, Error> {
    debug!("Authenticating with password: {}", password); // Logged!
    // ...
}

// ✅ Secure logging
pub async fn authenticate(password: &str) -> Result<User, Error> {
    debug!("Authentication attempt for user"); // No sensitive data
    // ...
}
```

Remember: Security is a process, not a destination. Regularly audit your code, update dependencies, and assume that attackers will find creative ways to exploit any weakness.
