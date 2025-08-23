# Architecture Principles

## System Overview

The blz project is a Rust-based search cache system built on Tantivy, designed for high-performance full-text search with efficient caching and CLI integration.

## Architecture Patterns

### Workspace Structure

**Crate Organization**

```
cache/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── blz-core/             # Core search and indexing logic
│   │   ├── src/
│   │   │   ├── lib.rs        # Public API
│   │   │   ├── index.rs      # Tantivy index management
│   │   │   ├── search.rs     # Search operations
│   │   │   ├── cache.rs      # Caching layer
│   │   │   ├── config.rs     # Configuration
│   │   │   └── error.rs      # Error types
│   │   └── Cargo.toml
│   ├── blz-cli/              # Command-line interface
│   │   ├── src/
│   │   │   ├── main.rs       # CLI entry point
│   │   │   ├── commands/     # CLI commands
│   │   │   └── shell/        # Shell integration
│   │   └── Cargo.toml
│   └── blz-mcp/              # MCP server integration
│       ├── src/
│       │   ├── main.rs       # MCP server
│       │   ├── handlers/     # Request handlers
│       │   └── protocol/     # MCP protocol
│       └── Cargo.toml
└── docs/                     # Documentation
```

### Module Boundaries

**Clear Separation of Concerns**

1. **`blz-core`**: Business logic and domain models
   - No CLI dependencies
   - No I/O beyond what's necessary for indexing
   - Pure, testable functions where possible

2. **`blz-cli`**: User interface and shell integration
   - Depends on `blz-core`
   - Handles argument parsing and output formatting
   - Manages configuration files

3. **`blz-mcp`**: Network protocol and server logic
   - Depends on `blz-core`
   - Handles JSON-RPC protocol
   - Manages concurrent requests

### Dependency Flow

**Unidirectional Dependencies**

```
blz-cli ──┐
          ├──→ blz-core
blz-mcp ──┘
```

**Forbidden Patterns**

- Core cannot depend on CLI or MCP
- CLI and MCP should not depend on each other
- No circular dependencies between any crates

## Design Patterns

### Error Handling Strategy

**Layered Error Types**

```rust
// Core domain errors
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Index error: {0}")]
    Index(#[from] IndexError),
    #[error("Query error: {0}")]
    Query(#[from] QueryError),
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
}

// CLI-specific errors
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("Cache operation failed: {0}")]
    Cache(#[from] blz_core::CacheError),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Shell integration error: {0}")]
    Shell(String),
}

// Application-level error handling
fn main() -> Result<(), Box<dyn std::error::Error>> {
    match run() {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Error: {}", e);
            let mut source = e.source();
            while let Some(err) = source {
                eprintln!("  Caused by: {}", err);
                source = err.source();
            }
            std::process::exit(1);
        }
    }
}
```

### Configuration Management

**Layered Configuration**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    pub index: IndexConfig,
    pub search: SearchConfig,
    pub storage: StorageConfig,
}

impl CacheConfig {
    pub fn load() -> Result<Self, ConfigError> {
        // Priority order:
        // 1. Environment variables
        // 2. Configuration file
        // 3. Default values

        let mut config = Self::default();

        // Load from file if exists
        if let Some(config_path) = Self::find_config_file()? {
            let file_config = Self::load_from_file(&config_path)?;
            config = config.merge(file_config);
        }

        // Override with environment variables
        config = config.merge(Self::from_env()?);

        Ok(config)
    }
}
```

### Resource Management

**RAII and Smart Pointers**

```rust
pub struct SearchIndex {
    // Tantivy index - automatically closed on drop
    index: tantivy::Index,
    // Reader pool for concurrent searches
    reader_pool: Arc<ReaderPool>,
    // Background tasks handle
    _background_tasks: JoinHandle<()>,
}

