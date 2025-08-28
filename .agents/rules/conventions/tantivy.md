# Tantivy-Specific Patterns

## Index Management

### Schema Design

**Well-Structured Schema Definition**
```rust
use tantivy::schema::{Schema, SchemaBuilder, TextOptions, TextFieldIndexing, IndexRecordOption};
use tantivy::{Index, IndexWriter, IndexReader, ReloadPolicy};
use std::path::Path;

/// Create a comprehensive schema for search documents
pub fn create_search_schema() -> Schema {
    let mut schema_builder = SchemaBuilder::default();
    
    // Title field - stored, indexed, with positions for phrase queries
    let title_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("en_stem")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions)
        )
        .set_stored()
        .set_fast();  // For aggregations and sorting
    
    let title_field = schema_builder.add_text_field("title", title_options);
    
    // Body field - indexed but not stored (too large), with positions
    let body_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("en_stem")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions)
        );
    // Note: not stored to save space, only indexed
    
    let body_field = schema_builder.add_text_field("body", body_options);
    
    // URL field - stored and indexed as keyword (exact matching)
    let url_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("keyword")
                .set_index_option(IndexRecordOption::Basic)
        )
        .set_stored();
        
    let url_field = schema_builder.add_text_field("url", url_options);
    
    // Tags field - multi-value text field for faceted search
    let tags_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("keyword")
                .set_index_option(IndexRecordOption::Basic)
        )
        .set_stored()
        .set_fast();  // For faceted search
        
    let tags_field = schema_builder.add_text_field("tags", tags_options);
    
    // Timestamp for sorting and filtering
    let timestamp_field = schema_builder.add_date_field(
        "timestamp", 
        tantivy::schema::DateOptions::default()
            .set_stored()
            .set_fast()  // For range queries and sorting
            .set_indexed()
    );
    
    // Content hash for deduplication
    let hash_field = schema_builder.add_bytes_field(
        "content_hash",
        tantivy::schema::BytesOptions::default()
            .set_stored()
            .set_indexed()
            .set_fast()
    );
    
    schema_builder.build()
}

/// Structured document for consistent field access
pub struct SearchDocument {
    pub title: String,
    pub body: String,
    pub url: String,
    pub tags: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub content_hash: [u8; 32], // SHA-256 hash
}

impl SearchDocument {
    /// Convert to Tantivy document with proper field mapping
    pub fn to_tantivy_doc(&self, schema: &Schema) -> tantivy::Document {
        let mut doc = tantivy::Document::default();
        
        // Get field handles
        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();
        let url_field = schema.get_field("url").unwrap();
        let tags_field = schema.get_field("tags").unwrap();
        let timestamp_field = schema.get_field("timestamp").unwrap();
        let hash_field = schema.get_field("content_hash").unwrap();
        
        // Add field values
        doc.add_text(title_field, &self.title);
        doc.add_text(body_field, &self.body);
        doc.add_text(url_field, &self.url);
        
        // Add multiple tags
        for tag in &self.tags {
            doc.add_text(tags_field, tag);
        }
        
        doc.add_date(timestamp_field, tantivy::DateTime::from_utc(self.timestamp));
        doc.add_bytes(hash_field, &self.content_hash[..]);
        
        doc
    }
}
```

### Index Creation and Configuration

