# blz-core Development Guide for Agents

## Context
This is the core library crate containing search, parsing, and indexing logic.
**Performance is critical** - this code targets <10ms search latency.

## Key Patterns Used Here

- @./.agents/rules/conventions/rust/async-patterns.md - Comprehensive async/await patterns
- @./.agents/rules/conventions/rust/unsafe-policy.md - Unsafe code policy and review requirements

### Async Task Spawning Template
```rust
// ✅ Correct: Move owned data into spawned task
use tokio::spawn;
use std::sync::Arc;

async fn process_documents(docs: Vec<Document>, index: SearchIndex) -> Result<()> {
    let shared_index = Arc::new(index);
    let mut handles = Vec::new();
    
    for doc in docs {  // doc is moved into each iteration
        let index_clone = Arc::clone(&shared_index);
        let handle = spawn(async move {
            // Both doc and index_clone are moved into the task
            index_clone.add_document(doc).await
        });
        handles.push(handle);
    }
    
    // Wait for all indexing to complete
    for handle in handles {
        handle.await??;  // First ? for JoinError, second for our Result
    }
    
    Ok(())
}

// ❌ Wrong: Borrowing across await
async fn bad_example(docs: &[Document]) {
    let first_doc = &docs[0];  // Borrowed reference
    some_async_operation().await;  // Borrow checker can't prove first_doc is still valid
    process_document(first_doc).await;  // Error: borrow might not be valid
}

// ✅ Fix: Clone before await or take ownership
async fn good_example(docs: Vec<Document>) {
    let first_doc = docs[0].clone();  // Owned copy
    some_async_operation().await;
    process_document(first_doc).await;  // Works: first_doc is owned
}
```

### Error Handling Patterns
```rust
// For internal library functions - use thiserror for structured errors
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid syntax at line {line}: {message}")]
    SyntaxError { line: usize, message: String },
    
    #[error("Unsupported markdown feature: {feature}")]
    UnsupportedFeature { feature: String },
    
    #[error("Tree-sitter parsing failed: {0}")]
    TreeSitter(#[from] tree_sitter::TreeSitterError),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// For complex error contexts - use anyhow
use anyhow::{Context, Result, bail};

pub fn parse_llms_file(path: &Path) -> Result<ParsedDocument> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read llms.txt file")?;
    
    if content.is_empty() {
        bail!("llms.txt file is empty");
    }
    
    parse_markdown_content(&content)
        .context("Failed to parse markdown content")
        .context(format!("Error processing file: {}", path.display()))
}
```

### Memory Management Patterns
```rust
// Use Arc for expensive-to-clone shared data
use std::sync::Arc;

pub struct SearchCache {
    index: Arc<SearchIndex>,     // Shared between tasks
    config: Arc<CacheConfig>,    // Shared configuration
}

impl SearchCache {
    pub async fn search_concurrent(&self, queries: Vec<String>) -> Vec<SearchResult> {
        let mut handles = Vec::new();
        
        for query in queries {
            let index = Arc::clone(&self.index);    // Cheap Arc clone
            let config = Arc::clone(&self.config);
            
            handles.push(tokio::spawn(async move {
                index.search(&query, &config).await
            }));
        }
        
        // Collect results...
        let mut results = Vec::new();
        for handle in handles {
            // JoinError -> anyhow::Error, then inner Result  
            results.push(handle.await??);
        }
        results
    }
}

// For owned data that needs to cross async boundaries
pub async fn process_search_results(results: Vec<SearchHit>) -> ProcessedResults {
    // results is moved into async function, safe to use across awaits
    let processed = results.into_iter()
        .map(|hit| process_hit(hit))  // hit moved into closure
        .collect::<Vec<_>>();
    
    save_results(&processed).await;  // processed can be borrowed here
    
    ProcessedResults { hits: processed }
}
```

