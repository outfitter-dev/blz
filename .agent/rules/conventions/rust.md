# Rust Language Conventions

## Code Style and Formatting

### Rustfmt Configuration

**Project .rustfmt.toml**
```toml
# Stable rustfmt options
edition = "2021"
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"

# Imports
imports_layout = "Vertical"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
reorder_imports = true

# Functions and control flow
fn_single_line = false
control_brace_style = "AlwaysNextLine"
brace_style = "SameLineWhere"

# Comments and strings
normalize_comments = true
wrap_comments = true
comment_width = 80
format_strings = true

# Macro formatting
format_macro_matchers = true
format_macro_bodies = true

# Advanced formatting
use_field_init_shorthand = true
use_try_shorthand = true
force_explicit_abi = true

# Trailing commas and semicolons
trailing_comma = "Vertical"
trailing_semicolon = true
```

### Naming Conventions

**Consistent Naming Patterns**
```rust
// Crate names: lowercase with dashes
// cache-core, cache-cli, cache-mcp

// Module names: snake_case
mod search_engine;
mod query_parser;
mod result_formatter;

// Type names: PascalCase
pub struct SearchIndex;
pub struct QueryValidator; 
pub struct DocumentProcessor;

// Trait names: PascalCase, often adjectives
pub trait Searchable;
pub trait Cacheable;
pub trait Serializable;

// Function names: snake_case, verb phrases
pub fn create_index() -> SearchIndex;
pub fn validate_query(query: &str) -> Result<ValidatedQuery, QueryError>;
pub fn process_document(doc: &str) -> ProcessedDocument;

// Constant names: SCREAMING_SNAKE_CASE
const MAX_QUERY_LENGTH: usize = 1000;
const DEFAULT_CACHE_SIZE: usize = 100;
const INDEX_VERSION: u32 = 1;

// Static names: SCREAMING_SNAKE_CASE
static GLOBAL_CONFIG: OnceCell<Config> = OnceCell::new();

// Variable names: snake_case
let search_results = index.search(&query)?;
let parsed_query = parser.parse(raw_query)?;
let document_count = index.document_count();

// Generic parameters: Single uppercase letter, descriptive
pub struct Cache<K, V> { /* ... */ }
pub trait Iterator<Item> { /* ... */ }
pub fn search<Query, Result>(q: Query) -> Result;

// Lifetime parameters: lowercase, descriptive
pub fn parse_document<'input>(content: &'input str) -> Document<'input>;
pub fn create_searcher<'index>(index: &'index Index) -> Searcher<'index>;
```

### Module Organization

**Clear Module Structure**
```rust
// lib.rs - Public API exports
pub use crate::search::{SearchIndex, SearchResults, SearchHit};
pub use crate::cache::{SearchCache, CacheConfig};
pub use crate::error::{CacheError, CacheResult};
pub use crate::config::Config;

// Re-export important traits
pub use crate::traits::{Searchable, Cacheable};

// Internal modules (not re-exported)
mod index;
mod parser;
mod storage;

// Public modules (re-exported selectively)
pub mod search;
pub mod cache;
pub mod error;
pub mod config;

// src/search/mod.rs - Module structure
use crate::error::{CacheError, CacheResult};
use crate::config::SearchConfig;

// Private submodules
mod engine;
mod query;
mod results;

// Public interface
pub use engine::SearchIndex;
pub use query::{Query, QueryParser};
pub use results::{SearchResults, SearchHit};

// Private implementation details
use engine::IndexEngine;
use query::ParsedQuery;

/// Main search functionality
/// 
/// This module provides high-level search operations built on top of Tantivy.
/// For low-level index operations, see the `index` module.
pub struct SearchIndex {
    engine: IndexEngine,
    config: SearchConfig,
}

impl SearchIndex {
    /// Create a new search index
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use blzr_core::search::SearchIndex;
    /// 
    /// let index = SearchIndex::new("./index_path")?;
    /// ```
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> CacheResult<Self> {
        let engine = IndexEngine::open(path.as_ref())?;
        let config = SearchConfig::default();
        
        Ok(Self { engine, config })
    }
}
```

## Type System Best Practices

### Rich Type Definitions

**Expressive Types**
```rust
use std::num::NonZeroU16;
use std::time::Duration;