**Production-Ready Index Setup**
```rust
use tantivy::{Index, IndexSettings, IndexSortByField, Order};
use std::path::Path;
use std::num::NonZeroUsize;

/// Index configuration for optimal search performance
pub struct IndexConfig {
    pub path: std::path::PathBuf,
    pub writer_memory_mb: usize,
    pub sort_by_timestamp: bool,
    pub enable_compression: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            path: std::path::PathBuf::from("./search_index"),
            writer_memory_mb: 100,  // 100MB writer heap
            sort_by_timestamp: true,
            enable_compression: true,
        }
    }
}

/// Create or open a Tantivy index with optimized settings
pub fn create_or_open_index(config: &IndexConfig) -> tantivy::Result<Index> {
    let schema = create_search_schema();
    
    // Try to open existing index first
    if config.path.exists() {
        match Index::open_in_dir(&config.path) {
            Ok(index) => {
                // Verify schema compatibility
                if schemas_compatible(&schema, index.schema()) {
                    return Ok(index);
                } else {
                    return Err(tantivy::TantivyError::SchemaError(
                        "Schema mismatch with existing index".to_string()
                    ));
                }
            }
            Err(_) => {
                // Index exists but corrupted, fall through to create new
            }
        }
    }
    
    // Create new index with optimized settings
    let mut index_builder = Index::builder().schema(schema);
    
    // Configure index settings
    let mut settings = IndexSettings::default();
    
    if config.sort_by_timestamp {
        let timestamp_field = index_builder.schema().get_field("timestamp").unwrap();
        settings = settings.sort_by_field(
            IndexSortByField {
                field: timestamp_field,
                order: Order::Desc,  // Newest first
            }
        );
    }
    
    if config.enable_compression {
        settings = settings.docstore_compression(
            tantivy::store::Compressor::Lz4
        );
    }
    
    // Set up index in directory
    let index = index_builder
        .settings(settings)
        .create_in_dir(&config.path)?;
    
    Ok(index)
}

fn schemas_compatible(new_schema: &tantivy::schema::Schema, existing_schema: &tantivy::schema::Schema) -> bool {
    // Simplified compatibility check - in production, implement full schema evolution
    new_schema.to_json() == existing_schema.to_json()
}
```

### Writer Management

**Efficient Document Indexing**
```rust
use tantivy::{IndexWriter, Opstamp};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use std::time::Duration;

/// Thread-safe index writer with batching and error recovery
pub struct ManagedIndexWriter {
    writer: Arc<Mutex<IndexWriter>>,
    schema: tantivy::schema::Schema,
    batch_size: usize,
    pending_docs: Arc<RwLock<Vec<tantivy::Document>>>,
    auto_commit_interval: Duration,
}

impl ManagedIndexWriter {
    pub fn new(
        index: &tantivy::Index, 
        writer_memory_mb: usize,
        batch_size: usize,
    ) -> tantivy::Result<Self> {
        let writer = index.writer(writer_memory_mb * 1024 * 1024)?;  // Convert MB to bytes
        
        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            schema: index.schema(),
            batch_size,
            pending_docs: Arc::new(RwLock::new(Vec::new())),
            auto_commit_interval: Duration::from_secs(60),  // Auto-commit every minute
        })
    }
    
    /// Add a document to the index with batching
    pub async fn add_document(&self, doc: SearchDocument) -> tantivy::Result<()> {
        let tantivy_doc = doc.to_tantivy_doc(&self.schema);
        
        // Add to pending batch
        {
            let mut pending = self.pending_docs.write().await;
            pending.push(tantivy_doc);
            
            // Check if batch is ready
            if pending.len() >= self.batch_size {
                let batch = std::mem::take(&mut *pending);
                drop(pending);  // Release lock early
                
                // Process batch
                self.flush_batch(batch).await?;
            }
        }
        
        Ok(())
    }
    
    /// Update document by first deleting, then adding
    pub async fn update_document(&self, url: &str, doc: SearchDocument) -> tantivy::Result<()> {
        // Delete existing document by URL
        self.delete_document_by_url(url).await?;
        
        // Add new version
        self.add_document(doc).await?;
        
        Ok(())
    }
    
    /// Delete document by URL
    pub async fn delete_document_by_url(&self, url: &str) -> tantivy::Result<()> {
        let url_field = self.schema.get_field("url").unwrap();
        let term = tantivy::Term::from_field_text(url_field, url);
        
        let mut writer = self.writer.lock().await;
        writer.delete_term(term);
        
        Ok(())
    }
    
    async fn flush_batch(&self, documents: Vec<tantivy::Document>) -> tantivy::Result<()> {
        let mut writer = self.writer.lock().await;
        
        for doc in documents {
            writer.add_document(doc)?;
        }
        
        Ok(())
    }
    
    /// Commit all pending changes
    pub async fn commit(&self) -> tantivy::Result<Opstamp> {
        // Flush any pending documents first
        {
            let mut pending = self.pending_docs.write().await;
            if !pending.is_empty() {
                let batch = std::mem::take(&mut *pending);
                drop(pending);
                
                self.flush_batch(batch).await?;
            }
        }
        
        // Commit to disk
        let mut writer = self.writer.lock().await;
        let opstamp = writer.commit()?;
        
        Ok(opstamp)
    }
    
    /// Start auto-commit background task
    pub async fn start_auto_commit(self: Arc<Self>) {
        let writer = Arc::clone(&self);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(writer.auto_commit_interval);
            
            loop {
                interval.tick().await;
                
                if let Err(e) = writer.commit().await {
                    tracing::error!("Auto-commit failed: {}", e);
                }
            }
        });
    }
}
```

