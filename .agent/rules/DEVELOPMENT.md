# Development Practices

## Development Environment

### Required Tools

**Core Toolchain**

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup component add clippy rustfmt rust-src

# Additional tools
cargo install cargo-audit      # Security auditing
cargo install cargo-outdated  # Dependency updates
cargo install cargo-deny      # Dependency policies
cargo install cargo-watch     # File watching
cargo install cargo-tarpaulin # Code coverage
cargo install flamegraph      # Performance profiling
```

**Editor Setup**

- **rust-analyzer**: Language server for IDE support
- **CodeLLDB** or **GDB**: Debugging support
- **Rust syntax highlighting**: For your preferred editor

### Workspace Configuration

**Root Cargo.toml**

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/outfitter-dev/cache"
rust-version = "1.70.0"

[workspace.dependencies]
# Core dependencies shared across crates
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
tantivy = "0.21"

# Development dependencies
criterion = "0.5"
proptest = "1.0"
rstest = "0.18"
tempfile = "3.0"

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
rust_2018_idioms = "deny"

[workspace.lints.clippy]
all = "deny"
pedantic = "deny"
cargo = "deny"
# Allow some pedantic lints that are too strict
module_name_repetitions = "allow"
missing_errors_doc = "allow"
```

## Development Workflow

### Daily Development

**Start of Day Setup**

```bash
# Update dependencies and tools
rustup update
cargo update

# Check for security issues
cargo audit

# Run full test suite
cargo test --workspace

# Check code quality
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

**Feature Development Process**

1. **Branch Creation**

```bash
git checkout -b feature/improve-search-performance
```

2. **Write Failing Tests First**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_blz_search_results() {
        // Test that doesn't pass yet
        let cache = SearchCache::new();
        let query = "rust programming";

        // First search should hit the index
        let result1 = cache.search(query).unwrap();
        assert!(!result1.from_blz);

        // Second search should hit the cache
        let result2 = cache.search(query).unwrap();
        assert!(result2.from_blz);
    }
}
```

3. **Implement Minimal Solution**

```rust
pub struct SearchCache {
    cache: HashMap<String, SearchResults>,
    index: SearchIndex,
}

impl SearchCache {
    pub fn search(&mut self, query: &str) -> Result<SearchResults, CacheError> {
        if let Some(cached) = self.cache.get(query) {
            return Ok(cached.clone_with_blz_flag(true));
        }

        let results = self.index.search(query)?;
        self.cache.insert(query.to_string(), results.clone());
        Ok(results)
    }
}
```

4. **Iterative Improvement**

```rust
// Add proper error handling
// Add thread safety
// Add expiration
// Add metrics
// Add tests for edge cases
```

5. **Quality Gates**

```bash
# Before committing
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt
cargo doc --workspace --no-deps

# Optional: Run comprehensive tests
cargo test --workspace --all-features
cargo bench
```

### Code Review Process

**Self-Review Checklist**

- [ ] All tests pass locally
- [ ] Clippy passes with zero warnings
- [ ] Code is formatted with rustfmt
- [ ] Documentation is updated
- [ ] Error handling is comprehensive
- [ ] No `unwrap()` or `expect()` in production code
- [ ] Performance implications considered

**Review Guidelines**

- Focus on correctness and maintainability over cleverness
- Check error handling paths thoroughly
- Verify thread safety for concurrent code
- Ensure API design follows Rust conventions
- Look for potential memory leaks or resource issues

### Debugging Practices

**Logging Strategy**

```rust
use tracing::{debug, info, warn, error, instrument, span, Level};

#[instrument(skip(self), fields(query_len = query.len()))]
pub async fn search(&self, query: &str) -> CacheResult<SearchResults> {
    let span = span!(Level::INFO, "search_operation", query = %query);
    let _enter = span.enter();

    debug!("Starting search operation");

    match self.execute_search(query).await {
        Ok(results) => {
            info!(result_count = results.len(), "Search completed successfully");
            Ok(results)
        }
        Err(e) => {
            error!(error = %e, "Search operation failed");
            Err(e)
        }
    }
}
```

**Debug Builds**

```rust
// Use debug assertions in development
debug_assert!(query.len() > 0, "Query cannot be empty");
debug_assert!(limit <= MAX_RESULTS, "Limit exceeds maximum");

// Conditional compilation for debug information
#[cfg(debug_assertions)]
fn debug_query_info(query: &ParsedQuery) {
    println!("Query AST: {:#?}", query);
    println!("Estimated cost: {}", query.estimated_cost());
}
```

**Performance Debugging**

