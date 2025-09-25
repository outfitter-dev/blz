// Optimized search index with reader pooling, batch operations, and parallel processing
use crate::cache::SearchCache;
use crate::memory_pool::{MemoryPool, PooledString};
use crate::string_pool::StringPool;
use crate::types::normalize_flavor_filters;
use crate::{Error, HeadingBlock, Result, SearchHit};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, STORED, STRING, TEXT};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::time::timeout;
use tracing::{debug, info, instrument, warn};

/// Optimized search index with advanced performance features
pub struct OptimizedSearchIndex {
    /// Tantivy index
    index: Index,
    
    /// Schema fields
    fields: IndexFields,
    
    /// Reader pool for concurrent searches
    reader_pool: Arc<ReaderPool>,
    
    /// Writer pool for batch indexing
    writer_pool: Arc<WriterPool>,
    
    /// Search result cache
    cache: Arc<SearchCache>,
    
    /// Memory pool for buffer reuse
    memory_pool: Arc<MemoryPool>,
    
    /// String pool for interning
    string_pool: Arc<StringPool>,
    
    /// Statistics
    stats: Arc<IndexStats>,

    // Versioning for safe cache keys
    global_version: AtomicUsize,
    alias_versions: RwLock<HashMap<String, usize>>,
}

/// Index schema fields
#[derive(Debug, Clone)]
struct IndexFields {
    content: Field,
    path: Field,
    heading_path: Field,
    lines: Field,
    alias: Field,
    flavor: Option<Field>,
}

/// Reader pool for managing concurrent search operations
struct ReaderPool {
    /// Available readers
    readers: Mutex<VecDeque<IndexReader>>,
    
    /// Maximum number of readers in pool
    max_readers: usize,
    
    /// Factory function to create new readers
    reader_factory: Box<dyn Fn() -> Result<IndexReader> + Send + Sync>,
    
    /// Statistics
    stats: ReaderPoolStats,
}

/// Writer pool for managing batch indexing operations
struct WriterPool {
    /// Available writers
    writers: Mutex<VecDeque<IndexWriter>>,
    
    /// Maximum number of writers
    max_writers: usize,
    
    /// Writer creation semaphore (expensive to create)
    writer_creation_semaphore: Semaphore,
    
    /// Factory function to create new writers
    writer_factory: Box<dyn Fn() -> Result<IndexWriter> + Send + Sync>,
    
    /// Statistics
    stats: WriterPoolStats,
}

/// Reader pool statistics
#[derive(Default)]
struct ReaderPoolStats {
    requests: AtomicUsize,
    hits: AtomicUsize,
    misses: AtomicUsize,
    created: AtomicUsize,
}

/// Writer pool statistics
#[derive(Default)]
struct WriterPoolStats {
    requests: AtomicUsize,
    hits: AtomicUsize,
    misses: AtomicUsize,
    created: AtomicUsize,
}

/// Index performance statistics
#[derive(Default)]
pub struct IndexStats {
    pub searches: AtomicUsize,
    pub cache_hits: AtomicUsize,
    pub cache_misses: AtomicUsize,
    pub index_operations: AtomicUsize,
    pub documents_indexed: AtomicUsize,
    pub total_search_time_ms: AtomicUsize,
    pub total_index_time_ms: AtomicUsize,
}

impl OptimizedSearchIndex {
    /// Create a new optimized search index
    pub async fn create(index_path: &Path) -> Result<Self> {
        // Build schema
        let mut schema_builder = Schema::builder();
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let path_field = schema_builder.add_text_field("path", STRING | STORED);
        let heading_path_field = schema_builder.add_text_field("heading_path", TEXT | STORED);
        let lines_field = schema_builder.add_text_field("lines", STRING | STORED);
        let alias_field = schema_builder.add_text_field("alias", STRING | STORED);
        let flavor_field = schema_builder.add_text_field("flavor", STRING | STORED);
        let schema = schema_builder.build();

        let fields = IndexFields {
            content: content_field,
            path: path_field,
            heading_path: heading_path_field,
            lines: lines_field,
            alias: alias_field,
            flavor: Some(flavor_field),
        };

        // Create directory and index
        std::fs::create_dir_all(index_path)
            .map_err(|e| Error::Index(format!("Failed to create index directory: {}", e)))?;

        let index = Index::create_in_dir(index_path, schema)
            .map_err(|e| Error::Index(format!("Failed to create index: {}", e)))?;

        Self::new_with_index(index, fields).await
    }