/// Validated search query with enforced constraints
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedQuery {
    /// The original query string
    raw: String,
    /// Parsed query tree
    parsed: ParsedQuery,
    /// Search limits to prevent resource exhaustion
    limits: QueryLimits,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryLimits {
    /// Maximum results to return (guaranteed non-zero)
    max_results: NonZeroU16,
    /// Query execution timeout
    timeout: Duration,
    /// Maximum memory usage in bytes
    max_memory_bytes: NonZeroU32,
}

impl Default for QueryLimits {
    fn default() -> Self {
        Self {
            max_results: NonZeroU16::new(10).unwrap(),
            timeout: Duration::from_secs(30),
            max_memory_bytes: NonZeroU32::new(100 * 1024 * 1024).unwrap(), // 100MB
        }
    }
}

/// Document ID that prevents invalid values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentId(NonZeroU64);

impl DocumentId {
    /// Create a new document ID
    /// 
    /// Returns `None` if id is zero
    pub fn new(id: u64) -> Option<Self> {
        NonZeroU64::new(id).map(Self)
    }
    
    /// Get the numeric value of the ID
    pub fn get(self) -> u64 {
        self.0.get()
    }
}

/// Search score with guaranteed valid range [0.0, 1.0]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchScore(f32);

impl SearchScore {
    pub fn new(score: f32) -> Result<Self, InvalidScoreError> {
        if score < 0.0 || score > 1.0 || score.is_nan() {
            Err(InvalidScoreError { score })
        } else {
            Ok(Self(score))
        }
    }
    
    pub fn get(self) -> f32 {
        self.0
    }
    
    /// Maximum possible score
    pub const MAX: Self = Self(1.0);
    
    /// Minimum possible score  
    pub const MIN: Self = Self(0.0);
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid search score: {score} (must be between 0.0 and 1.0)")]
pub struct InvalidScoreError {
    score: f32,
}
```

### Smart Constructors

**Prevent Invalid States**
```rust
use std::path::{Path, PathBuf};
use std::collections::HashSet;

/// Index configuration with validation
#[derive(Debug, Clone)]
pub struct IndexConfig {
    path: PathBuf,
    schema_fields: Vec<FieldConfig>,
    settings: IndexSettings,
}

impl IndexConfig {
    /// Create a new index configuration
    /// 
    /// Validates that:
    /// - Path is accessible
    /// - At least one field is configured
    /// - Field names are unique
    /// - Settings are valid
    pub fn new<P: AsRef<Path>>(
        path: P,
        schema_fields: Vec<FieldConfig>,
        settings: IndexSettings,
    ) -> Result<Self, ConfigError> {
        let path = path.as_ref().to_path_buf();
        
        // Validate path
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(ConfigError::InvalidPath {
                    path: path.clone(),
                    reason: "Parent directory does not exist".to_string(),
                });
            }
        }
        
        // Validate fields
        if schema_fields.is_empty() {
            return Err(ConfigError::InvalidSchema {
                reason: "At least one field must be configured".to_string(),
            });
        }
        
        // Check for duplicate field names
        let mut field_names = HashSet::new();
        for field in &schema_fields {
            if !field_names.insert(&field.name) {
                return Err(ConfigError::InvalidSchema {
                    reason: format!("Duplicate field name: '{}'", field.name),
                });
            }
        }
        
        // Validate settings
        settings.validate()?;
        
        Ok(Self {
            path,
            schema_fields,
            settings,
        })
    }
    
    /// Get the index path
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    /// Get schema fields
    pub fn schema_fields(&self) -> &[FieldConfig] {
        &self.schema_fields
    }
    
    /// Get index settings
    pub fn settings(&self) -> &IndexSettings {
        &self.settings
    }
}

/// Field configuration with type safety
#[derive(Debug, Clone, PartialEq)]
pub struct FieldConfig {
    name: String,
    field_type: FieldType,
    options: FieldOptions,
}

impl FieldConfig {
    pub fn new<S: Into<String>>(
        name: S,
        field_type: FieldType,
        options: FieldOptions,
    ) -> Self {
        Self {
            name: name.into(),
            field_type,
            options,
        }
    }
    
    /// Create a text field with standard options
    pub fn text<S: Into<String>>(name: S) -> Self {
        Self::new(
            name,
            FieldType::Text,
            FieldOptions::text_default(),
        )
    }
    