```bash
# Profile with perf
cargo build --release
perf record --call-graph=dwarf ./target/release/cache-cli search "rust"
perf report

# Generate flamegraphs
cargo install flamegraph
cargo flamegraph --bin cache-cli -- search "rust programming"

# Memory profiling with valgrind
cargo build --target x86_64-unknown-linux-gnu
valgrind --tool=massif --stacks=yes ./target/x86_64-unknown-linux-gnu/debug/cache-cli
```

## Code Organization

### Module Structure

**Library Crate Layout**

```
cache-core/src/
├── lib.rs              # Public API exports
├── config.rs           # Configuration management
├── error.rs            # Error types and conversions
├── index/              # Index management
│   ├── mod.rs
│   ├── builder.rs      # Index creation
│   ├── reader.rs       # Search operations
│   └── writer.rs       # Index updates
├── cache/              # Caching layer
│   ├── mod.rs
│   ├── memory.rs       # In-memory cache
│   └── disk.rs         # Persistent cache
├── query/              # Query processing
│   ├── mod.rs
│   ├── parser.rs       # Query parsing
│   └── validator.rs    # Query validation
└── utils/              # Shared utilities
    ├── mod.rs
    └── fs.rs           # File system helpers
```

**Import Organization**

```rust
// Standard library imports first
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// External crates
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tantivy::{Index, IndexReader};
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument};

// Internal imports - absolute paths from crate root
use crate::config::CacheConfig;
use crate::error::{CacheError, CacheResult};
use crate::index::SearchIndex;

// Local module imports last
use super::query::ParsedQuery;
```

### Documentation Standards

**Module Documentation**

```rust
//! Cache management module
//!
//! This module provides caching functionality for search results,
//! including both in-memory and persistent storage options.
//!
//! # Examples
//!
//! ```rust
//! use blz_core::cache::{CacheConfig, SearchCache};
//!
//! let config = CacheConfig::default();
//! let cache = SearchCache::new(config)?;
//! let results = cache.search("rust programming").await?;
//! ```

use std::collections::HashMap;
```

**Function Documentation**

```rust
/// Searches the index for documents matching the query
///
/// This function first checks the cache for existing results before
/// querying the underlying Tantivy index. Results are automatically
/// cached for future requests.
///
/// # Arguments
///
/// * `query` - The search query string to execute
/// * `limit` - Maximum number of results to return (1-1000)
///
/// # Returns
///
/// Returns a `Result` containing `SearchResults` on success, or a
/// `CacheError` if the query fails or is invalid.
///
/// # Examples
///
/// ```rust
/// let results = cache.search("rust programming", 10).await?;
/// println!("Found {} results", results.len());
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// * The query string is empty or invalid
/// * The limit is outside the valid range (1-1000)
/// * The underlying index is corrupted or inaccessible
/// * A timeout occurs during query execution
#[instrument(skip(self), fields(query_len = query.len(), limit = limit))]
pub async fn search(&self, query: &str, limit: u16) -> CacheResult<SearchResults> {
    // Implementation
}
```

### Error Handling Patterns

**Comprehensive Error Context**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Query parsing failed: '{query}' - {source}")]
    QueryParsing {
        query: String,
        #[source]
        source: tantivy::query::QueryParserError,
    },

    #[error("Index operation failed: {operation}")]
    IndexOperation {
        operation: String,
        #[source]
        source: tantivy::TantivyError,
    },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Resource limit exceeded: {resource} = {current}, max = {limit}")]
    ResourceLimit {
        resource: String,
        current: u64,
        limit: u64,
    },
}

// Usage with context
impl SearchIndex {
    pub fn search(&self, query: &str) -> CacheResult<SearchResults> {
        let parsed = self.query_parser.parse_query(query)
            .map_err(|e| CacheError::QueryParsing {
                query: query.to_string(),
                source: e,
            })?;

        // Execute search with context
        self.execute_query(parsed)
            .map_err(|e| CacheError::IndexOperation {
                operation: "search".to_string(),
                source: e,
            })
    }
}
```

## Quality Assurance

### Code Quality Gates

**Pre-commit Checks**

```bash
#!/bin/bash
# .git/hooks/pre-commit

echo "Running pre-commit checks..."

# Check formatting
echo "Checking formatting..."
cargo fmt --check
if [ $? -ne 0 ]; then
    echo "Code is not formatted. Run 'cargo fmt' to fix."
    exit 1
fi

# Run clippy
echo "Running Clippy..."
cargo clippy --workspace --all-targets -- -D warnings
if [ $? -ne 0 ]; then
    echo "Clippy found issues. Please fix them."
    exit 1
fi

# Run tests
echo "Running tests..."
cargo test --workspace
if [ $? -ne 0 ]; then
    echo "Tests failed."
    exit 1
fi

echo "Pre-commit checks passed!"
```