impl Drop for SearchIndex {
    fn drop(&mut self) {
        // Graceful shutdown of background tasks
        if let Some(handle) = self._background_tasks.take() {
            handle.abort();
            // Wait for graceful shutdown with timeout
            let _ = futures::executor::block_on(async {
                tokio::time::timeout(
                    Duration::from_secs(5),
                    handle
                ).await
            });
        }
    }
}
```

### Concurrency Patterns

**Share-Nothing Architecture**

```rust
pub struct CacheManager {
    // Immutable shared state
    config: Arc<CacheConfig>,
    // Thread-safe index access
    index: Arc<SearchIndex>,
    // Request-scoped state
    request_context: Arc<RwLock<RequestContext>>,
}

impl CacheManager {
    pub async fn search(&self, query: String) -> CacheResult<SearchResults> {
        // Clone Arc for async move
        let index = Arc::clone(&self.index);
        let config = Arc::clone(&self.config);

        tokio::spawn(async move {
            index.search_with_config(&query, &config).await
        }).await?
    }
}
```

### Type-Driven Design

**Rich Domain Models**

```rust
/// A validated search query ready for execution
#[derive(Debug, Clone)]
pub struct ValidatedQuery {
    raw: String,
    parsed: Box<dyn tantivy::query::Query>,
    limits: QueryLimits,
}

/// Query execution limits to prevent resource exhaustion
#[derive(Debug, Clone)]
pub struct QueryLimits {
    max_results: NonZeroU32,
    timeout: Duration,
    max_memory_mb: NonZeroU32,
}

/// Search results with metadata
#[derive(Debug, Clone)]
pub struct SearchResults {
    hits: Vec<SearchHit>,
    total_count: u64,
    execution_time: Duration,
    from_cache: bool,
}

// Smart constructors prevent invalid states
impl ValidatedQuery {
    pub fn new(raw: String, schema: &Schema) -> Result<Self, QueryError> {
        let parser = QueryParser::for_index(&index, vec![]);
        let parsed = parser.parse_query(&raw)?;

        Ok(Self {
            raw,
            parsed: Box::new(parsed),
            limits: QueryLimits::default(),
        })
    }
}
```

## Integration Patterns

### Plugin Architecture

**Trait-Based Extensions**

```rust
pub trait CacheSource: Send + Sync {
    async fn fetch(&self, request: &FetchRequest) -> Result<Document, SourceError>;
    fn source_type(&self) -> SourceType;
    fn supports_streaming(&self) -> bool { false }
}

pub struct CacheRegistry {
    sources: HashMap<SourceType, Box<dyn CacheSource>>,
}

impl CacheRegistry {
    pub fn register<T>(&mut self, source: T)
    where
        T: CacheSource + 'static
    {
        self.sources.insert(source.source_type(), Box::new(source));
    }

    pub async fn fetch(&self, source_type: SourceType, request: &FetchRequest)
        -> Result<Document, SourceError>
    {
        let source = self.sources
            .get(&source_type)
            .ok_or(SourceError::UnknownSource(source_type))?;

        source.fetch(request).await
    }
}
```

### Event-Driven Updates

**Observer Pattern with Channels**

```rust
#[derive(Debug, Clone)]
pub enum CacheEvent {
    IndexUpdated { path: PathBuf, document_count: u64 },
    QueryExecuted { query: String, duration: Duration },
    CacheHit { query: String },
    CacheMiss { query: String },
}

pub struct EventBus {
    sender: broadcast::Sender<CacheEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CacheEvent> {
        self.sender.subscribe()
    }

    pub fn publish(&self, event: CacheEvent) {
        let _ = self.sender.send(event);
    }
}
```

## Data Flow Architecture

### Request Processing Pipeline

**Functional Pipeline**

```rust
pub struct SearchPipeline {
    validator: QueryValidator,
    cache: QueryCache,
    index: SearchIndex,
    formatter: ResultFormatter,
}

impl SearchPipeline {
    pub async fn execute(&self, raw_query: String) -> CacheResult<FormattedResults> {
        // Functional pipeline with error short-circuiting
        let validated = self.validator.validate(raw_query)?;

        // Try cache first
        if let Some(cached) = self.cache.get(&validated).await? {
            return Ok(self.formatter.format(cached, true)?);
        }

        // Execute search
        let results = self.index.search(&validated).await?;

        // Update cache in background
        let cache = self.cache.clone();
        let cache_key = validated.cache_key();
        let cache_results = results.clone();
        tokio::spawn(async move {
            let _ = cache.set(cache_key, cache_results).await;
        });

        Ok(self.formatter.format(results, false)?)
    }
}
```

### State Management

**Immutable State with COW**

```rust
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct IndexState {
    pub version: u64,
    pub document_count: u64,
    pub last_updated: SystemTime,
    pub schema_version: u32,
}