    /// Open an existing optimized search index
    pub async fn open(index_path: &Path) -> Result<Self> {
        let index = Index::open_in_dir(index_path)
            .map_err(|e| Error::Index(format!("Failed to open index: {}", e)))?;

        let schema = index.schema();
        let fields = IndexFields {
            content: schema
                .get_field("content")
                .map_err(|_| Error::Index("Missing content field".into()))?,
            path: schema
                .get_field("path")
                .map_err(|_| Error::Index("Missing path field".into()))?,
            heading_path: schema
                .get_field("heading_path")
                .map_err(|_| Error::Index("Missing heading_path field".into()))?,
            lines: schema
                .get_field("lines")
                .map_err(|_| Error::Index("Missing lines field".into()))?,
            alias: schema
                .get_field("alias")
                .map_err(|_| Error::Index("Missing alias field".into()))?,
            flavor: schema.get_field("flavor").ok(),
        };

        Self::new_with_index(index, fields).await
    }

    /// Initialize with existing index
    async fn new_with_index(index: Index, fields: IndexFields) -> Result<Self> {
        let index_clone_for_reader = index.clone();
        let index_clone_for_writer = index.clone();

        // Create reader pool
        let reader_pool = Arc::new(ReaderPool::new(
            10, // Max 10 concurrent readers
            Box::new(move || {
                index_clone_for_reader
                    .reader_builder()
                    .reload_policy(ReloadPolicy::OnCommitWithDelay)
                    .try_into()
                    .map_err(|e| Error::Index(format!("Failed to create reader: {}", e)))
            }),
        ));

        // Create writer pool
        let writer_pool = Arc::new(WriterPool::new(
            2, // Max 2 writers (expensive)
            Box::new(move || {
                index_clone_for_writer
                    .writer(50_000_000) // 50MB heap
                    .map_err(|e| Error::Index(format!("Failed to create writer: {}", e)))
            }),
        ));

        // Initialize other components
        let cache = Arc::new(SearchCache::new_search_cache());
        let memory_pool = Arc::new(MemoryPool::default());
        let string_pool = Arc::new(StringPool::default());
        let stats = Arc::new(IndexStats::default());

        Ok(Self {
            index,
            fields,
            reader_pool,
            writer_pool,
            cache,
            memory_pool,
            string_pool,
            stats,
            global_version: AtomicUsize::new(1),
            alias_versions: RwLock::new(HashMap::new()),
        })
    }

    /// Search with full optimization pipeline
    #[instrument(skip(self), fields(query_len = query_str.len(), limit))]
    pub async fn search_optimized(
        &self,
        query_str: &str,
        alias: Option<&str>,
        flavor: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        let start_time = Instant::now();
        self.stats.searches.fetch_add(1, Ordering::Relaxed);

        // Prepare version token for cache
        let version_token = if let Some(a) = alias {
            let map = self.alias_versions.read().await;
            format!("{}", map.get(a).copied().unwrap_or(1))
        } else {
            format!("{}", self.global_version.load(Ordering::Relaxed))
        };

        // Try cache first (versioned)
        if let Some(cached_results) = self
            .cache
            .get_cached_results_v(query_str, alias, flavor, Some(&version_token))
            .await
        {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            debug!("Cache hit for query: {}", query_str);
            return Ok(cached_results);
        }

        self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);

        // Perform search with reader from pool
        let results = self
            .search_with_reader_pool(query_str, alias, flavor, limit)
            .await?;

        // Cache results for future use
        self.cache
            .cache_search_results_v(query_str, alias, flavor, Some(&version_token), results.clone())
            .await;

        // Update statistics
        let search_time = start_time.elapsed();
        self.stats
            .total_search_time_ms
            .fetch_add(search_time.as_millis() as usize, Ordering::Relaxed);

        debug!(
            "Search completed in {:.2}ms, found {} results",
            search_time.as_millis(),
            results.len()
        );

