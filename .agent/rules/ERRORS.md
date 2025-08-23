# Error Handling

## Error Handling Philosophy

Rust's type system enables explicit, comprehensive error handling. Use the Result pattern consistently, provide rich error context, and fail fast with clear error messages.

## Error Strategy: Hybrid thiserror + anyhow

Based on research findings, use a hybrid approach:

- **Libraries** (`cache-core`): Use `thiserror` for structured error types
- **Applications** (`cache-cli`, `cache-mcp`): Use `anyhow` for error handling

## Library Error Design (thiserror)

### Core Domain Errors

**Structured Error Types**

```rust
use thiserror::Error;

/// Core cache operation errors
#[derive(Error, Debug)]
pub enum CacheError {
    /// Query parsing failed
    #[error("Query parsing failed: '{query}' - {reason}")]
    QueryParsing {
        query: String,
        reason: String,
        #[source]
        source: Option<tantivy::query::QueryParserError>,
    },

    /// Index operation failed
    #[error("Index operation '{operation}' failed")]
    IndexOperation {
        operation: String,
        #[source]
        source: tantivy::TantivyError,
    },

    /// Search execution failed
    #[error("Search execution failed for query: '{query}'")]
    SearchExecution {
        query: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Cache storage error
    #[error("Cache storage operation failed: {operation}")]
    CacheStorage {
        operation: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Configuration error
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Resource limits exceeded
    #[error("Resource limit exceeded: {resource} = {current}, max = {limit}")]
    ResourceLimit {
        resource: String,
        current: u64,
        limit: u64,
    },

    /// Invalid input provided
    #[error("Invalid input: {field} - {reason}")]
    InvalidInput {
        field: String,
        reason: String,
    },

    /// Concurrent access conflict
    #[error("Concurrent access conflict: {operation}")]
    ConcurrencyConflict {
        operation: String,
    },

    /// System resource unavailable
    #[error("System resource unavailable: {resource}")]
    ResourceUnavailable {
        resource: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

/// Type alias for Results in the cache crate
pub type CacheResult<T> = Result<T, CacheError>;
```

### Error Construction Helpers

**Builder Pattern for Rich Errors**

```rust
impl CacheError {
    /// Create a query parsing error with context
    pub fn query_parsing<S: Into<String>>(
        query: S,
        reason: S,
        source: Option<tantivy::query::QueryParserError>,
    ) -> Self {
        Self::QueryParsing {
            query: query.into(),
            reason: reason.into(),
            source,
        }
    }

    /// Create an index operation error
    pub fn index_operation<S: Into<String>>(
        operation: S,
        source: tantivy::TantivyError,
    ) -> Self {
        Self::IndexOperation {
            operation: operation.into(),
            source,
        }
    }

    /// Create a resource limit error
    pub fn resource_limit<S: Into<String>>(
        resource: S,
        current: u64,
        limit: u64,
    ) -> Self {
        Self::ResourceLimit {
            resource: resource.into(),
            current,
            limit,
        }
    }

    /// Create an invalid input error
    pub fn invalid_input<S: Into<String>>(field: S, reason: S) -> Self {
        Self::InvalidInput {
            field: field.into(),
            reason: reason.into(),
        }
    }
}
```

### Specialized Error Types

**Query Processing Errors**

```rust
/// Specific errors for query processing
#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Empty query provided")]
    EmptyQuery,

    #[error("Query too long: {length} characters, max {max_length}")]
    TooLong { length: usize, max_length: usize },

    #[error("Invalid field name: '{field}' - {reason}")]
    InvalidField { field: String, reason: String },

    #[error("Unsupported query syntax: {syntax}")]
    UnsupportedSyntax { syntax: String },

    #[error("Boolean query too complex: depth {depth}, max {max_depth}")]
    TooComplex { depth: u32, max_depth: u32 },

    #[error("Invalid query parameter: {parameter} = '{value}' - {reason}")]
    InvalidParameter {
        parameter: String,
        value: String,
        reason: String,
    },
}

// Convert specialized errors to general cache errors
impl From<QueryError> for CacheError {
    fn from(err: QueryError) -> Self {
        match err {
            QueryError::EmptyQuery => CacheError::InvalidInput {
                field: "query".to_string(),
                reason: "Query cannot be empty".to_string(),
            },
            QueryError::TooLong { length, max_length } => CacheError::ResourceLimit {
                resource: "query_length".to_string(),
                current: length as u64,
                limit: max_length as u64,
            },
            other => CacheError::QueryParsing {
                query: "unknown".to_string(),
                reason: other.to_string(),
                source: None,
            },
        }
    }
}
```

