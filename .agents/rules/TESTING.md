# Testing Strategy

## Testing Philosophy

Testing in Rust should leverage the type system to catch bugs at compile time, then use comprehensive runtime tests to verify behavior, performance, and edge cases.

## Test Categories

### 1. Unit Tests

#### Command/CLI seams

We are moving CLI commands toward a "thin shell + testable core" pattern. When touching CLI code:

- Extract work into a pure function (`execute_*` or `*_core`) that accepts injected traits for storage/indexing/network prompts. See:
  - `crates/blz-cli/src/commands/clear.rs` (`ClearStorage`, `execute_clear`).
  - `crates/blz-cli/src/commands/list.rs` (`ListStorage`, `render_list`).
  - `crates/blz-cli/src/commands/remove.rs` (`RemoveStorage`, `execute_remove`).
  - `crates/blz-cli/src/commands/update.rs` (`UpdateStorage`, `apply_update`).
- Keep the public `execute` wrapper responsible only for wiring real dependencies (stdout, stdin confirmations, progress bars).
- Mock storage/indexers in unit tests to avoid hitting the filesystem or network. Prefer light in-crate structs (no external mocking framework yet).
- Verify both behavior and side effects by inspecting the mock (e.g., which aliases were saved/indexed).

This keeps unit tests fast, deterministic, and unlocks seamless coverage without complex integration scaffolding.

**Co-located with Source Code**

```rust
// src/query/parser.rs
pub fn parse_query(input: &str) -> Result<ParsedQuery, ParseError> {
    // Implementation
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case("simple", Ok(ParsedQuery::Term("simple".to_string())))]
    #[case("title:rust", Ok(ParsedQuery::Field { field: "title", term: "rust" }))]
    #[case("", Err(ParseError::EmptyQuery))]
    #[case("title:", Err(ParseError::MissingTerm))]
    fn parse_query_cases(#[case] input: &str, #[case] expected: Result<ParsedQuery, ParseError>) {
        assert_eq!(parse_query(input), expected);
    }

    #[test]
    fn parse_complex_boolean_query() {
        let input = "(title:rust OR body:programming) AND NOT deprecated:true";
        let result = parse_query(input).unwrap();

        match result {
            ParsedQuery::Boolean { op: BoolOp::And, left, right } => {
                // Verify structure
                assert!(matches!(**left, ParsedQuery::Boolean { op: BoolOp::Or, .. }));
                assert!(matches!(**right, ParsedQuery::Not(_)));
            }
            _ => panic!("Expected boolean query"),
        }
    }
}
```

**Testing Traits and Generics**

```rust
pub trait CacheStorage: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError>;
    async fn set(&self, key: &str, value: Vec<u8>) -> Result<(), StorageError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    // Mock implementation for testing
    #[derive(Default)]
    struct MockStorage {
        data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    }

    #[async_trait::async_trait]
    impl CacheStorage for MockStorage {
        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
            let data = self.data.lock().await;
            Ok(data.get(key).cloned())
        }

        async fn set(&self, key: &str, value: Vec<u8>) -> Result<(), StorageError> {
            let mut data = self.data.lock().await;
            data.insert(key.to_string(), value);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_blz_storage_contract() {
        let storage = MockStorage::default();

        // Test empty cache
        assert_eq!(storage.get("key1").await.unwrap(), None);

        // Test set and get
        let value = b"test_value".to_vec();
        storage.set("key1", value.clone()).await.unwrap();
        assert_eq!(storage.get("key1").await.unwrap(), Some(value));

        // Test overwrite
        let new_value = b"new_value".to_vec();
        storage.set("key1", new_value.clone()).await.unwrap();
        assert_eq!(storage.get("key1").await.unwrap(), Some(new_value));
    }
}
```

### 2. Integration Tests

**Separate Test Directory**