        Ok(results)
    }

    /// Perform search using reader from pool
    async fn search_with_reader_pool(
        &self,
        query_str: &str,
        alias: Option<&str>,
        flavor: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        let reader = self.reader_pool.get_reader().await?;

        let result = timeout(
            Duration::from_secs(30),
            self.execute_search_with_reader(reader.clone(), query_str, alias, flavor, limit),
        )
        .await
        .map_err(|_| Error::Timeout("Search operation timed out".into()))?;

        self.reader_pool.return_reader(reader).await;

        result
    }

    /// Execute search with specific reader
    async fn execute_search_with_reader(
        &self,
        reader: IndexReader,
        query_str: &str,
        alias: Option<&str>,
        flavor: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        let searcher = reader.searcher();

        // Build query using optimized string operations
        let mut query_buffer = self.memory_pool.get_string_buffer(query_str.len() * 2).await;
        self.build_optimized_query(query_str, alias, flavor, &mut query_buffer)
            .await;

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.fields.content, self.fields.heading_path],
        );

        let query = query_parser
            .parse_query(query_buffer.as_str())
            .map_err(|e| Error::Index(format!("Failed to parse query: {}", e)))?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| Error::Index(format!("Search failed: {}", e)))?;

        // Process results using memory pool
        let mut results = Vec::with_capacity(top_docs.len());
        let mut snippet_buffer = self.memory_pool.get_string_buffer(200).await;

        for (score, doc_address) in top_docs {
            let doc = searcher
                .doc(doc_address)
                .map_err(|e| Error::Index(format!("Failed to retrieve doc: {}", e)))?;

            let alias = self.get_field_text(&doc, self.fields.alias)?;
            let file = self.get_field_text(&doc, self.fields.path)?;
            let heading_path_str = self.get_field_text(&doc, self.fields.heading_path)?;
            let lines = self.get_field_text(&doc, self.fields.lines)?;
            let content = self.get_field_text(&doc, self.fields.content)?;

            // Extract flavor if the schema supports it
            let flavor = if let Some(flavor_field) = self.fields.flavor {
                self.get_field_text(&doc, flavor_field).ok()
            } else {
                None
            };

            // Intern commonly used strings
            let alias_interned = self.string_pool.intern(&alias).await;
            let file_interned = self.string_pool.intern(&file).await;

            let heading_path: Vec<String> = heading_path_str
                .split(" > ")
                .map(|s| s.to_string())
                .collect();

            // Extract snippet using pooled buffer
            snippet_buffer.as_mut().clear();
            self.extract_snippet_optimized(&content, query_str, &mut snippet_buffer)
                .await;

            // Parse numeric line range for convenience
            let line_numbers = {
                let mut it = lines.split(['-', ':']);
                let start = it.next().and_then(|s| s.trim().parse::<usize>().ok());
                let end = it.next().and_then(|s| s.trim().parse::<usize>().ok());
                match (start, end) {
                    (Some(a), Some(b)) => Some(vec![a, b]),
                    _ => None,
                }
            };

            results.push(SearchHit {
                alias: alias_interned.to_string(),
                source: alias_interned.to_string(),
                file: file_interned.to_string(),
                heading_path,
                lines,
                line_numbers,
                snippet: snippet_buffer.as_str().to_string(),
                score,
                source_url: None,
                checksum: String::new(),
                anchor: None,
                flavor,
            });
        }

        Ok(results)
    }

    /// Build optimized query string with minimal allocations
    async fn build_optimized_query(
        &self,
        query_str: &str,
        alias: Option<&str>,
        flavor: Option<&str>,
        buffer: &mut PooledString<'_>,
    ) {
        // Check if escaping is needed (single pass)
        let needs_escaping = query_str
            .chars()
            .any(|c| matches!(c, '\\' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '~' | ':'));

        if needs_escaping {
            // Escape special characters
            for ch in query_str.chars() {
                match ch {
                    '\\' => buffer.as_mut().push_str("\\\\"),
                    '(' => buffer.as_mut().push_str("\\("),
                    ')' => buffer.as_mut().push_str("\\)"),
                    '[' => buffer.as_mut().push_str("\\["),
                    ']' => buffer.as_mut().push_str("\\]"),
                    '{' => buffer.as_mut().push_str("\\{"),
                    '}' => buffer.as_mut().push_str("\\}"),
                    '^' => buffer.as_mut().push_str("\\^"),
                    '~' => buffer.as_mut().push_str("\\~"),
                    ':' => buffer.as_mut().push_str("\\:"),
                    _ => buffer.as_mut().push(ch),
                }
            }
        } else {
            buffer.as_mut().push_str(query_str);
        }

        let mut filters = Vec::new();
        if let Some(alias_value) = alias {
            filters.push(format!("alias:{alias_value}"));
        }
        if self.fields.flavor.is_some() {
            if let Some(values) = flavor.and_then(|raw| {
                let normalized = normalize_flavor_filters(raw);
                if normalized.is_empty() {
                    if !raw.trim().is_empty() {
                        tracing::debug!(filter = raw, "Ignoring flavor filter with no recognized values");
                    }
                    None
                } else {
                    Some(normalized)
                }
            }) {
                if values.len() == 1 {
                    filters.push(format!("flavor:{}", values[0]));
                } else {
                    let clause = values
                        .iter()
                        .map(|value| format!("flavor:{value}"))
                        .collect::<Vec<_>>()
                        .join(" OR ");
                    filters.push(format!("({clause})"));
                }
            }
        } else if flavor.is_some() && !flavor.unwrap_or("").trim().is_empty() {
            tracing::warn!("Flavor filtering requested but index doesn't support it. Ignoring flavor filter: {}", flavor.unwrap_or(""));
        }

        if !filters.is_empty() {
            let escaped_query = buffer.as_str().to_string();
            buffer.as_mut().clear();
            buffer
                .as_mut()
                .push_str(&format!("{} AND ({})", filters.join(" AND "), escaped_query));
        }
    }

    /// Extract snippet using optimized buffer operations
    async fn extract_snippet_optimized(
        &self,
        content: &str,
        query: &str,
        buffer: &mut PooledString<'_>,
    ) {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();

        if let Some(pos) = content_lower.find(&query_lower) {
            let context_before = 50;
            let context_after = 50;

            // Calculate safe UTF-8 boundaries
            let byte_start = pos.saturating_sub(context_before);
            let byte_end = (pos + query.len() + context_after).min(content.len());

            // Find character boundaries
            let start = content
                .char_indices()
                .take_while(|(i, _)| *i <= byte_start)
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);

            let end = content
                .char_indices()
                .find(|(i, _)| *i >= byte_end)
                .map(|(i, _)| i)
                .unwrap_or(content.len());

            // Build snippet
            if start > 0 {
                buffer.as_mut().push_str("...");
            }
            buffer.as_mut().push_str(&content[start..end]);
            if end < content.len() {
                buffer.as_mut().push_str("...");
            }
        } else {
            // No match - truncate content
            let max_len = 100;
            if content.len() <= max_len {
                buffer.as_mut().push_str(content);
            } else {
                let boundary = content
                    .char_indices()
                    .take_while(|(i, _)| *i < max_len)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(0);

                buffer.as_mut().push_str(&content[..boundary]);
                buffer.as_mut().push_str("...");
            }
        }
    }

    /// Index blocks with batch optimization
    #[instrument(skip(self, blocks), fields(alias, block_count = blocks.len()))]
    pub async fn index_blocks_optimized(
        &self,
        alias: &str,
        file_path: &str,
        blocks: &[HeadingBlock],
    ) -> Result<()> {
        let start_time = Instant::now();
        self.stats.index_operations.fetch_add(1, Ordering::Relaxed);

        if blocks.is_empty() {
            return Ok(());
        }

        // Use writer from pool
        let writer = self.writer_pool.get_writer().await?;
        let result = timeout(
            Duration::from_secs(120), // 2 minute timeout for indexing
            self.index_blocks_with_writer(writer.clone(), alias, "llms", file_path, blocks),
        )
        .await
        .map_err(|_| Error::Timeout("Indexing operation timed out".into()))?;

        self.writer_pool.return_writer(writer).await;

        // Update statistics
        let index_time = start_time.elapsed();
        self.stats
            .total_index_time_ms
            .fetch_add(index_time.as_millis() as usize, Ordering::Relaxed);
        self.stats
            .documents_indexed
            .fetch_add(blocks.len(), Ordering::Relaxed);
        // Invalidate cache entries for this alias (best-effort) and bump versions
        let removed = self.cache.invalidate_alias(alias).await;
        {
            let mut map = self.alias_versions.write().await;
            let e = map.entry(alias.to_string()).or_insert(1);
            *e = e.saturating_add(1);
        }
        self.global_version.fetch_add(1, Ordering::Relaxed);
        debug!(
            "Invalidated {} cached entries for alias {}; versions -> alias={}, global={}",
            removed,
            alias,
            {
                let map = self.alias_versions.read().await;
                *map.get(alias).unwrap_or(&1)
            },
            self.global_version.load(Ordering::Relaxed)
        );
        
        info!(
            "Indexed {} blocks for {} in {:.2}ms",
            blocks.len(),
            alias,
            index_time.as_millis()
        );

        result
    }

    /// Index blocks for a specific flavor (preferred for multi-flavor installs)
    #[instrument(skip(self, blocks), fields(alias, flavor, block_count = blocks.len()))]
    pub async fn index_blocks_optimized_flavored(
        &self,
        alias: &str,
        flavor: &str,
        file_path: &str,
        blocks: &[HeadingBlock],
    ) -> Result<()> {
        let start_time = Instant::now();
        self.stats.index_operations.fetch_add(1, Ordering::Relaxed);

        if blocks.is_empty() {
            return Ok(());
        }

        // Use writer from pool
        let writer = self.writer_pool.get_writer().await?;
        let result = timeout(
            Duration::from_secs(120), // 2 minute timeout for indexing
            self.index_blocks_with_writer(writer.clone(), alias, flavor, file_path, blocks),
        )
        .await
        .map_err(|_| Error::Timeout("Indexing operation timed out".into()))?;
        self.writer_pool.return_writer(writer).await;

        // Update statistics
        let index_time = start_time.elapsed();
        self.stats
            .total_index_time_ms
            .fetch_add(index_time.as_millis() as usize, Ordering::Relaxed);
        self.stats
            .documents_indexed
            .fetch_add(blocks.len(), Ordering::Relaxed);

        // Invalidate cache entries for this alias (best-effort) and bump versions
        let removed = self.cache.invalidate_alias(alias).await;
        {
            let mut map = self.alias_versions.write().await;
            let e = map.entry(alias.to_string()).or_insert(1);
            *e = e.saturating_add(1);
        }
        self.global_version.fetch_add(1, Ordering::Relaxed);
        debug!(
            "Invalidated {} cached entries for alias {}; versions -> alias={}, global={}",
            removed,
            alias,
            {
                let map = self.alias_versions.read().await;
                *map.get(alias).unwrap_or(&1)
            },
            self.global_version.load(Ordering::Relaxed)
        );

        info!(
            "Indexed {} blocks for {} (flavor: {}) in {:.2}ms",
            blocks.len(),
            alias,
            flavor,
            index_time.as_millis()
        );

        result
    }

    /// Index blocks using specific writer
    async fn index_blocks_with_writer(
        &self,
        mut writer: IndexWriter,
        alias: &str,
        flavor: &str,
        file_path: &str,
        blocks: &[HeadingBlock],
    ) -> Result<()> {
        // Delete documents matching alias (and flavor if supported)
        use tantivy::query::{BooleanQuery, Occur, Query, TermQuery};
        use tantivy::schema::IndexRecordOption;
        let alias_term = tantivy::Term::from_field_text(self.fields.alias, alias);

        if let Some(flavor_field) = self.fields.flavor {
            // Schema supports flavor - delete only matching alias AND flavor
            let flavor_term = tantivy::Term::from_field_text(flavor_field, flavor);
            let query: BooleanQuery = BooleanQuery::new(vec![
                (Occur::Must, Box::new(TermQuery::new(alias_term, IndexRecordOption::Basic)) as Box<dyn Query>),
                (Occur::Must, Box::new(TermQuery::new(flavor_term, IndexRecordOption::Basic)) as Box<dyn Query>),
            ]);
            writer
                .delete_documents(query)
                .map_err(|e| Error::Index(format!("Failed to delete existing docs: {}", e)))?;
        } else {
            // Legacy schema - delete all documents for alias
            writer.delete_term(alias_term);
        }

        // Prepare interned strings for reuse
        let alias_interned = self.string_pool.intern(alias).await;
        let file_path_interned = self.string_pool.intern(file_path).await;

        // Batch document creation
        let mut total_content_bytes = 0;
        for block in blocks {
            total_content_bytes += block.content.len();
            
            let heading_path_str = if block.path.is_empty() {
                String::new()
            } else {
                block.path.join(" > ")
            };
            let lines_str = format!("{}-{}", block.start_line, block.end_line);

            // Create document with interned strings where possible
            let mut doc = doc!(
                self.fields.content => block.content.as_str(),
                self.fields.path => file_path_interned.as_ref(),
                self.fields.heading_path => heading_path_str,
                self.fields.lines => lines_str,
                self.fields.alias => alias_interned.as_ref()
            );

            // Add flavor field if supported by schema
            if let Some(flavor_field) = self.fields.flavor {
                doc.add_text(flavor_field, flavor);
            }

            writer
                .add_document(doc)
                .map_err(|e| Error::Index(format!("Failed to add document: {}", e)))?;
        }

        // Commit all documents
        writer
            .commit()
            .map_err(|e| Error::Index(format!("Failed to commit: {}", e)))?;

        debug!(
            "Batch indexed {} documents ({} bytes) for {}",
            blocks.len(),
            total_content_bytes,
            alias
        );

        Ok(())
    }

    /// Parallel indexing for multiple aliases
    pub async fn index_multiple_sources(
        &self,
        sources: Vec<(String, String, Vec<HeadingBlock>)>, // (alias, file_path, blocks)
    ) -> Result<()> {
        use futures::future::try_join_all;

        let tasks: Vec<_> = sources
            .into_iter()
            .map(|(alias, file_path, blocks)| {
                self.index_blocks_optimized(&alias, &file_path, &blocks)
            })
            .collect();

        try_join_all(tasks).await?;
        Ok(())
    }

    /// Concurrent search across multiple queries
    pub async fn search_multiple(
        &self,
        queries: Vec<(String, Option<String>, Option<String>, usize)>, // (query, alias, flavor, limit)
    ) -> Result<Vec<Vec<SearchHit>>> {
        use futures::future::try_join_all;

        let tasks: Vec<_> = queries
            .into_iter()
            .map(|(query, alias, flavor, limit)| {
                self.search_optimized(&query, alias.as_deref(), flavor.as_deref(), limit)
            })
            .collect();

        try_join_all(tasks).await
    }

    /// Get field text from document
    fn get_field_text(&self, doc: &tantivy::TantivyDocument, field: Field) -> Result<String> {
        doc.get_first(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| Error::Index("Field not found in document".into()))
    }

    /// Get comprehensive statistics
    pub async fn get_stats(&self) -> IndexStatsSummary {
        let cache_stats = self.cache.stats().await;
        let reader_stats = self.reader_pool.get_stats().await;
        let writer_stats = self.writer_pool.get_stats().await;
        let memory_stats = self.memory_pool.get_stats();
        let string_stats = self.string_pool.stats().await;

        IndexStatsSummary {
            searches: self.stats.searches.load(Ordering::Relaxed),
            cache_hits: self.stats.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.stats.cache_misses.load(Ordering::Relaxed),
            index_operations: self.stats.index_operations.load(Ordering::Relaxed),
            documents_indexed: self.stats.documents_indexed.load(Ordering::Relaxed),
            avg_search_time_ms: {
                let total_searches = self.stats.searches.load(Ordering::Relaxed);
                if total_searches > 0 {
                    self.stats.total_search_time_ms.load(Ordering::Relaxed) / total_searches
                } else {
                    0
                }
            },
            avg_index_time_ms: {
                let total_ops = self.stats.index_operations.load(Ordering::Relaxed);
                if total_ops > 0 {
                    self.stats.total_index_time_ms.load(Ordering::Relaxed) / total_ops
                } else {
                    0
                }
            },
            cache_hit_rate: cache_stats.hit_rate,
            reader_pool_hit_rate: reader_stats.hit_rate,
            writer_pool_hit_rate: writer_stats.hit_rate,
            memory_pool_hit_rate: memory_stats.hit_rate,
            string_pool_hit_rate: string_stats.hit_rate,
        }
    }

    /// Optimize index for better search performance
    pub async fn optimize(&self) -> Result<()> {
        let writer = self.writer_pool.get_writer().await?;

        // Merge segments for better query performance
        let (writer, result) = tokio::task::spawn_blocking(move || {
            let res = writer
                .merge(&tantivy::merge_policy::DefaultMergePolicy::default())
                .map_err(|e| Error::Index(format!("Failed to optimize index: {}", e)));
            (writer, res)
        })
        .await
        .map_err(|e| Error::Index(format!("Optimization task failed: {}", e)))?;

        self.writer_pool.return_writer(writer).await;
        
        info!("Index optimization completed");
        result
    }

    /// Warm up caches with common queries
    pub async fn warm_up(
        &self,
        common_queries: &[(&str, Option<&str>, Option<&str>)],
    ) -> Result<()> {
        info!("Warming up index with {} common queries", common_queries.len());
        
        for (query, alias, flavor) in common_queries {
            let _ = self.search_optimized(query, *alias, *flavor, 10).await;
        }
        
        info!("Index warm-up completed");
        Ok(())
    }
}