    /// Create a keyword field (exact matching)
    pub fn keyword<S: Into<String>>(name: S) -> Self {
        Self::new(
            name,
            FieldType::Keyword,
            FieldOptions::keyword_default(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Text,
    Keyword, 
    Integer,
    Float,
    Date,
    Boolean,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldOptions {
    pub indexed: bool,
    pub stored: bool,
    pub tokenized: bool,
}

impl FieldOptions {
    pub fn text_default() -> Self {
        Self {
            indexed: true,
            stored: true,
            tokenized: true,
        }
    }
    
    pub fn keyword_default() -> Self {
        Self {
            indexed: true,
            stored: true,
            tokenized: false,
        }
    }
}
```

### Error Handling Patterns

**Structured Error Types**
```rust
use thiserror::Error;
use std::path::PathBuf;

/// Comprehensive error types for different failure modes
#[derive(Error, Debug)]
pub enum CacheError {
    // I/O and file system errors
    #[error("File system error: {operation} failed for '{path}'")]
    FileSystem {
        operation: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    
    // Parsing and validation errors
    #[error("Validation error in {field}: {message}")]
    Validation {
        field: String,
        message: String,
    },
    
    // External dependency errors
    #[error("Tantivy error during {operation}")]
    Tantivy {
        operation: String,
        #[source]
        source: tantivy::TantivyError,
    },
    
    // Resource limit errors
    #[error("Resource limit exceeded: {resource}")]
    ResourceLimit {
        resource: String,
        current: u64,
        maximum: u64,
    },
    
    // Configuration errors
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

// Convenient constructors
impl CacheError {
    pub fn file_system<S: Into<String>, P: Into<PathBuf>>(
        operation: S,
        path: P,
        source: std::io::Error,
    ) -> Self {
        Self::FileSystem {
            operation: operation.into(),
            path: path.into(),
            source,
        }
    }
    
    pub fn validation<S: Into<String>>(field: S, message: S) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }
    
    pub fn resource_limit<S: Into<String>>(resource: S, current: u64, maximum: u64) -> Self {
        Self::ResourceLimit {
            resource: resource.into(),
            current,
            maximum,
        }
    }
}

// Convenient conversion from common error types
impl From<std::io::Error> for CacheError {
    fn from(error: std::io::Error) -> Self {
        Self::FileSystem {
            operation: "unknown".to_string(),
            path: PathBuf::new(),
            source: error,
        }
    }
}

impl From<tantivy::TantivyError> for CacheError {
    fn from(error: tantivy::TantivyError) -> Self {
        Self::Tantivy {
            operation: "unknown".to_string(),
            source: error,
        }
    }
}
```

## Ownership and Borrowing

### Borrowing Best Practices

**Prefer Borrowing Over Cloning**
```rust
// ❌ Unnecessary clones
pub fn process_query(query: String, config: Config) -> SearchResults {
    let normalized = normalize_query(query.clone());
    let parsed = parse_query(normalized, config.clone());
    execute_search(parsed)
}

// ✅ Use borrowing
pub fn process_query(query: &str, config: &Config) -> SearchResults {
    let normalized = normalize_query(query);
    let parsed = parse_query(&normalized, config);
    execute_search(parsed)
}

// When ownership is needed, be explicit about it
pub fn process_and_blz_query(query: String, config: &Config) -> (SearchResults, String) {
    let normalized = normalize_query(&query);
    let parsed = parse_query(&normalized, config);
    let results = execute_search(parsed);
    
    // Return both results and owned query for caching
    (results, query)
}
```

### Lifetime Management

**Clear Lifetime Annotations**
```rust
/// Document parser that borrows from input string
pub struct DocumentParser<'input> {
    content: &'input str,
    current_position: usize,
}

impl<'input> DocumentParser<'input> {
    pub fn new(content: &'input str) -> Self {
        Self {
            content,
            current_position: 0,
        }
    }
    
    /// Extract title section, returning borrowed string slice
    pub fn extract_title(&mut self) -> Option<&'input str> {
        // Find title markers and return slice
        let start = self.find_title_start()?;
        let end = self.find_title_end(start)?;
        
        Some(&self.content[start..end])
    }
    
    /// Extract multiple sections, all borrowing from original input
    pub fn extract_sections(&mut self) -> Vec<DocumentSection<'input>> {
        let mut sections = Vec::new();
        
        while let Some(section) = self.next_section() {
            sections.push(section);
        }
        
        sections
    }
}

#[derive(Debug)]
pub struct DocumentSection<'input> {
    pub title: &'input str,
    pub content: &'input str,
    pub level: u8,
}

// When multiple lifetimes are needed
pub fn merge_documents<'a, 'b>(
    primary: &'a Document,
    secondary: &'b Document,
) -> MergedDocument<'a, 'b> {
    MergedDocument {
        title: primary.title, // Borrows from 'a
        content: secondary.content, // Borrows from 'b
        metadata: combine_metadata(&primary.metadata, &secondary.metadata),
    }
}

pub struct MergedDocument<'a, 'b> {
    pub title: &'a str,
    pub content: &'b str,
    pub metadata: Metadata,
}
```

### Smart Pointer Usage

**When to Use Arc, Rc, Box**
```rust
use std::sync::Arc;
use std::rc::Rc;

// Use Box for owned data on the heap
pub struct LargeConfig {
    // Large configuration that should live on heap
    data: Box<ConfigData>,
}

// Use Arc for shared ownership across threads
pub struct SharedIndex {
    // Index can be shared across multiple threads
    tantivy_index: Arc<tantivy::Index>,
    config: Arc<IndexConfig>,
}

impl SharedIndex {
    pub fn clone_handle(&self) -> Self {
        Self {
            tantivy_index: Arc::clone(&self.tantivy_index),
            config: Arc::clone(&self.config),
        }
    }
}

// Use Rc for shared ownership in single-threaded contexts
pub struct SingleThreadedCache {
    config: Rc<CacheConfig>,
    storage: Rc<RefCell<HashMap<String, CachedResult>>>,
}

// Prefer borrowing when possible
pub struct SearchEngine<'a> {
    // Borrow instead of Arc when lifetime allows
    index: &'a tantivy::Index,
    config: &'a SearchConfig,
}
```

## Trait Design

### Effective Trait Patterns

**Small, Focused Traits**
```rust
/// Core search functionality
pub trait Searchable {
    type Query;
    type Result;
    type Error;
    