```rust
// tests/integration/search_pipeline.rs
use blz_core::{SearchIndex, CacheConfig, SearchCache};
use tempfile::TempDir;
use tokio_test;

struct TestContext {
    temp_dir: TempDir,
    config: CacheConfig,
    index: SearchIndex,
    cache: SearchCache,
}

impl TestContext {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let mut config = CacheConfig::default();
        config.index_path = temp_dir.path().join("index");

        let index = SearchIndex::create(&config.index_path)?;
        let cache = SearchCache::new(config.clone())?;

        Ok(Self {
            temp_dir,
            config,
            index,
            cache,
        })
    }

    async fn populate_test_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let documents = vec![
            TestDocument {
                title: "Rust Programming Language".to_string(),
                body: "Rust is a systems programming language focused on safety and performance.".to_string(),
                tags: vec!["programming".to_string(), "systems".to_string()],
            },
            TestDocument {
                title: "Advanced Rust Patterns".to_string(),
                body: "Learn advanced patterns in Rust including async programming and macros.".to_string(),
                tags: vec!["programming".to_string(), "advanced".to_string()],
            },
        ];

        for doc in documents {
            self.index.add_document(doc).await?;
        }

        self.index.commit().await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_full_search_pipeline() {
    let mut ctx = TestContext::new().await.unwrap();
    ctx.populate_test_data().await.unwrap();

    // Test basic search
    let results = ctx.cache.search("rust").await.unwrap();
    assert_eq!(results.hits.len(), 2);
    assert!(!results.from_blz);

    // Test cache hit
    let cached_results = ctx.cache.search("rust").await.unwrap();
    assert_eq!(cached_results.hits.len(), 2);
    assert!(cached_results.from_blz);
    assert!(cached_results.execution_time < results.execution_time);

    // Test field-specific search
    let title_results = ctx.cache.search("title:rust").await.unwrap();
    assert_eq!(title_results.hits.len(), 2);

    // Test Boolean search
    let boolean_results = ctx.cache.search("rust AND advanced").await.unwrap();
    assert_eq!(boolean_results.hits.len(), 1);
    assert!(boolean_results.hits[0].title.contains("Advanced"));
}

#[tokio::test]
async fn test_concurrent_searches() {
    let mut ctx = TestContext::new().await.unwrap();
    ctx.populate_test_data().await.unwrap();

    let cache = Arc::new(ctx.cache);
    let mut handles = Vec::new();

    // Launch 10 concurrent searches
    for i in 0..10 {
        let cache_clone = Arc::clone(&cache);
        let query = if i % 2 == 0 { "rust" } else { "programming" };

        handles.push(tokio::spawn(async move {
            cache_clone.search(query).await
        }));
    }

    // Wait for all searches to complete
    let results: Vec<_> = futures::future::try_join_all(handles)
        .await
        .unwrap()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    // Verify all searches succeeded
    assert_eq!(results.len(), 10);
    for result in results {
        assert!(!result.hits.is_empty());
    }
}
```

### 3. Property-Based Tests

**Using Proptest**