## Query Construction

### Query Builder Pattern

**Type-Safe Query Construction**
```rust
use tantivy::query::{Query, BooleanQuery, TermQuery, PhraseQuery, FuzzyTermQuery, RangeQuery, Occur};
use tantivy::{Term, Score};
use std::ops::Bound;

/// Type-safe query builder for complex search queries
pub struct SearchQueryBuilder {
    schema: tantivy::schema::Schema,
    query_parts: Vec<(Box<dyn Query>, Occur)>,
    boost_map: std::collections::HashMap<String, Score>,
}

impl SearchQueryBuilder {
    pub fn new(schema: tantivy::schema::Schema) -> Self {
        Self {
            schema,
            query_parts: Vec::new(),
            boost_map: std::collections::HashMap::new(),
        }
    }
    
    /// Add a term query for a specific field
    pub fn term<S: AsRef<str>>(mut self, field_name: &str, term: S) -> Self {
        if let Ok(field) = self.schema.get_field(field_name) {
            let term = Term::from_field_text(field, term.as_ref());
            let query: Box<dyn Query> = Box::new(TermQuery::new(term, tantivy::schema::IndexRecordOption::Basic));
            
            // Apply boost if configured
            let query = if let Some(&boost) = self.boost_map.get(field_name) {
                Box::new(tantivy::query::BoostQuery::new(query, boost))
            } else {
                query
            };
            
            self.query_parts.push((query, Occur::Must));
        }
        self
    }
    
    /// Add a phrase query for exact phrase matching
    pub fn phrase<S: AsRef<str>>(mut self, field_name: &str, phrase: S) -> Self {
        if let Ok(field) = self.schema.get_field(field_name) {
            let terms: Vec<Term> = phrase
                .as_ref()
                .split_whitespace()
                .map(|word| Term::from_field_text(field, word))
                .collect();
            
            if !terms.is_empty() {
                let query: Box<dyn Query> = Box::new(PhraseQuery::new(terms));
                self.query_parts.push((query, Occur::Must));
            }
        }
        self
    }
    
    /// Add a fuzzy term query for typo tolerance
    pub fn fuzzy<S: AsRef<str>>(mut self, field_name: &str, term: S, distance: u8) -> Self {
        if let Ok(field) = self.schema.get_field(field_name) {
            let term = Term::from_field_text(field, term.as_ref());
            let query: Box<dyn Query> = Box::new(FuzzyTermQuery::new(term, distance, true));
            self.query_parts.push((query, Occur::Should));  // Fuzzy queries are optional
        }
        self
    }
    
    /// Add a date range query
    pub fn date_range(
        mut self, 
        field_name: &str, 
        start: Option<chrono::DateTime<chrono::Utc>>,
        end: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        if let Ok(field) = self.schema.get_field(field_name) {
            let start_bound = start
                .map(|dt| Bound::Included(tantivy::DateTime::from_utc(dt)))
                .unwrap_or(Bound::Unbounded);
            let end_bound = end
                .map(|dt| Bound::Included(tantivy::DateTime::from_utc(dt)))
                .unwrap_or(Bound::Unbounded);
                
            let query: Box<dyn Query> = Box::new(
                RangeQuery::new_date_bounds(field, start_bound, end_bound)
            );
            self.query_parts.push((query, Occur::Must));
        }
        self
    }
    
    /// Add field-specific boost for relevance tuning
    pub fn boost_field(mut self, field_name: &str, boost: Score) -> Self {
        self.boost_map.insert(field_name.to_string(), boost);
        self
    }
    
    /// Add an optional (should) clause
    pub fn should(mut self, query: Box<dyn Query>) -> Self {
        self.query_parts.push((query, Occur::Should));
        self
    }
    
    /// Add a required (must) clause  
    pub fn must(mut self, query: Box<dyn Query>) -> Self {
        self.query_parts.push((query, Occur::Must));
        self
    }
    
    /// Add a negation (must_not) clause
    pub fn must_not(mut self, query: Box<dyn Query>) -> Self {
        self.query_parts.push((query, Occur::MustNot));
        self
    }
    
    /// Build the final boolean query
    pub fn build(self) -> Box<dyn Query> {
        if self.query_parts.is_empty() {
            // Return match-all query
            Box::new(tantivy::query::AllQuery)
        } else if self.query_parts.len() == 1 {
            // Single query, no need for boolean wrapper
            self.query_parts.into_iter().next().unwrap().0
        } else {
            // Multiple queries, wrap in boolean
            Box::new(BooleanQuery::from(self.query_parts))
        }
    }
}

// Usage example
pub fn build_complex_search_query(
    schema: &tantivy::schema::Schema,
    query_text: &str,
    tags: &[String],
    date_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
) -> Box<dyn Query> {
    let mut builder = SearchQueryBuilder::new(schema.clone())
        .boost_field("title", 2.0)    // Title matches are twice as important
        .boost_field("tags", 1.5);    // Tag matches get moderate boost
    
    // Add main search terms to title and body
    if !query_text.trim().is_empty() {
        // Search in title (exact and fuzzy)
        builder = builder
            .term("title", query_text)
            .fuzzy("title", query_text, 2);
        
        // Search in body  
        builder = builder.term("body", query_text);
    }
    
    // Add tag filters
    for tag in tags {
        builder = builder.term("tags", tag);
    }
    
    // Add date range if specified
    if let Some((start, end)) = date_range {
        builder = builder.date_range("timestamp", Some(start), Some(end));
    }
    
    builder.build()
}
```