**Index Management Errors**

```rust
/// Index-specific errors with detailed context
#[derive(Error, Debug)]
pub enum IndexError {
    #[error("Index not found at path: {path}")]
    NotFound { path: std::path::PathBuf },

    #[error("Index corrupted: {details}")]
    Corrupted {
        details: String,
        #[source]
        source: Option<tantivy::TantivyError>,
    },

    #[error("Index locked by another process: {path}")]
    Locked { path: std::path::PathBuf },

    #[error("Insufficient disk space: need {needed} bytes, available {available}")]
    InsufficientSpace { needed: u64, available: u64 },

    #[error("Schema mismatch: expected version {expected}, found {found}")]
    SchemaMismatch { expected: u32, found: u32 },

    #[error("Write operation failed: {operation}")]
    WriteFailed {
        operation: String,
        #[source]
        source: tantivy::TantivyError,
    },
}

impl From<IndexError> for CacheError {
    fn from(err: IndexError) -> Self {
        match err {
            IndexError::NotFound { path } => CacheError::Configuration {
                message: format!("Index directory not found: {}", path.display()),
                source: None,
            },
            IndexError::InsufficientSpace { needed, available } => {
                CacheError::ResourceLimit {
                    resource: "disk_space".to_string(),
                    current: available,
                    limit: needed,
                }
            }
            other => CacheError::IndexOperation {
                operation: "general".to_string(),
                source: tantivy::TantivyError::IoError {
                    io_error: std::io::Error::new(
                        std::io::ErrorKind::Other,
                        other.to_string()
                    ),
                    filepath: None,
                },
            },
        }
    }
}
```

## Application Error Handling (anyhow)

### CLI Application Errors

**Simple Error Handling with Context**

```rust
use anyhow::{Context, Result, bail, ensure};

/// CLI main function with comprehensive error handling
fn main() -> Result<()> {
    // Initialize logging first to capture errors
    init_logging()?;

    match run() {
        Ok(()) => Ok(()),
        Err(e) => {
            // Log the full error chain
            error!("Application failed: {:#}", e);

            // Print user-friendly error
            eprintln!("Error: {}", e);

            // Print error chain for debugging
            let mut source = e.source();
            while let Some(err) = source {
                eprintln!("  Caused by: {}", err);
                source = err.source();
            }

            std::process::exit(1);
        }
    }
}

fn run() -> Result<()> {
    let config = load_config()
        .context("Failed to load configuration")?;

    let cache = create_blz(&config)
        .context("Failed to initialize cache")?;

    let args = parse_args()
        .context("Failed to parse command line arguments")?;

    match args.command {
        Command::Search { query, limit } => {
            search_command(&cache, &query, limit)
                .context("Search command failed")?;
        }
        Command::Index { path } => {
            index_command(&cache, &path)
                .context("Index command failed")?;
        }
        Command::Clear => {
            clear_command(&cache)
                .context("Clear command failed")?;
        }
    }

    Ok(())
}
```

**Rich Error Context in Operations**