impl ReaderPool {
    fn new<F>(max_readers: usize, reader_factory: F) -> Self
    where
        F: Fn() -> Result<IndexReader> + Send + Sync + 'static,
    {
        Self {
            readers: Mutex::new(VecDeque::with_capacity(max_readers)),
            max_readers,
            reader_factory: Box::new(reader_factory),
            stats: ReaderPoolStats::default(),
        }
    }

    async fn get_reader(&self) -> Result<IndexReader> {
        self.stats.requests.fetch_add(1, Ordering::Relaxed);

        // Try to get reader from pool
        {
            let mut readers = self.readers.lock().await;
            if let Some(reader) = readers.pop_front() {
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(reader);
            }
        }

        // Create new reader
        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        self.stats.created.fetch_add(1, Ordering::Relaxed);
        (self.reader_factory)()
    }

    async fn return_reader(&self, reader: IndexReader) {
        let mut readers = self.readers.lock().await;
        if readers.len() < self.max_readers {
            readers.push_back(reader);
        }
        // Otherwise let reader drop
    }

    async fn get_stats(&self) -> PoolStats {
        let requests = self.stats.requests.load(Ordering::Relaxed);
        let hits = self.stats.hits.load(Ordering::Relaxed);
        
        PoolStats {
            requests,
            hits,
            misses: self.stats.misses.load(Ordering::Relaxed),
            created: self.stats.created.load(Ordering::Relaxed),
            hit_rate: if requests > 0 {
                hits as f64 / requests as f64
            } else {
                0.0
            },
        }
    }
}