pub struct StateManager {
    current: Arc<RwLock<IndexState>>,
    history: Arc<RwLock<VecDeque<IndexState>>>,
}

impl StateManager {
    pub fn update<F>(&self, updater: F) -> Result<(), StateError>
    where
        F: FnOnce(&IndexState) -> Result<IndexState, StateError>
    {
        let mut current = self.current.write().unwrap();
        let mut history = self.history.write().unwrap();

        let new_state = updater(&current)?;

        // Keep history for rollback
        history.push_back(current.clone());
        if history.len() > 10 {
            history.pop_front();
        }

        *current = new_state;
        Ok(())
    }
}
```

## Quality Attributes

### Performance Requirements

- **Search Latency**: <10ms p95 for cached queries, <100ms for uncached
- **Throughput**: >1000 queries/second sustained load
- **Memory Usage**: <100MB baseline, <500MB under load
- **Startup Time**: <1s for CLI, <5s for full index load

### Scalability Requirements

- **Index Size**: Support up to 10GB indexes efficiently
- **Concurrent Users**: Handle 100+ concurrent search requests
- **Document Count**: Scale to 10M+ documents per index
- **Query Complexity**: Support complex boolean and phrase queries

### Reliability Requirements

- **Availability**: 99.9% uptime for MCP server mode
- **Error Rate**: <0.1% for valid queries
- **Data Integrity**: 100% consistency across crashes
- **Recovery Time**: <30s for service restart

### Security Requirements

- **Input Validation**: All query input sanitized and validated
- **Resource Limits**: Query execution bounded by time and memory
- **Access Control**: File system permissions respected
- **Audit Logging**: All operations logged for debugging

## Anti-Patterns

### Architecture Anti-Patterns

**Avoid These Patterns**

- **God Objects**: Keep services focused and single-purpose
- **Circular Dependencies**: Maintain clean dependency hierarchy
- **Shared Mutable State**: Prefer message passing over shared memory
- **Blocking in Async**: Never block async executor threads
- **Resource Leaks**: Always implement Drop for resources
- **Premature Abstraction**: Don't abstract until you have 3+ use cases

### Rust-Specific Anti-Patterns

- **Arc<Mutex<T>> Everywhere**: Use channels for communication when possible
- **Clone to Solve Borrow Issues**: Understand borrowing instead
- **Unsafe Without Documentation**: Any unsafe must have safety comments
- **Panic in Libraries**: Libraries should return Results, not panic

## Testing Architecture

### Test Organization

```rust
// Unit tests alongside code
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_validation() {
        // Unit test logic
    }
}

// Integration tests in tests/ directory
// tests/integration/search_pipeline.rs
#[tokio::test]
async fn test_end_to_end_search() {
    // Integration test logic
}

// Benchmarks in benches/ directory
// benches/search_performance.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn search_benchmark(c: &mut Criterion) {
    c.bench_function("search_common_query", |b| {
        b.iter(|| {
            // Benchmark logic
        })
    });
}
```

## Deployment Architecture

### Binary Distribution

- **Single Binary**: CLI tool as self-contained executable
- **Library Crate**: Core functionality available as library
- **MCP Server**: Standalone server mode for Claude Code integration
- **Shell Integration**: Fish/Bash/Zsh completion scripts

### Configuration Management

- **Environment Variables**: Runtime configuration
- **Config Files**: Persistent settings in TOML/JSON
- **CLI Arguments**: Override configuration for specific operations
- **Defaults**: Sensible defaults that work out of the box

Remember: Architecture should enable the team to move fast while maintaining quality. Every architectural decision should be justified by concrete requirements, not theoretical future needs.