```rust
async fn search_command(cache: &SearchCache, query: &str, limit: u16) -> Result<()> {
    // Input validation with clear error messages
    ensure!(!query.trim().is_empty(), "Search query cannot be empty");
    ensure!(limit > 0, "Result limit must be greater than 0");
    ensure!(limit <= 1000, "Result limit cannot exceed 1000");

    // Execute search with context
    let results = cache
        .search(query, limit)
        .await
        .with_context(|| format!("Failed to search for: '{}'", query))?;

    // Check for empty results
    if results.hits.is_empty() {
        println!("No results found for '{}'", query);
        return Ok(());
    }

    // Display results with error handling
    display_results(&results)
        .with_context(|| format!("Failed to display {} results", results.hits.len()))?;

    Ok(())
}

async fn index_command(cache: &SearchCache, path: &std::path::Path) -> Result<()> {
    // Check path exists
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }

    if !path.is_file() {
        bail!("Path is not a file: {}", path.display());
    }

    // Read and validate file
    let content = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    if content.trim().is_empty() {
        bail!("File is empty: {}", path.display());
    }

    // Add to index
    cache
        .add_document_from_content(&content)
        .await
        .with_context(|| format!("Failed to index file: {}", path.display()))?;

    println!("Successfully indexed: {}", path.display());
    Ok(())
}
```

### MCP Server Error Handling

**JSON-RPC Error Responses**

```rust
use anyhow::{Context, Result};
use serde_json::Value;

/// MCP server error handling with proper JSON-RPC responses
async fn handle_search_request(params: Value) -> Result<Value> {
    // Parse parameters with validation
    let query: String = params
        .get("query")
        .and_then(|v| v.as_str())
        .context("Missing or invalid 'query' parameter")?
        .to_string();

    let limit: u16 = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as u16;

    // Validate parameters
    ensure!(
        !query.trim().is_empty(),
        "Query parameter cannot be empty"
    );
    ensure!(
        limit <= 1000,
        "Limit parameter cannot exceed 1000, got {}",
        limit
    );

    // Execute search with context
    let cache = get_global_blz()
        .context("Cache not initialized")?;

    let results = cache
        .search(&query, limit)
        .await
        .with_context(|| format!("Search failed for query: '{}'", query))?;

    // Convert to JSON response
    let response = serde_json::to_value(&results)
        .context("Failed to serialize search results")?;

    Ok(response)
}

/// Error conversion for JSON-RPC responses
fn convert_error_to_json_rpc(error: anyhow::Error) -> jsonrpc_core::Error {
    // Analyze error type and provide appropriate JSON-RPC error codes
    let error_code = if error.to_string().contains("not found") {
        -32602 // Invalid params
    } else if error.to_string().contains("timeout") {
        -32000 // Server error
    } else if error.to_string().contains("limit") {
        -32602 // Invalid params
    } else {
        -32603 // Internal error
    };

    jsonrpc_core::Error {
        code: error_code.into(),
        message: error.to_string(),
        data: Some(serde_json::json!({
            "error_chain": format!("{:#}", error)
        })),
    }
}
```

## Error Recovery Strategies

### Retry with Backoff

**Exponential Backoff for Transient Errors**

```rust
use std::time::Duration;
use tokio::time::sleep;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_factor: 2.0,
        }
    }
}

/// Retry a fallible async operation with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    mut operation: F,
    config: RetryConfig,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut delay = config.initial_delay;

    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                if attempt == config.max_attempts {
                    return Err(error);
                }

                // Log retry attempt
                debug!(
                    attempt = attempt,
                    delay = ?delay,
                    error = %error,
                    "Operation failed, retrying"
                );

                sleep(delay).await;

                // Calculate next delay with exponential backoff
                delay = std::cmp::min(
                    Duration::from_millis(
                        (delay.as_millis() as f64 * config.backoff_factor) as u64
                    ),
                    config.max_delay,
                );
            }
        }
    }

    unreachable!("Loop should have returned or broken");
}

// Usage example
async fn search_with_retry(cache: &SearchCache, query: &str) -> CacheResult<SearchResults> {
    retry_with_backoff(
        || cache.search(query, 10),
        RetryConfig::default(),
    ).await
}
```

### Circuit Breaker Pattern

**Fail Fast for Cascading Failures**