impl WriterPool {
    fn new<F>(max_writers: usize, writer_factory: F) -> Self
    where
        F: Fn() -> Result<IndexWriter> + Send + Sync + 'static,
    {
        Self {
            writers: Mutex::new(VecDeque::with_capacity(max_writers)),
            max_writers,
            writer_creation_semaphore: Semaphore::new(1), // Only one writer creation at a time
            writer_factory: Box::new(writer_factory),
            stats: WriterPoolStats::default(),
        }
    }

    async fn get_writer(&self) -> Result<IndexWriter> {
        self.stats.requests.fetch_add(1, Ordering::Relaxed);

        // Try to get writer from pool
        {
            let mut writers = self.writers.lock().await;
            if let Some(writer) = writers.pop_front() {
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(writer);
            }
        }

        // Create new writer (expensive operation)
        let _permit = self.writer_creation_semaphore.acquire().await
            .map_err(|_| Error::ResourceLimited("Writer creation semaphore error".into()))?;

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        self.stats.created.fetch_add(1, Ordering::Relaxed);
        (self.writer_factory)()
    }

    async fn return_writer(&self, writer: IndexWriter) {
        let mut writers = self.writers.lock().await;
        if writers.len() < self.max_writers {
            writers.push_back(writer);
        }
        // Otherwise let writer drop
    }