    fn search(&self, query: Self::Query) -> Result<Self::Result, Self::Error>;
}

/// Caching capability
pub trait Cacheable {
    type Key;
    type Value;
    
    fn get(&self, key: &Self::Key) -> Option<&Self::Value>;
    fn put(&mut self, key: Self::Key, value: Self::Value);
    fn clear(&mut self);
}

/// Configurable components
pub trait Configurable {
    type Config;
    
    fn configure(&mut self, config: Self::Config) -> Result<(), ConfigError>;
    fn get_config(&self) -> &Self::Config;
}

// Blanket implementations for common patterns
impl<T> Cacheable for std::collections::HashMap<String, T> {
    type Key = String;
    type Value = T;
    
    fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
        self.get(key)
    }
    
    fn put(&mut self, key: Self::Key, value: Self::Value) {
        self.insert(key, value);
    }
    
    fn clear(&mut self) {
        self.clear();
    }
}

// Compound traits for related functionality
pub trait SearchCache: Searchable + Cacheable {
    fn search_or_blz(&mut self, query: Self::Query) -> Result<Self::Result, Self::Error>
    where
        Self::Key: From<Self::Query>,
        Self::Value: From<Self::Result>,
        Self::Result: Clone,
    {
        let cache_key = Self::Key::from(query.clone());
        
        if let Some(cached) = self.get(&cache_key) {
            return Ok(Self::Result::from(cached.clone()));
        }
        
        let result = self.search(query)?;
        self.put(cache_key, Self::Value::from(result.clone()));
        Ok(result)
    }
}
```

### Generic Programming

**Effective Use of Generics**
```rust
use std::marker::PhantomData;
use std::hash::Hash;

