# blz-core Agent Guide

This crate contains the core search functionality for the blz project. It provides the fundamental building blocks for indexing, searching, and managing documentation caches.

## Architecture Overview

- **Performance-Critical**: This is the hot path for all search operations
- **Target Latency**: Sub-10ms search response times
- **Memory Management**: Careful allocation patterns, some unsafe code for optimization
- **Async-First**: All I/O operations are async using tokio

## Key Modules

- **`index.rs`**: Tantivy search index management and querying
- **`parser.rs`**: Tree-sitter based markdown parsing
- **`fetcher.rs`**: HTTP client with ETag support and conditional fetching  
- **`storage.rs`**: Local filesystem operations with atomic writes
- **`cache.rs`**: In-memory caching layer with LRU eviction
- **`memory_pool.rs`**: Custom memory allocation for hot paths
- **`registry.rs`**: Source management and configuration

## Performance Considerations

### Memory Allocation Patterns

```rust
// ✅ GOOD: Pre-allocate collections when size is known
let mut results = Vec::with_capacity(expected_count);

// ✅ GOOD: Use string pools for repeated allocations
let pool = StringPool::new();
let cached_str = pool.get_or_intern("repeated_value");

// ❌ BAD: Allocating in hot paths
for item in items {
    let vec = Vec::new(); // New allocation every iteration!
}
```

### Async Patterns for High Performance

```rust
// ✅ GOOD: Concurrent operations with bounded parallelism
use tokio::sync::Semaphore;

pub async fn search_multiple_indices(
    queries: Vec<String>,
    indices: &[SearchIndex],
) -> Result<Vec<SearchResults>> {
    let semaphore = Arc::new(Semaphore::new(10)); // Limit concurrency
    
    let futures = queries.into_iter().map(|query| {
        let permit = semaphore.clone();
        let indices = indices.to_vec();
        
        async move {
            let _permit = permit.acquire().await?;
            search_all_indices(&query, &indices).await
        }
    });
    
    futures::future::try_join_all(futures).await
}
```

### Memory Pool Usage

The `memory_pool.rs` module contains unsafe code for performance. When working with it:

```rust
// SAFETY REQUIREMENTS when modifying memory_pool.rs:
// 1. All pointers must be valid for their declared lifetime
// 2. No data races - ensure exclusive access during mutation
// 3. Proper cleanup in Drop implementations
// 4. Document all invariants with // SAFETY: comments

unsafe fn link_node(&mut self, node: NonNull<Node<T>>) {
    // SAFETY: node pointer is valid, obtained from Box::into_raw
    // and we have exclusive access through &mut self
    let node_ref = &mut *node.as_ptr();
    // ... rest of implementation
}
```

## Error Handling Patterns

### Propagation Strategy

```rust
// ✅ GOOD: Use ? operator with context
use anyhow::Context;

pub async fn search_with_context(query: &str) -> Result<SearchResults> {
    let parsed = parse_query(query)
        .context("Failed to parse search query")?;
    
    let results = execute_search(&parsed).await
        .context("Search execution failed")?;
    
    Ok(results)
}
```

### Performance vs Error Detail Trade-offs

```rust
// For hot paths, prefer simple error types
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Invalid query")]
    InvalidQuery,
    #[error("Index unavailable")]
    IndexUnavailable,
    #[error("Timeout")]
    Timeout,
}

// For configuration/setup, use detailed errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid index path: {path}")]
    InvalidPath { path: String },
    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },
}
```

## Testing Patterns

### Performance Tests

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use criterion::black_box;
    
    #[tokio::test]
    async fn bench_search_latency() {
        let index = create_test_index().await;
        let query = "test query";
        
        let start = std::time::Instant::now();
        let _results = index.search(black_box(query), 10).await.unwrap();
        let duration = start.elapsed();
        
        // Assert performance requirement
        assert!(duration.as_millis() < 10, "Search took {}ms", duration.as_millis());
    }
}
```

### Async Test Patterns

```rust
#[tokio::test]
async fn test_concurrent_searches() {
    let index = Arc::new(create_test_index().await);
    
    let tasks: Vec<_> = (0..100).map(|i| {
        let index = Arc::clone(&index);
        tokio::spawn(async move {
            index.search(&format!("query {}", i), 5).await
        })
    }).collect();
    
    let results = futures::future::try_join_all(tasks).await.unwrap();
    
    // All searches should succeed
    assert!(results.iter().all(|r| r.is_ok()));
}
```

## Common Agent Tasks

### Adding New Search Features

1. **Modify the query parser** in `parser.rs`
2. **Update the search index** in `index.rs` 
3. **Add performance tests** in `tests/`
4. **Update benchmarks** in `benches/`
5. **Document performance characteristics**

### Memory Optimization

1. **Profile with flamegraph**: `cargo flamegraph --bench search_performance`
2. **Check allocations with dhat**: Enable `dhat-heap` feature
3. **Use memory pools for hot paths**
4. **Prefer `&str` over `String` when possible**

### Adding New Data Sources

1. **Implement `SourceProvider` trait**
2. **Add to registry in `registry.rs`**
3. **Write integration tests**
4. **Document any performance implications**

## Common Gotchas

### Tantivy Integration

```rust
// ❌ BAD: Creating reader on every search
pub async fn search(&self, query: &str) -> Result<SearchResults> {
    let reader = self.index.reader()?; // Expensive!
    let searcher = reader.searcher();
    // ...search
}

// ✅ GOOD: Reuse readers with periodic refresh
pub struct SearchIndex {
    reader: Arc<IndexReader>,
    last_refresh: Instant,
}

impl SearchIndex {
    pub async fn search(&self, query: &str) -> Result<SearchResults> {
        // Refresh reader if needed (infrequent)
        if self.last_refresh.elapsed() > Duration::from_secs(30) {
            self.refresh_reader();
        }
        
        let searcher = self.reader.searcher();
        // ...search
    }
}
```

### Async File I/O

```rust
// ✅ GOOD: Use tokio for all file operations
use tokio::fs;

pub async fn load_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path).await
        .context("Failed to read config file")?;
    
    toml::from_str(&content)
        .context("Failed to parse config")
}

// ❌ BAD: Don't mix std::fs with async
pub async fn bad_load_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)?; // Blocks executor!
    // ...
}
```

## Development Workflow

### Running Tests
```bash
# Unit tests only
cargo test -p blz-core --lib

# Integration tests  
cargo test -p blz-core --test '*'

# Benchmarks
cargo bench -p blz-core
```

### Performance Monitoring
```bash
# Profile a specific benchmark
cargo flamegraph --bench search_performance --features flamegraph

# Check for memory leaks
cargo test --features dhat-heap
```

### Code Quality
```bash
# This crate has strict performance requirements
cargo clippy -p blz-core -- -D warnings
cargo fmt --package blz-core -- --check

# Run the linter script
./scripts/lint.sh blz-core
```

Remember: This crate is performance-critical. Always benchmark changes and maintain sub-10ms search latencies.