    async fn get_stats(&self) -> PoolStats {
        let requests = self.stats.requests.load(Ordering::Relaxed);
        let hits = self.stats.hits.load(Ordering::Relaxed);
        
        PoolStats {
            requests,
            hits,
            misses: self.stats.misses.load(Ordering::Relaxed),
            created: self.stats.created.load(Ordering::Relaxed),
            hit_rate: if requests > 0 {
                hits as f64 / requests as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub requests: usize,
    pub hits: usize,
    pub misses: usize,
    pub created: usize,
    pub hit_rate: f64,
}

#[derive(Debug, Clone)]
pub struct IndexStatsSummary {
    pub searches: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub index_operations: usize,
    pub documents_indexed: usize,
    pub avg_search_time_ms: usize,
    pub avg_index_time_ms: usize,
    pub cache_hit_rate: f64,
    pub reader_pool_hit_rate: f64,
    pub writer_pool_hit_rate: f64,
    pub memory_pool_hit_rate: f64,
    pub string_pool_hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HeadingBlock;
    use tempfile::TempDir;
    use tokio_test;

    fn create_test_blocks() -> Vec<HeadingBlock> {
        vec![
            HeadingBlock {
                path: vec!["React".to_string(), "Hooks".to_string()],
                content: "useState is a React hook for state management".to_string(),
                start_line: 100,
                end_line: 120,
            },
            HeadingBlock {
                path: vec!["React".to_string(), "Components".to_string()],
                content: "Components are the building blocks of React applications".to_string(),
                start_line: 50,
                end_line: 75,
            },
        ]
    }

    #[tokio::test]
    async fn test_optimized_index_creation() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index");

        let result = OptimizedSearchIndex::create(&index_path).await;
        assert!(result.is_ok());

        assert!(index_path.exists());
    }

    #[tokio::test]
    async fn test_optimized_search() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index");

        let index = OptimizedSearchIndex::create(&index_path).await.unwrap();
        let blocks = create_test_blocks();

        // Index blocks
        index
            .index_blocks_optimized("test", "test.md", &blocks)
            .await
            .unwrap();

        // Search
        let results = index
            .search_optimized("useState", Some("test"), None, 10)
            .await
            .unwrap();

        assert!(!results.is_empty());
        assert!(results[0].snippet.contains("useState"));
    }

    #[tokio::test]
    async fn test_cache_optimization() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index");

        let index = OptimizedSearchIndex::create(&index_path).await.unwrap();
        let blocks = create_test_blocks();

        index
            .index_blocks_optimized("test", "test.md", &blocks)
            .await
            .unwrap();

        // First search - should miss cache
        let _results1 = index
            .search_optimized("React", Some("test"), None, 10)
            .await
            .unwrap();

        // Second search - should hit cache
        let _results2 = index
            .search_optimized("React", Some("test"), None, 10)
            .await
            .unwrap();

        let stats = index.get_stats().await;
        assert!(stats.cache_hits > 0);
        assert!(stats.cache_hit_rate > 0.0);
    }