```rust
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing fast
    HalfOpen, // Testing if service recovered
}

/// Circuit breaker for protecting against cascading failures
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<AtomicU32>,
    last_failure_time: Arc<AtomicU64>,
    failure_threshold: u32,
    recovery_timeout: Duration,
    operation_timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            last_failure_time: Arc::new(AtomicU64::new(0)),
            failure_threshold,
            recovery_timeout,
            operation_timeout: Duration::from_secs(10),
        }
    }

    pub async fn execute<F, Fut, T>(&self, operation: F) -> CacheResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = CacheResult<T>>,
    {
        // Check circuit state
        match self.check_state().await {
            CircuitState::Open => {
                return Err(CacheError::ResourceUnavailable {
                    resource: "service".to_string(),
                    source: None,
                });
            }
            CircuitState::HalfOpen => {
                // Allow one test operation
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }

        // Execute operation with timeout
        let result = tokio::time::timeout(
            self.operation_timeout,
            operation()
        ).await;

        match result {
            Ok(Ok(value)) => {
                self.on_success().await;
                Ok(value)
            }
            Ok(Err(error)) => {
                self.on_failure().await;
                Err(error)
            }
            Err(_timeout) => {
                self.on_failure().await;
                Err(CacheError::ResourceUnavailable {
                    resource: "operation_timeout".to_string(),
                    source: None,
                })
            }
        }
    }

    async fn check_state(&self) -> CircuitState {
        let current_state = *self.state.read().await;

        match current_state {
            CircuitState::Open => {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);

                if now - last_failure >= self.recovery_timeout.as_secs() {
                    // Try to recover
                    *self.state.write().await = CircuitState::HalfOpen;
                    CircuitState::HalfOpen
                } else {
                    CircuitState::Open
                }
            }
            other => other,
        }
    }

    async fn on_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        *self.state.write().await = CircuitState::Closed;
    }

    async fn on_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        self.last_failure_time.store(now, Ordering::Relaxed);

        if failures >= self.failure_threshold {
            *self.state.write().await = CircuitState::Open;
        }
    }
}

// Usage in cache operations
pub struct ResilientSearchCache {
    cache: SearchCache,
    circuit_breaker: CircuitBreaker,
}

impl ResilientSearchCache {
    pub async fn search(&self, query: &str, limit: u16) -> CacheResult<SearchResults> {
        self.circuit_breaker
            .execute(|| self.cache.search(query, limit))
            .await
    }
}
```

## Error Logging and Monitoring

### Structured Error Logging

**Consistent Error Logging**

```rust
use tracing::{error, warn, info, debug, instrument};

#[instrument(skip(self), fields(query = %query, limit = limit))]
pub async fn search(&self, query: &str, limit: u16) -> CacheResult<SearchResults> {
    debug!("Starting search operation");

    // Validate input
    if query.trim().is_empty() {
        let error = CacheError::invalid_input("query", "Query cannot be empty");
        warn!(error = %error, "Invalid search query provided");
        return Err(error);
    }

    // Execute search
    match self.execute_search_internal(query, limit).await {
        Ok(results) => {
            info!(
                result_count = results.hits.len(),
                execution_time = ?results.execution_time,
                from_blz = results.from_blz,
                "Search completed successfully"
            );
            Ok(results)
        }
        Err(error) => {
            error!(
                error = %error,
                error_chain = format!("{:#}", error),
                "Search operation failed"
            );
            Err(error)
        }
    }
}
```

### Error Metrics

**Prometheus-Style Metrics**