### Advanced Query Patterns

**Custom Scoring and Relevance**
```rust
use tantivy::query::{Explanation, Scorer};
use tantivy::{DocId, Score, SegmentReader};

/// Custom scorer that combines multiple relevance signals
pub struct CustomRelevanceScorer {
    title_scorer: Box<dyn Scorer>,
    body_scorer: Box<dyn Scorer>,
    recency_weight: Score,
}

impl CustomRelevanceScorer {
    pub fn new(
        title_scorer: Box<dyn Scorer>,
        body_scorer: Box<dyn Scorer>,
        recency_weight: Score,
    ) -> Self {
        Self {
            title_scorer,
            body_scorer,
            recency_weight,
        }
    }
}

impl Scorer for CustomRelevanceScorer {
    fn score(&mut self) -> Score {
        let title_score = self.title_scorer.score() * 2.0;  // Title boost
        let body_score = self.body_scorer.score() * 1.0;   // Base body score
        
        // TODO: Add recency scoring based on document timestamp
        let recency_score = self.calculate_recency_score();
        
        title_score + body_score + (recency_score * self.recency_weight)
    }
    
    fn doc(&self) -> DocId {
        self.title_scorer.doc()
    }
    
    fn advance(&mut self) -> DocId {
        let title_doc = self.title_scorer.advance();
        let body_doc = self.body_scorer.advance();
        
        // Return the minimum doc id (both scorers should advance together)
        std::cmp::min(title_doc, body_doc)
    }
    
    fn explain(&self, doc: DocId) -> tantivy::Result<Explanation> {
        let title_explanation = self.title_scorer.explain(doc)?;
        let body_explanation = self.body_scorer.explain(doc)?;
        
        let explanation = Explanation::new(
            "Custom relevance score",
            self.score(),
        )
        .add_detail(title_explanation)
        .add_detail(body_explanation);
        
        Ok(explanation)
    }
}

impl CustomRelevanceScorer {
    fn calculate_recency_score(&self) -> Score {
        // TODO: Implement recency scoring
        // This would typically involve:
        // 1. Getting document timestamp from fast field
        // 2. Calculating time difference from now
        // 3. Converting to a score (newer = higher score)
        1.0
    }
}
```