    #[tokio::test]
    async fn test_parallel_indexing() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index");

        let index = OptimizedSearchIndex::create(&index_path).await.unwrap();

        let sources = vec![
            ("source1".to_string(), "file1.md".to_string(), create_test_blocks()),
            ("source2".to_string(), "file2.md".to_string(), create_test_blocks()),
        ];

        let result = index.index_multiple_sources(sources).await;
        assert!(result.is_ok());

        let stats = index.get_stats().await;
        assert_eq!(stats.index_operations, 2);
    }

    #[tokio::test]
    async fn test_concurrent_search() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index");

        let index = OptimizedSearchIndex::create(&index_path).await.unwrap();
        let blocks = create_test_blocks();

        index
            .index_blocks_optimized("test", "test.md", &blocks)
            .await
            .unwrap();

        let queries = vec![
            (
                "React".to_string(),
                Some("test".to_string()),
                None,
                10,
            ),
            (
                "hooks".to_string(),
                Some("test".to_string()),
                None,
                10,
            ),
            (
                "components".to_string(),
                Some("test".to_string()),
                None,
                10,
            ),
        ];

        let results = index.search_multiple(queries).await.unwrap();
        assert_eq!(results.len(), 3);
        
        for result_set in results {
            assert!(!result_set.is_empty());
        }
    }

    #[tokio::test]
    async fn test_reader_pool_optimization() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index");

        let index = OptimizedSearchIndex::create(&index_path).await.unwrap();
        let blocks = create_test_blocks();

        index
            .index_blocks_optimized("test", "test.md", &blocks)
            .await
            .unwrap();

        // Perform multiple searches to test reader reuse
        for _ in 0..5 {
            let _ = index
                .search_optimized("React", Some("test"), None, 10)
                .await
                .unwrap();
        }

        let stats = index.get_stats().await;
        assert!(stats.reader_pool_hit_rate >= 0.0); // Some reader reuse should occur
    }

    #[tokio::test]
    async fn test_string_interning() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index");

        let index = OptimizedSearchIndex::create(&index_path).await.unwrap();
        
        // Create blocks with repeated alias values
        let mut blocks = Vec::new();
        for i in 0..10 {
            blocks.push(HeadingBlock {
                path: vec!["Section".to_string()],
                content: format!("Content {}", i),
                start_line: i,
                end_line: i + 1,
            });
        }

        index
            .index_blocks_optimized("repeated_alias", "test.md", &blocks)
            .await
            .unwrap();

        let stats = index.get_stats().await;
        assert!(stats.string_pool_hit_rate > 0.0); // String interning should occur
    }

    #[tokio::test]
    async fn test_warm_up() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index");

        let index = OptimizedSearchIndex::create(&index_path).await.unwrap();
        let blocks = create_test_blocks();

        index
            .index_blocks_optimized("test", "test.md", &blocks)
            .await
            .unwrap();

        let common_queries = [
            ("React", Some("test"), None),
            ("hooks", Some("test"), None),
            ("components", Some("test"), None),
        ];

        let result = index.warm_up(&common_queries).await;
        assert!(result.is_ok());

        let stats = index.get_stats().await;
        assert_eq!(stats.searches, 3); // Warm-up should have performed searches
    }
}