```rust
use std::sync::Arc;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Error metrics collector
pub struct ErrorMetrics {
    error_counts: Arc<RwLock<HashMap<String, AtomicU64>>>,
    total_errors: AtomicU64,
}

impl ErrorMetrics {
    pub fn new() -> Self {
        Self {
            error_counts: Arc::new(RwLock::new(HashMap::new())),
            total_errors: AtomicU64::new(0),
        }
    }

    pub fn record_error(&self, error: &CacheError) {
        self.total_errors.fetch_add(1, Ordering::Relaxed);

        let error_type = match error {
            CacheError::QueryParsing { .. } => "query_parsing",
            CacheError::IndexOperation { .. } => "index_operation",
            CacheError::SearchExecution { .. } => "search_execution",
            CacheError::CacheStorage { .. } => "cache_storage",
            CacheError::Configuration { .. } => "configuration",
            CacheError::ResourceLimit { .. } => "resource_limit",
            CacheError::InvalidInput { .. } => "invalid_input",
            CacheError::ConcurrencyConflict { .. } => "concurrency_conflict",
            CacheError::ResourceUnavailable { .. } => "resource_unavailable",
        };

        let mut counts = self.error_counts.write().unwrap();
        counts
            .entry(error_type.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_error_counts(&self) -> HashMap<String, u64> {
        let counts = self.error_counts.read().unwrap();
        counts
            .iter()
            .map(|(k, v)| (k.clone(), v.load(Ordering::Relaxed)))
            .collect()
    }
}

// Integration with cache operations
impl SearchCache {
    pub async fn search_with_metrics(
        &self,
        query: &str,
        limit: u16,
        metrics: &ErrorMetrics,
    ) -> CacheResult<SearchResults> {
        match self.search(query, limit).await {
            Ok(results) => Ok(results),
            Err(error) => {
                metrics.record_error(&error);
                Err(error)
            }
        }
    }
}
```

## Testing Error Scenarios

### Error Path Testing

**Comprehensive Error Testing**

```rust
#[cfg(test)]
mod error_tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case("", CacheError::InvalidInput { .. })]
    #[case("   ", CacheError::InvalidInput { .. })]
    #[case(&"x".repeat(10000), CacheError::ResourceLimit { .. })]
    async fn test_invalid_query_inputs(
        #[case] query: &str,
        #[case] expected_error: CacheError,
    ) {
        let cache = create_test_blz().await;

        let result = cache.search(query, 10).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), expected_error));
    }

    #[tokio::test]
    async fn test_index_corruption_handling() {
        let cache = create_test_blz().await;

        // Simulate index corruption
        corrupt_index_files(&cache.index_path()).await;

        let result = cache.search("test", 10).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CacheError::IndexOperation { operation, .. } => {
                assert!(operation.contains("search"));
            }
            other => panic!("Expected IndexOperation error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_concurrent_error_handling() {
        let cache = Arc::new(create_test_blz().await);

        // Launch concurrent operations that will fail
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let cache = Arc::clone(&cache);
                tokio::spawn(async move {
                    cache.search("invalid:query::", 10).await
                })
            })
            .collect();

        let results = futures::future::join_all(handles).await;

        // All operations should fail gracefully
        for result in results {
            let search_result = result.unwrap();
            assert!(search_result.is_err());
        }
    }
}
```

## Error Handling Anti-Patterns

### Avoid These Patterns

**Common Error Handling Mistakes**

```rust
// ❌ Swallowing errors silently
pub async fn search(&self, query: &str) -> Option<SearchResults> {
    match self.cache.search(query, 10).await {
        Ok(results) => Some(results),
        Err(_) => None, // Information is lost!
    }
}

// ✅ Proper error propagation
pub async fn search(&self, query: &str) -> CacheResult<SearchResults> {
    self.cache.search(query, 10).await
        .map_err(|e| CacheError::SearchExecution {
            query: query.to_string(),
            source: Box::new(e),
        })
}

// ❌ Generic error messages
return Err("something went wrong".into());

// ✅ Specific, actionable error messages
return Err(CacheError::Configuration {
    message: format!(
        "Invalid index path '{}': directory does not exist or is not readable",
        path.display()
    ),
    source: Some(Box::new(io_error)),
});

// ❌ Using panic for recoverable errors
pub fn parse_query(input: &str) -> ParsedQuery {
    if input.is_empty() {
        panic!("Query cannot be empty!"); // Don't panic in libraries!
    }
    // ...
}

// ✅ Return Result for recoverable errors
pub fn parse_query(input: &str) -> Result<ParsedQuery, QueryError> {
    if input.is_empty() {
        return Err(QueryError::EmptyQuery);
    }
    // ...
}
```

Remember: Good error handling provides clear information about what went wrong, why it happened, and what the user can do about it. Errors should be structured, logged appropriately, and recovered from when possible.