**Continuous Integration**

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy

    - name: Cache cargo
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

    - name: Check formatting
      run: cargo fmt --check

    - name: Lint with Clippy
      run: cargo clippy --workspace --all-targets -- -D warnings

    - name: Run tests
      run: cargo test --workspace --verbose

    - name: Run doc tests
      run: cargo test --workspace --doc

    - name: Check documentation
      run: cargo doc --workspace --no-deps --document-private-items

    - name: Security audit
      run: |
        cargo install --force cargo-audit
        cargo audit
```

### Performance Monitoring

**Benchmark Integration**

```rust
// benches/search_performance.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use blz_core::{SearchIndex, CacheConfig};

fn search_benchmarks(c: &mut Criterion) {
    let index = SearchIndex::new("test_index").unwrap();

    let queries = vec![
        "rust",
        "rust programming",
        "rust programming language tutorial",
        "title:rust AND body:programming",
    ];

    let mut group = c.benchmark_group("search");

    for query in queries {
        group.bench_with_input(
            BenchmarkId::new("cached", query),
            &query,
            |b, query| {
                b.iter(|| {
                    // First call to populate cache
                    let _ = index.search(black_box(query));
                    // Second call should hit cache
                    black_box(index.search(query))
                })
            }
        );

        group.bench_with_input(
            BenchmarkId::new("uncached", query),
            &query,
            |b, query| {
                b.iter(|| {
                    index.clear_blz();
                    black_box(index.search(query))
                })
            }
        );
    }

    group.finish();
}

criterion_group!(benches, search_benchmarks);
criterion_main!(benches);
```

## Development Anti-Patterns

### Common Mistakes

**Avoid These Patterns**

```rust
// ❌ Using unwrap in production code
let results = search_index.search(query).unwrap();

// ✅ Proper error handling
let results = search_index.search(query)
    .map_err(|e| CacheError::SearchFailed {
        query: query.to_string(),
        source: e
    })?;

// ❌ Blocking in async code
async fn search(&self, query: &str) -> Result<SearchResults, Error> {
    let file_content = std::fs::read_to_string("config.toml"); // Blocks!
    // ...
}

// ✅ Async file operations
async fn search(&self, query: &str) -> Result<SearchResults, Error> {
    let file_content = tokio::fs::read_to_string("config.toml").await?;
    // ...
}

// ❌ Clone everywhere to satisfy borrow checker
let results1 = expensive_data.clone();
let results2 = expensive_data.clone();
process_data(results1, results2);

// ✅ Understand borrowing
let results1 = &expensive_data;
let results2 = &expensive_data;
process_data(results1, results2);
```

### Performance Anti-Patterns

**Avoid These Patterns**

```rust
// ❌ Unnecessary allocations in hot paths
pub fn format_query(query: &str) -> String {
    let mut result = String::new();
    for word in query.split_whitespace() {
        result += &format!("{}:", word); // Allocates each time
    }
    result
}

// ✅ Efficient string building
pub fn format_query(query: &str) -> String {
    let mut result = String::with_capacity(query.len() * 2);
    for word in query.split_whitespace() {
        result.push_str(word);
        result.push(':');
    }
    result
}

// ❌ Synchronous I/O in async context
async fn load_config(&self) -> Result<Config, Error> {
    let content = std::fs::read_to_string("config.toml")?; // Blocks executor
    Ok(toml::from_str(&content)?)
}

// ✅ Async I/O
async fn load_config(&self) -> Result<Config, Error> {
    let content = tokio::fs::read_to_string("config.toml").await?;
    Ok(toml::from_str(&content)?)
}
```

## Maintenance Practices

### Dependency Management

**Regular Updates**

```bash
# Check for outdated dependencies
cargo outdated

# Update dependencies
cargo update

# Check for security vulnerabilities
cargo audit

# Check licenses and policies
cargo deny check
```

**Version Pinning Strategy**

```toml
[dependencies]
# Pin exact versions for security-critical deps
ring = "=0.16.20"

# Use compatible versions for stable deps
serde = "1.0"

# Use minimum versions for unstable deps
tantivy = ">=0.21.0, <0.22.0"
```

### Technical Debt Management

**Code Health Metrics**

- Monitor cyclomatic complexity with `cargo clippy`
- Track test coverage with `cargo tarpaulin`
- Measure documentation coverage with `cargo doc`
- Profile performance with `cargo bench`

**Refactoring Guidelines**

- Refactor when adding third use case (rule of three)
- Extract functions when complexity > 10
- Split modules when file > 500 lines
- Review architecture when crate > 10,000 lines

Remember: Development practices should enable fast, safe iteration while maintaining high code quality. Every practice should have a clear purpose and measurable benefit.