```rust
use proptest::prelude::*;

// Generate arbitrary queries for testing
#[derive(Debug, Clone)]
struct ArbitraryQuery {
    terms: Vec<String>,
    field_queries: Vec<(String, String)>,
    boolean_ops: Vec<BooleanOp>,
}

impl Arbitrary for ArbitraryQuery {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        (
            prop::collection::vec("[a-z]+", 1..5),
            prop::collection::vec(("[a-z]+", "[a-z]+"), 0..3),
            prop::collection::vec(prop_oneof![
                Just(BooleanOp::And),
                Just(BooleanOp::Or),
                Just(BooleanOp::Not)
            ], 0..2),
        )
        .prop_map(|(terms, field_queries, boolean_ops)| ArbitraryQuery {
            terms,
            field_queries,
            boolean_ops,
        })
        .boxed()
    }
}

proptest! {
    #[test]
    fn query_parsing_roundtrip(query in any::<ArbitraryQuery>()) {
        let query_string = query.to_string();

        // If parsing succeeds, serializing and parsing again should be identical
        if let Ok(parsed) = parse_query(&query_string) {
            let serialized = parsed.to_string();
            let reparsed = parse_query(&serialized).unwrap();
            prop_assert_eq!(parsed, reparsed);
        }
    }

    #[test]
    fn search_results_consistent(
        query in "[a-zA-Z ]+",
        limit in 1u16..100u16
    ) {
        prop_assume!(!query.trim().is_empty());

        let index = create_test_index();

        // Same query should return same results
        let results1 = index.search(&query, limit).unwrap();
        let results2 = index.search(&query, limit).unwrap();

        prop_assert_eq!(results1.hits, results2.hits);
        prop_assert_eq!(results1.total_count, results2.total_count);
    }

    #[test]
    fn cache_preserves_semantics(
        query in "[a-zA-Z ]+",
        limit in 1u16..100u16
    ) {
        prop_assume!(!query.trim().is_empty());

        let mut cache = create_test_blz();

        // First search (miss)
        let uncached = cache.search(&query, limit).unwrap();
        prop_assert!(!uncached.from_blz);

        // Second search (hit)
        let cached = cache.search(&query, limit).unwrap();
        prop_assert!(cached.from_blz);

        // Results should be identical except for cache flag
        prop_assert_eq!(uncached.hits, cached.hits);
        prop_assert_eq!(uncached.total_count, cached.total_count);
    }
}
```

### 4. Benchmark Tests

**Performance Regression Detection**

```rust
// benches/search_benchmarks.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use blz_core::*;

fn search_performance_benchmarks(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let index = rt.block_on(async {
        let mut index = create_large_test_index().await.unwrap();
        populate_with_documents(&mut index, 10_000).await.unwrap();
        index
    });

    let mut group = c.benchmark_group("search_performance");

    // Benchmark different query types
    let query_types = vec![
        ("simple_term", "rust"),
        ("field_query", "title:programming"),
        ("boolean_and", "rust AND programming"),
        ("boolean_or", "rust OR python"),
        ("phrase_query", "\"rust programming language\""),
        ("wildcard", "program*"),
    ];

    for (name, query) in query_types {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(
            BenchmarkId::new("uncached", name),
            &query,
            |b, &query| {
                b.to_async(&rt).iter(|| async {
                    index.clear_blz().await;
                    black_box(index.search(query, 10).await.unwrap())
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("cached", name),
            &query,
            |b, &query| {
                // Warm up cache
                rt.block_on(async { index.search(query, 10).await.unwrap() });

                b.to_async(&rt).iter(|| async {
                    black_box(index.search(query, 10).await.unwrap())
                })
            },
        );
    }

    group.finish();
}

fn indexing_performance_benchmarks(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("indexing_performance");

    // Benchmark document sizes
    let doc_sizes = vec![100, 1_000, 10_000, 100_000];

    for size in doc_sizes {
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::new("add_document", size),
            &size,
            |b, &size| {
                b.to_async(&rt).iter_with_setup(
                    || {
                        let mut index = rt.block_on(create_test_index()).unwrap();
                        let doc = generate_document_of_size(size);
                        (index, doc)
                    },
                    |(mut index, doc)| async move {
                        black_box(index.add_document(doc).await.unwrap())
                    }
                )
            },
        );
    }

    group.finish();
}

criterion_group!(benches, search_performance_benchmarks, indexing_performance_benchmarks);
criterion_main!(benches);
```

### 5. Stress Tests

**Resource Limits and Edge Cases**