## Search Execution

### Searcher Management

**Efficient Search Operations**
```rust
use tantivy::{IndexReader, ReloadPolicy, Searcher, LeasedItem};
use tantivy::collector::{TopDocs, Count};
use tantivy::query::Query;
use std::sync::Arc;

/// High-level search interface with connection pooling
pub struct SearchManager {
    reader: IndexReader,
    default_limit: usize,
}

impl SearchManager {
    pub fn new(index: &tantivy::Index) -> tantivy::Result<Self> {
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)  // Auto-reload on commits
            .try_into()?;
            
        Ok(Self {
            reader,
            default_limit: 10,
        })
    }
    
    /// Execute search with comprehensive result information
    pub async fn search(
        &self,
        query: &dyn Query,
        limit: Option<usize>,
        offset: usize,
    ) -> tantivy::Result<SearchResults> {
        let searcher = self.reader.searcher();
        let limit = limit.unwrap_or(self.default_limit);
        
        // Execute search with timing
        let start_time = std::time::Instant::now();
        
        // Get total count and top documents
        let (total_count, top_docs) = tokio::task::spawn_blocking({
            let query = query.box_clone();
            let searcher = searcher.clone();
            move || -> tantivy::Result<(usize, Vec<(tantivy::Score, tantivy::DocAddress)>)> {
                // Count total matches
                let count = searcher.search(&*query, &Count)?;
                
                // Get top documents with offset
                let top_collector = TopDocs::with_limit(limit + offset);
                let top_docs = searcher.search(&*query, &top_collector)?;
                
                // Apply offset
                let top_docs = top_docs.into_iter().skip(offset).collect();
                
                Ok((count, top_docs))
            }
        }).await.unwrap()?;
        
        let search_duration = start_time.elapsed();
        
        // Convert to search hits
        let hits = self.convert_to_search_hits(&searcher, top_docs).await?;
        
        Ok(SearchResults {
            hits,
            total_count,
            execution_time: search_duration,
            query_info: QueryInfo {
                offset,
                limit,
                has_more: total_count > offset + hits.len(),
            },
        })
    }
    
    async fn convert_to_search_hits(
        &self,
        searcher: &Searcher,
        doc_addresses: Vec<(tantivy::Score, tantivy::DocAddress)>,
    ) -> tantivy::Result<Vec<SearchHit>> {
        let schema = searcher.schema();
        let title_field = schema.get_field("title").unwrap();
        let url_field = schema.get_field("url").unwrap();
        let timestamp_field = schema.get_field("timestamp").unwrap();
        
        let mut hits = Vec::new();
        
        for (score, doc_address) in doc_addresses {
            let doc = searcher.doc(doc_address)?;
            
            // Extract stored fields
            let title = doc
                .get_first(title_field)
                .and_then(|v| v.as_text())
                .unwrap_or("Untitled")
                .to_string();
            
            let url = doc
                .get_first(url_field)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();
            
            let timestamp = doc
                .get_first(timestamp_field)
                .and_then(|v| v.as_date())
                .map(|dt| dt.into_utc())
                .unwrap_or_else(|| chrono::Utc::now());
            
            // Generate snippet (simplified - in production use proper snippet generation)
            let snippet = self.generate_snippet(&title, 150);
            
            hits.push(SearchHit {
                title,
                snippet,
                url,
                score,
                timestamp,
                doc_address,
            });
        }
        
        Ok(hits)
    }
    
    fn generate_snippet(&self, content: &str, max_length: usize) -> String {
        if content.len() <= max_length {
            content.to_string()
        } else {
            let truncated = &content[..max_length];
            if let Some(last_space) = truncated.rfind(' ') {
                format!("{}...", &truncated[..last_space])
            } else {
                format!("{}...", truncated)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub total_count: usize,
    pub execution_time: std::time::Duration,
    pub query_info: QueryInfo,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub title: String,
    pub snippet: String,
    pub url: String,
    pub score: tantivy::Score,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub doc_address: tantivy::DocAddress,
}

#[derive(Debug, Clone)]
pub struct QueryInfo {
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}
```