/// Generic cache with configurable storage backend
pub struct Cache<K, V, S> 
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: StorageBackend<K, V>,
{
    storage: S,
    max_size: usize,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V, S> Cache<K, V, S>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: StorageBackend<K, V>,
{
    pub fn new(storage: S, max_size: usize) -> Self {
        Self {
            storage,
            max_size,
            _phantom: PhantomData,
        }
    }
    
    pub fn get(&self, key: &K) -> Option<V> {
        self.storage.get(key)
    }
    
    pub fn put(&mut self, key: K, value: V) -> Result<(), CacheError> {
        if self.storage.len() >= self.max_size {
            self.storage.evict_oldest()?;
        }
        
        self.storage.insert(key, value)
    }
}

/// Storage backend trait for different cache implementations
pub trait StorageBackend<K, V> {
    fn get(&self, key: &K) -> Option<V>;
    fn insert(&mut self, key: K, value: V) -> Result<(), CacheError>;
    fn remove(&mut self, key: &K) -> Option<V>;
    fn len(&self) -> usize;
    fn evict_oldest(&mut self) -> Result<(), CacheError>;
}

// Concrete implementations
pub struct MemoryStorage<K, V> {
    data: std::collections::HashMap<K, V>,
}

pub struct LruStorage<K, V> {
    data: linked_hash_map::LinkedHashMap<K, V>,
}

// Type aliases for common configurations
pub type MemoryCache<K, V> = Cache<K, V, MemoryStorage<K, V>>;
pub type LruCache<K, V> = Cache<K, V, LruStorage<K, V>>;

// Associated type patterns for complex relationships
pub trait QueryProcessor {
    type Input;
    type Output;
    type Config;
    type Error;
    
    fn process(
        &self,
        input: Self::Input,
        config: &Self::Config,
    ) -> Result<Self::Output, Self::Error>;
}

pub struct TantivyQueryProcessor;

impl QueryProcessor for TantivyQueryProcessor {
    type Input = String;
    type Output = ParsedQuery;
    type Config = QueryParserConfig;
    type Error = QueryParseError;
    
    fn process(
        &self,
        input: Self::Input,
        config: &Self::Config,
    ) -> Result<Self::Output, Self::Error> {
        // Implementation
        todo!()
    }
}
```

## Documentation Conventions

### Comprehensive Documentation

**Doc Comments and Examples**
```rust
//! Search cache implementation using Tantivy
//!
//! This crate provides a high-performance search cache built on top of Tantivy,
//! with features including:
//!
//! - Full-text search with boolean queries
//! - LRU caching with memory limits
//! - Concurrent search operations
//! - Shell integration for CLI usage
//!
//! # Quick Start
//!
//! ```rust
//! use blzr_core::{SearchIndex, SearchCache};
//!
//! # tokio_test::block_on(async {
//! // Create an index
//! let index = SearchIndex::new("./my_index").await?;
//!
//! // Create a cache
//! let mut cache = SearchCache::new(index)?;
//!
//! // Search with caching
//! let results = cache.search("rust programming", 10).await?;
//! println!("Found {} results", results.hits.len());
//! # Ok::<(), blzr_core::CacheError>(())
//! # });
//! ```

use std::path::Path;
use crate::error::{CacheError, CacheResult};

/// High-performance search index with caching
///
/// `SearchIndex` provides full-text search capabilities using Tantivy as the
/// underlying search engine. It supports:
///
/// - Boolean queries with field-specific search
/// - Phrase queries and wildcard matching
/// - Result scoring and ranking
/// - Concurrent search operations
///
/// # Thread Safety
///
/// `SearchIndex` is thread-safe and can be shared across multiple threads
/// using `Arc`. Search operations are read-only and can be performed
/// concurrently.
///
/// # Memory Usage
///
/// The index maintains an in-memory cache of recent search results to
/// improve performance. Cache size can be configured using [`CacheConfig`].
///
/// # Examples
///
/// Basic search operation:
///
/// ```rust
/// use blzr_core::SearchIndex;
///
/// # tokio_test::block_on(async {
/// let index = SearchIndex::new("./search_index").await?;
///
/// // Simple term search
/// let results = index.search("rust", 10).await?;
/// 
/// // Field-specific search
/// let results = index.search("title:programming", 5).await?;
///
/// // Boolean query
/// let results = index.search("rust AND (programming OR tutorial)", 20).await?;
/// # Ok::<(), blzr_core::CacheError>(())
/// # });
/// ```
///
/// Advanced usage with configuration:
///
/// ```rust
/// use blzr_core::{SearchIndex, IndexConfig};
///
/// # tokio_test::block_on(async {
/// let config = IndexConfig::new()
///     .with_blz_size(1000)
///     .with_max_query_length(500);
///
/// let index = SearchIndex::with_config("./search_index", config).await?;
/// let results = index.search("complex query here", 50).await?;
/// # Ok::<(), blzr_core::CacheError>(())
/// # });
/// ```
pub struct SearchIndex {
    // Internal implementation details
}

impl SearchIndex {
    /// Create a new search index at the specified path
    ///
    /// This function will create a new Tantivy index if one doesn't exist,
    /// or open an existing index if found. The index directory will be
    /// created if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path where the index files will be stored
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `SearchIndex` on success, or a
    /// [`CacheError`] if the index cannot be created or opened.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    ///
    /// - The path is not accessible (permissions, disk space, etc.)
    /// - An existing index at the path is corrupted
    /// - The directory cannot be created
    /// - The index schema is incompatible
    ///
    /// # Examples
    ///
    /// ```rust
    /// use blzr_core::SearchIndex;
    ///
    /// # tokio_test::block_on(async {
    /// // Create index in current directory
    /// let index = SearchIndex::new("./my_index").await?;
    ///
    /// // Create index with absolute path
    /// let index = SearchIndex::new("/tmp/search_index").await?;
    /// # Ok::<(), blzr_core::CacheError>(())
    /// # });
    /// ```
    ///
    /// # Platform Differences
    ///
    /// On Windows, paths should use forward slashes or escaped backslashes:
    ///
    /// ```rust,no_run
    /// # tokio_test::block_on(async {
    /// // Good
    /// let index = SearchIndex::new("C:/indexes/my_index").await?;
    /// let index = SearchIndex::new("C:\\\\indexes\\\\my_index").await?;
    /// # Ok::<(), blzr_core::CacheError>(())
    /// # });
    /// ```
    pub async fn new<P: AsRef<Path>>(path: P) -> CacheResult<Self> {
        // Implementation details...
        todo!()
    }
    
    /// Search the index for documents matching the query
    ///
    /// Executes a search query against the index and returns matching documents
    /// ranked by relevance score. Results are automatically cached for faster
    /// subsequent searches.
    ///
    /// # Query Syntax
    ///
    /// The query parser supports:
    ///
    /// - **Term queries**: `rust` matches documents containing "rust"
    /// - **Field queries**: `title:programming` matches "programming" in title field
    /// - **Phrase queries**: `"rust programming"` matches the exact phrase
    /// - **Boolean queries**: `rust AND programming`, `rust OR python`
    /// - **Negation**: `rust NOT deprecated`
    /// - **Wildcards**: `program*` matches "programming", "programmer", etc.
    /// - **Grouping**: `(rust OR go) AND tutorial`
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string using supported query syntax
    /// * `limit` - Maximum number of results to return (1-1000)
    ///
    /// # Returns
    ///
    /// Returns a [`SearchResults`] containing:
    /// - Matching documents with relevance scores
    /// - Total count of matches (may exceed limit)
    /// - Execution time and caching information
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if:
    /// - Query syntax is invalid or too complex
    /// - Limit is outside valid range (1-1000)
    /// - Index is corrupted or inaccessible
    /// - Query execution times out (>30 seconds)
    ///
    /// # Examples
    ///
    /// Simple searches:
    ///
    /// ```rust
    /// # use blzr_core::SearchIndex;
    /// # tokio_test::block_on(async {
    /// # let index = SearchIndex::new("./test_index").await?;
    /// // Find documents about Rust
    /// let results = index.search("rust", 10).await?;
    /// println!("Found {} documents", results.hits.len());
    ///
    /// // Search in specific field
    /// let results = index.search("title:tutorial", 5).await?;
    ///
    /// // Case insensitive search
    /// let results = index.search("RUST", 10).await?; // Same as "rust"
    /// # Ok::<(), blzr_core::CacheError>(())
    /// # });
    /// ```
    ///
    /// Complex queries:
    ///
    /// ```rust
    /// # use blzr_core::SearchIndex;
    /// # tokio_test::block_on(async {
    /// # let index = SearchIndex::new("./test_index").await?;
    /// // Boolean combination
    /// let results = index.search("rust AND programming", 20).await?;
    ///
    /// // Field-specific boolean query
    /// let results = index.search("title:rust OR body:golang", 15).await?;
    ///
    /// // Phrase search with negation
    /// let results = index.search("\"web development\" NOT deprecated", 10).await?;
    /// # Ok::<(), blzr_core::CacheError>(())
    /// # });
    /// ```
    ///
    /// # Performance
    ///
    /// - First execution: 10-100ms depending on index size and query complexity
    /// - Cached execution: <1ms for identical queries
    /// - Memory usage: ~1KB per cached result set
    ///
    /// # See Also
    ///
    /// - [`SearchResults`] for result format details
    /// - [`CacheConfig`] for cache configuration options
    /// - [Query syntax guide](../query_syntax/index.html) for detailed syntax documentation
    pub async fn search(&self, query: &str, limit: u16) -> CacheResult<SearchResults> {
        // Implementation details...
        todo!()
    }
}
```

Remember: Good Rust code is self-documenting through expressive types, clear naming, and comprehensive documentation. The compiler is your friend—use its type system to catch bugs at compile time rather than runtime.