```rust
// tests/stress/resource_limits.rs
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_memory_usage_under_load() {
    let cache = create_test_blz_with_limits();
    let memory_tracker = MemoryTracker::new();

    // Generate load
    let tasks = (0..1000)
        .map(|i| {
            let cache = cache.clone();
            let query = format!("test query {}", i % 100); // Some overlap for caching

            tokio::spawn(async move {
                cache.search(&query, 10).await
            })
        })
        .collect::<Vec<_>>();

    let results = futures::future::join_all(tasks).await;

    // Verify all searches completed successfully
    for result in results {
        assert!(result.unwrap().is_ok());
    }

    // Verify memory usage stayed within bounds
    let peak_memory = memory_tracker.peak_usage();
    assert!(peak_memory < 500_000_000, "Peak memory usage: {} bytes", peak_memory);
}

#[tokio::test]
async fn test_query_timeout_handling() {
    let index = create_slow_test_index(); // Intentionally slow for timeout testing

    let query = "complex AND expensive AND query";
    let result = timeout(
        Duration::from_millis(100),
        index.search(query, 1000)
    ).await;

    match result {
        Ok(search_result) => {
            // Query completed within timeout - that's fine
            assert!(search_result.is_ok());
        }
        Err(_) => {
            // Timeout occurred - verify system is still responsive
            let simple_result = index.search("simple", 1).await;
            assert!(simple_result.is_ok(), "Index should remain responsive after timeout");
        }
    }
}

#[tokio::test]
async fn test_concurrent_index_updates() {
    let index = Arc::new(create_test_index().await.unwrap());
    let counter = Arc::new(AtomicUsize::new(0));

    // Concurrent readers
    let readers = (0..10).map(|_| {
        let index = Arc::clone(&index);
        tokio::spawn(async move {
            for _ in 0..100 {
                let _ = index.search("test", 10).await;
            }
        })
    });

    // Concurrent writers
    let writers = (0..5).map(|_| {
        let index = Arc::clone(&index);
        let counter = Arc::clone(&counter);
        tokio::spawn(async move {
            for _ in 0..20 {
                let doc_id = counter.fetch_add(1, Ordering::SeqCst);
                let doc = create_test_document(doc_id);
                let _ = index.add_document(doc).await;
            }
        })
    });

    // Wait for all operations to complete
    let _ = futures::future::try_join_all(readers).await;
    let _ = futures::future::try_join_all(writers).await;

    // Verify index is in consistent state
    index.commit().await.unwrap();
    let final_count = index.document_count().await.unwrap();
    assert_eq!(final_count, 100); // 5 writers × 20 docs each
}
```

## Test Organization

### Test Structure

**Directory Layout**

```
cache/
├── src/                    # Source code with inline unit tests
│   └── lib.rs             # #[cfg(test)] mod tests { ... }
├── tests/                 # Integration tests
│   ├── integration/       # Full system tests
│   │   ├── search.rs
│   │   └── cache.rs
│   ├── stress/           # Load and stress tests
│   │   ├── memory.rs
│   │   └── concurrency.rs
│   └── common/           # Shared test utilities
│       ├── mod.rs
│       └── fixtures.rs
├── benches/              # Benchmark tests
│   ├── search.rs
│   └── indexing.rs
└── examples/             # Example code that serves as tests
    └── basic_usage.rs
```

### Test Utilities

**Common Test Infrastructure**

```rust
// tests/common/mod.rs
use std::sync::Once;
use tracing_subscriber;

static INIT: Once = Once::new();

pub fn init_test_logging() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("blz_core=debug")
            .with_test_writer()
            .init();
    });
}

pub struct TestIndex {
    pub index: SearchIndex,
    pub temp_dir: TempDir,
}

impl TestIndex {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let index = SearchIndex::create(temp_dir.path()).await?;

        Ok(Self { index, temp_dir })
    }

    pub async fn with_documents(documents: Vec<TestDocument>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut test_index = Self::new().await?;

        for doc in documents {
            test_index.index.add_document(doc).await?;
        }

        test_index.index.commit().await?;
        Ok(test_index)
    }
}

// Builders for test data
pub struct TestDocumentBuilder {
    title: String,
    body: String,
    tags: Vec<String>,
    url: Option<String>,
}

impl TestDocumentBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: String::new(),
            tags: Vec::new(),
            url: None,
        }
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn build(self) -> TestDocument {
        TestDocument {
            title: self.title,
            body: self.body,
            tags: self.tags,
            url: self.url,
        }
    }
}

// Usage in tests:
// let doc = TestDocumentBuilder::new("Rust Guide")
//     .body("Learn Rust programming")
//     .tags(["rust", "programming"])
//     .build();
```