### Faceted Search

**Category and Filter Support**
```rust
use tantivy::aggregation::agg_req::{Aggregations, BucketAggregationType, TermsAggregation};
use tantivy::aggregation::AggregationCollector;
use std::collections::HashMap;

/// Faceted search with category aggregations
pub struct FacetedSearchManager {
    search_manager: SearchManager,
    facet_fields: Vec<String>,
}

impl FacetedSearchManager {
    pub fn new(index: &tantivy::Index, facet_fields: Vec<String>) -> tantivy::Result<Self> {
        Ok(Self {
            search_manager: SearchManager::new(index)?,
            facet_fields,
        })
    }
    
    /// Execute search with facet aggregations
    pub async fn search_with_facets(
        &self,
        query: &dyn Query,
        limit: Option<usize>,
        offset: usize,
    ) -> tantivy::Result<FacetedSearchResults> {
        let searcher = self.search_manager.reader.searcher();
        
        // Build aggregation request for facets
        let mut agg_req = Aggregations::default();
        let schema = searcher.schema();
        
        for facet_field in &self.facet_fields {
            if let Ok(field) = schema.get_field(facet_field) {
                agg_req.add_bucket(
                    facet_field.clone(),
                    BucketAggregationType::Terms(TermsAggregation {
                        field: field.field_name().to_string(),
                        size: Some(20),  // Top 20 facet values
                        ..Default::default()
                    }),
                );
            }
        }
        
        // Execute search with facets
        let start_time = std::time::Instant::now();
        
        let (search_results, facet_results) = tokio::task::spawn_blocking({
            let query = query.box_clone();
            let searcher = searcher.clone();
            let limit = limit.unwrap_or(10);
            
            move || -> tantivy::Result<(SearchResults, HashMap<String, Vec<FacetValue>>)> {
                // Regular search
                let search_results = tokio::runtime::Handle::current()
                    .block_on(async {
                        SearchManager::search(&search_manager, &*query, Some(limit), offset).await
                    })?;
                
                // Facet aggregation
                let agg_collector = AggregationCollector::from_aggs(agg_req);
                let agg_results = searcher.search(&*query, &agg_collector)?;
                
                // Parse facet results
                let facet_results = parse_facet_results(agg_results);
                
                Ok((search_results, facet_results))
            }
        }).await.unwrap()?;
        
        let total_duration = start_time.elapsed();
        
        Ok(FacetedSearchResults {
            search_results,
            facets: facet_results,
            execution_time: total_duration,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FacetedSearchResults {
    pub search_results: SearchResults,
    pub facets: HashMap<String, Vec<FacetValue>>,
    pub execution_time: std::time::Duration,
}

#[derive(Debug, Clone)]
pub struct FacetValue {
    pub value: String,
    pub count: u64,
}

fn parse_facet_results(
    agg_results: tantivy::aggregation::AggregationResults,
) -> HashMap<String, Vec<FacetValue>> {
    let mut facets = HashMap::new();
    
    for (facet_name, agg_result) in agg_results {
        if let Some(bucket_result) = agg_result.as_bucket() {
            if let Some(terms) = bucket_result.as_terms() {
                let facet_values = terms
                    .buckets
                    .iter()
                    .map(|bucket| FacetValue {
                        value: bucket.key.as_str().unwrap_or("").to_string(),
                        count: bucket.doc_count,
                    })
                    .collect();
                
                facets.insert(facet_name, facet_values);
            }
        }
    }
    
    facets
}
```

