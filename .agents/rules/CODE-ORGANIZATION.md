# Code Organization

## Module Structure

### Library Crate Layout

```
blz-core/src/
├── lib.rs              # Public API exports
├── config.rs           # Configuration management
├── error.rs            # Error types and conversions
├── index.rs            # Search index management
├── fetcher.rs          # HTTP client with ETag support
├── parser.rs           # Tree-sitter markdown parser
├── storage.rs          # Local filesystem storage
├── registry.rs         # Source management
├── profiling.rs        # Performance profiling
└── types.rs            # Shared type definitions
```

### Binary Crate Structure

```
blz-cli/src/
├── main.rs             # CLI entry point and command dispatch
└── build.rs            # Build script for completions

blz-mcp/src/
└── main.rs             # MCP server implementation
```

## Import Organization

Follow this order for imports:

```rust
// 1. Standard library imports
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// 2. External crates (alphabetical)
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tantivy::{Index, IndexReader};
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument};

// 3. Internal crates (from workspace)
use blz_core::config::Config;
use blz_core::error::{Error, Result};

// 4. Current crate imports (absolute from crate root)
use crate::config::CacheConfig;
use crate::error::{CacheError, CacheResult};
use crate::index::SearchIndex;

// 5. Parent module imports
use super::query::ParsedQuery;

// 6. Sub-module imports
use self::internal::Helper;
```

## Module Guidelines

### When to Create a New Module

- When a file exceeds 500 lines
- When functionality is logically distinct
- When code will be reused across multiple modules
- When testing in isolation is beneficial

### Module Naming

- Use lowercase with underscores
- Be descriptive but concise
- Avoid generic names like `utils` or `helpers` (be specific)

### Module Documentation

Every module must have documentation:

```rust
//! Module-level documentation
//!
//! This module provides functionality for...
//!
//! # Examples
//!
//! ```rust
//! use blz_core::module_name;
//! // example usage
//! ```
```

## File Organization

### Maximum File Sizes

- **500 lines**: Consider splitting
- **750 lines**: Should split
- **1000 lines**: Must split

### Test Organization

Tests go in the same file for small modules:

```rust
// src/small_module.rs

pub fn functionality() { }

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_functionality() { }
}
```

For larger modules, use separate test files:

```
src/
├── large_module.rs
└── large_module/
    └── tests.rs
```

## Visibility Rules

### API Design

- Start with private visibility
- Only expose what's necessary
- Use `pub(crate)` for internal APIs
- Document all public APIs

```rust
// Private by default
struct InternalHelper;

// Crate-visible for internal use
pub(crate) struct CrateHelper;

// Public API - must be documented
/// Public structure for...
pub struct PublicApi {
    // Private fields by default
    internal: String,
    
    // Selectively expose fields
    pub name: String,
}
```

## Type Organization

### Type Aliases

Use type aliases for clarity:

```rust
type Result<T> = std::result::Result<T, Error>;
type LineNumber = u32;
type Score = f32;
```

### Newtype Pattern

Use newtypes for type safety:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct QueryId(String);

#[derive(Debug, Clone, Copy)]
pub struct LineRange {
    pub start: LineNumber,
    pub end: LineNumber,
}
```

## Error Handling

### Error Types

Define errors in `error.rs`:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Query parsing failed: {0}")]
    QueryParse(String),
    
    #[error("Index error: {0}")]
    Index(#[from] tantivy::TantivyError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

### Error Context

Add context to errors:

```rust
use anyhow::{Context, Result};

pub fn load_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read config file")?;
    
    toml::from_str(&content)
        .context("Failed to parse config file")
}
```

## Documentation Standards

### Function Documentation

```rust
/// Searches the index for documents matching the query
///
/// # Arguments
///
/// * `query` - The search query string
/// * `limit` - Maximum number of results
///
/// # Returns
///
/// Returns search results ordered by relevance
///
/// # Errors
///
/// Returns an error if:
/// * The query is invalid
/// * The index is corrupted
///
/// # Examples
///
/// ```rust
/// let results = index.search("rust", 10)?;
/// ```
pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
    // implementation
}
```

### Struct Documentation

```rust
/// Represents a search result from the index
///
/// Contains the document content and metadata needed
/// to display search results to users.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The source alias (e.g., "bun", "node")
    pub alias: String,
    
    /// Relevance score (higher is better)
    pub score: f32,
    
    /// Line range in the source document
    pub lines: LineRange,
    
    /// Snippet of matching content
    pub snippet: String,
}
```

## Common Patterns

### Builder Pattern

For complex configuration:

```rust
pub struct IndexBuilder {
    path: PathBuf,
    max_results: usize,
    // ... other fields
}

impl IndexBuilder {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            max_results: 100,
            // ... defaults
        }
    }
    
    pub fn max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }
    
    pub fn build(self) -> Result<Index> {
        // construct index
    }
}
```

### Resource Management

Use RAII patterns:

```rust
pub struct TempIndex {
    path: PathBuf,
    _temp_dir: TempDir, // Dropped when TempIndex drops
}

impl TempIndex {
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().to_path_buf();
        
        Ok(Self {
            path,
            _temp_dir: temp_dir,
        })
    }
}
```