## Test Quality Standards

### Coverage Requirements

**Minimum Coverage Targets**

- **Unit Tests**: 90% line coverage, 85% branch coverage
- **Integration Tests**: 80% of public API paths
- **Critical Paths**: 100% coverage (error handling, security, data integrity)

**Measuring Coverage**

```bash
# Install coverage tool
cargo install cargo-tarpaulin

# Run with coverage
cargo tarpaulin --workspace --timeout 120 --out Html --output-dir coverage

# View results
open coverage/tarpaulin-report.html
```

### Test Performance Standards

**Speed Requirements**

- **Unit tests**: Complete in <1ms each, <5s total
- **Integration tests**: Complete in <100ms each, <30s total
- **Property tests**: Complete in <10s each
- **Benchmarks**: Stable results with <5% variance

**Reliability Requirements**

- **Zero flaky tests**: All tests must be deterministic
- **Parallel execution**: Tests must not interfere with each other
- **Resource cleanup**: No test should leak resources
- **Isolation**: Each test should start with clean state

### Test Documentation

**Test Descriptions**

```rust
#[test]
fn should_blz_repeated_queries() {
    // Given: A search cache with a populated index
    let mut cache = create_test_blz();
    let query = "rust programming";

    // When: The same query is executed twice
    let first_result = cache.search(query, 10).unwrap();
    let second_result = cache.search(query, 10).unwrap();

    // Then: The second result should come from cache
    assert!(!first_result.from_blz, "First search should miss cache");
    assert!(second_result.from_blz, "Second search should hit cache");
    assert_eq!(first_result.hits, second_result.hits, "Results should be identical");
    assert!(second_result.execution_time < first_result.execution_time,
             "Cached result should be faster");
}
```

## Test Anti-Patterns

### Common Testing Mistakes

**Avoid These Patterns**

```rust
// ❌ Tests that depend on external state
#[test]
fn test_search_with_real_api() {
    let client = ApiClient::new("https://api.example.com");
    let results = client.search("rust").unwrap(); // Flaky!
    assert!(!results.is_empty());
}

// ✅ Use mocks and fixtures
#[test]
fn test_search_with_mock_api() {
    let mock_client = MockApiClient::new();
    mock_client.expect_search("rust")
        .returning(|_| Ok(vec![create_test_result()]));

    let results = mock_client.search("rust").unwrap();
    assert_eq!(results.len(), 1);
}

// ❌ Tests with hardcoded timing
#[tokio::test]
async fn test_async_operation() {
    let task = start_background_task();
    tokio::time::sleep(Duration::from_millis(100)).await; // Flaky!
    assert!(task.is_complete());
}

// ✅ Use proper synchronization
#[tokio::test]
async fn test_async_operation() {
    let task = start_background_task();
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        task.wait_for_completion()
    ).await;
    assert!(result.is_ok());
}

// ❌ Overly complex test setup
#[test]
fn test_complex_scenario() {
    // 50 lines of setup code
    let complex_state = build_extremely_complex_test_state();
    // Test becomes hard to understand
}

// ✅ Break into focused tests with helper functions
#[test]
fn test_search_with_filters() {
    let index = create_index_with_filtered_documents();
    let results = index.search_with_filters("rust", &["programming"]).unwrap();
    assert_eq!(results.len(), 2);
}
```

Remember: Good tests are fast, reliable, and clearly express the expected behavior. They should serve as living documentation of how the system works and catch regressions before they reach production.