### Performance-Critical Code Patterns
```rust
// Zero-allocation string processing where possible
pub fn extract_title(content: &str) -> Option<&str> {
    // Return string slice (no allocation) instead of String
    content.lines()
        .find(|line| line.starts_with("# "))
        .map(|line| &line[2..])  // Strip "# " prefix
}

// Use Cow for flexible owned/borrowed data
use std::borrow::Cow;

pub fn normalize_query(query: &str) -> Cow<str> {
    if query.chars().any(|c| c.is_uppercase()) {
        // Need to modify - return owned String
        Cow::Owned(query.to_lowercase())
    } else {
        // No changes needed - return borrowed &str
        Cow::Borrowed(query)
    }
}

// Pool resources for high-frequency operations
use crate::memory_pool::MemoryPool;

pub struct Parser {
    buffer_pool: MemoryPool,
}

impl Parser {
    pub async fn parse_document(&self, content: &str) -> Result<Document> {
        // Get pooled buffer instead of allocating new one
        let mut buffer = self.buffer_pool.get_buffer(content.len() * 2).await;
        
        // Use buffer for parsing work
        parse_into_buffer(content, buffer.as_mut())?;
        
        // buffer automatically returned to pool when dropped
        Ok(Document { /* ... */ })
    }
}
```

### Unsafe Code Policy
- Only use `unsafe` for performance-critical code where safe alternatives are measurably insufficient
- Every unsafe block MUST have a `// SAFETY:` comment explaining invariants
- Prefer well-tested crates over custom unsafe code when possible
- All unsafe code must pass Miri testing

### Current Unsafe Blocks in blz-core
```rust
// memory_pool.rs: Arena allocator for zero-allocation parsing
// SAFETY: All pointer arithmetic is bounds-checked and lifetimes are managed carefully
unsafe fn alloc_raw(&mut self, layout: std::alloc::Layout) -> *mut u8 {
    // Detailed safety reasoning here...
}

// cache.rs: Hand-optimized LRU cache for search results  
// SAFETY: NonNull pointers are never null and linked list invariants are maintained
unsafe fn move_to_front(&mut self, node_ptr: NonNull<Node<K, V>>) {
    // Detailed safety reasoning here...
}
```

## Performance Requirements
- **Search operations**: P50 < 10ms, P99 < 50ms
- **Parse operations**: < 150ms per MB of markdown
- **Memory usage**: < 2x source document size during parsing
- **Index operations**: < 1ms for simple additions

## Testing Patterns
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_concurrent_search() {
        let index = create_test_index().await;
        let queries = vec!["rust", "programming", "async"];
        
        let results = search_concurrent(queries, &index).await;
        
        assert_eq!(results.len(), 3);
        for result in results {
            assert!(!result.hits.is_empty());
        }
    }
    
    // Property-based testing for parser
    #[cfg(feature = "proptest")]
    proptest! {
        #[test]
        fn parse_never_panics(content in "\\PC*") {
            // Parser should handle any valid UTF-8 input without panicking
            let result = parse_markdown_content(&content);
            // Should return Ok or structured error, never panic
        }
    }
}
```

## Async Best Practices for this Crate
1. **Spawn tasks with owned data** - avoid borrowing across `.await`
2. **Use Arc for shared expensive resources** - indices, configurations
3. **Prefer stream processing** - don't collect all results in memory
4. **Implement timeouts** - prevent hanging operations
5. **Use structured concurrency** - join all tasks before returning

## Common Pitfalls to Avoid
- **Don't hold mutex guards across `.await`** - use scope blocks
- **Don't clone large data unnecessarily** - use Arc for sharing
- **Don't ignore Send/Sync bounds** - they prevent data races
- **Don't use blocking I/O in async functions** - use tokio::fs instead

## Integration with Other Crates
- **blz-cli**: Provides high-level user-facing APIs using anyhow errors
- **blz-mcp**: Uses structured JSON responses, needs Serialize and Deserialize
- **External**: tantivy for search, tree-sitter for parsing

## Debugging Tips

- @./.agents/rules/conventions/rust/compiler-loop.md - Compiler-in-the-loop diagnostics

1. Use `tracing::instrument` on public functions for observability
2. Add debug assertions for invariant checking: `debug_assert!(condition)`
3. Use `cargo expand` to debug derive macro issues
4. Profile with `pprof` feature flag for performance issues