## Performance Optimization

### Index Optimization

**Merge Policy and Segment Management**
```rust
use tantivy::{MergePolicy, LogMergePolicy};

/// Configure optimal merge policy for search performance
pub fn configure_merge_policy() -> LogMergePolicy {
    LogMergePolicy::default()
        .set_merge_factor(10)           // Merge 10 segments at once
        .set_max_merge_size(5_000_000)  // Max 5MB segments for faster merges
        .set_max_docs_before_merge(10_000)  // Merge after 10k documents
}

/// Index maintenance operations
pub struct IndexMaintenance {
    writer: Arc<Mutex<tantivy::IndexWriter>>,
}

impl IndexMaintenance {
    /// Optimize index by merging segments
    pub async fn optimize(&self) -> tantivy::Result<()> {
        let mut writer = self.writer.lock().await;
        
        // Merge all segments into one for optimal read performance
        writer.merge(&[]).wait()?;
        
        Ok(())
    }
    
    /// Garbage collect deleted documents
    pub async fn garbage_collect(&self) -> tantivy::Result<()> {
        let mut writer = self.writer.lock().await;
        
        // This will remove deleted documents and reclaim space
        writer.garbage_collect_files()?;
        
        Ok(())
    }
}
```

### Memory Management

**Fast Fields and Caching**
```rust
use tantivy::fastfield::{FastFieldReader, FastFieldReaders};
use std::collections::HashMap;

/// Cache for fast field readers to avoid repeated lookups
pub struct FastFieldCache {
    readers: HashMap<String, Box<dyn FastFieldReader<u64>>>,
    schema: tantivy::schema::Schema,
}

impl FastFieldCache {
    pub fn new(searcher: &tantivy::Searcher) -> Self {
        Self {
            readers: HashMap::new(),
            schema: searcher.schema(),
        }
    }
    
    /// Get or create a fast field reader for efficient sorting/filtering
    pub fn get_u64_reader(&mut self, field_name: &str, searcher: &tantivy::Searcher) 
        -> tantivy::Result<&dyn FastFieldReader<u64>> 
    {
        if !self.readers.contains_key(field_name) {
            let field = self.schema.get_field(field_name)?;
            let reader = searcher.segment_readers()[0]  // Simplified - handle multiple segments
                .fast_fields()
                .u64(field)?;
            self.readers.insert(field_name.to_string(), reader);
        }
        
        Ok(self.readers.get(field_name).unwrap().as_ref())
    }
}
```

## Error Handling

### Tantivy-Specific Error Patterns

**Comprehensive Error Handling**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    
    #[error("Query parsing failed: {query} - {reason}")]
    QueryParsing { query: String, reason: String },
    
    #[error("Index not found: {path}")]
    IndexNotFound { path: std::path::PathBuf },
    
    #[error("Schema validation failed: {reason}")]
    SchemaValidation { reason: String },
    
    #[error("Search timeout after {timeout:?}")]
    SearchTimeout { timeout: std::time::Duration },
}

/// Convert Tantivy errors to application errors with context
impl From<tantivy::query::QueryParserError> for SearchError {
    fn from(err: tantivy::query::QueryParserError) -> Self {
        SearchError::QueryParsing {
            query: "unknown".to_string(),
            reason: err.to_string(),
        }
    }
}
```

Remember: Tantivy is a powerful but complex search engine. Focus on understanding its core concepts (schema, indexing, querying) and build abstractions that make it easier to use correctly while maintaining performance.