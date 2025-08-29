# Performance Optimization

## Performance Philosophy

Performance optimization in Rust follows the principle: **Measure First, Optimize with Data**. Leverage Rust's zero-cost abstractions and only apply optimizations where profiling shows they provide measurable benefit (ROI > 2x).

## Measurement Strategy

### Profiling Tools

**CPU Profiling**

```bash
# Install profiling tools
cargo install flamegraph
cargo install cargo-instruments # macOS only

# Profile CPU usage with flamegraph
cargo build --release
flamegraph --bin cache-cli -- search "rust programming" --limit 1000

# Profile with perf (Linux)
perf record --call-graph=dwarf ./target/release/cache-cli search "rust programming"
perf report

# Profile with Instruments (macOS)
cargo instruments -t "Time Profiler" --bin cache-cli -- search "rust programming"
```

**Memory Profiling**

```bash
# Install memory profiling tools
cargo install dhat

# Memory profiling with dhat
DHAT_HEAP_HISTORY=1 cargo run --bin cache-cli -- search "rust programming"

# Valgrind (Linux)
valgrind --tool=massif --stacks=yes ./target/release/cache-cli search "rust"
ms_print massif.out.* | head -30
```

**Benchmarking Framework**

```rust
// benches/performance.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use blz_core::*;

fn search_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create test data once
    let (cache, test_queries) = rt.block_on(async {
        let mut cache = create_large_test_blz().await.unwrap();
        populate_test_data(&mut cache, 100_000).await.unwrap();

        let queries = vec![
            ("simple", "rust"),
            ("field", "title:programming"),
            ("boolean", "rust AND programming"),
            ("phrase", "\"rust programming language\""),
            ("wildcard", "program*"),
            ("complex", "(title:rust OR body:language) AND NOT deprecated:true"),
        ];

        (cache, queries)
    });

    let mut group = c.benchmark_group("search_performance");
    group.throughput(Throughput::Elements(1));

    for (name, query) in test_queries {
        // Cold cache performance
        group.bench_with_input(
            BenchmarkId::new("cold_blz", name),
            &query,
            |b, &query| {
                b.to_async(&rt).iter(|| async {
                    cache.clear_blz().await;
                    black_box(cache.search(query, 10).await.unwrap())
                })
            },
        );

        // Warm cache performance
        group.bench_with_input(
            BenchmarkId::new("warm_blz", name),
            &query,
            |b, &query| {
                // Warm up cache
                rt.block_on(async { cache.search(query, 10).await.unwrap() });

                b.to_async(&rt).iter(|| async {
                    black_box(cache.search(query, 10).await.unwrap())
                })
            },
        );
    }

    group.finish();
}

fn indexing_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("indexing_performance");

    // Benchmark different document sizes
    let sizes = [1_000, 10_000, 100_000];

    for size in sizes {
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::new("add_document", size),
            &size,
            |b, &size| {
                b.to_async(&rt).iter_with_setup(
                    || {
                        let cache = rt.block_on(create_test_blz()).unwrap();
                        let doc = generate_document(size);
                        (cache, doc)
                    },
                    |(mut cache, doc)| async move {
                        black_box(cache.add_document(doc).await.unwrap())
                    }
                )
            },
        );
    }

    group.finish();
}

criterion_group!(benches, search_performance, indexing_performance);
criterion_main!(benches);
```

### Performance Monitoring

**Runtime Metrics Collection**

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Performance metrics collector
#[derive(Debug)]
pub struct PerformanceMetrics {
    // Counters
    pub search_count: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub index_operations: AtomicU64,

    // Timing histograms (simplified - use proper histogram in production)
    pub search_latency_sum: AtomicU64,
    pub search_latency_count: AtomicU64,
    pub index_latency_sum: AtomicU64,
    pub index_latency_count: AtomicU64,

    // Memory tracking
    pub memory_usage_bytes: AtomicU64,
    pub peak_memory_bytes: AtomicU64,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            search_count: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            index_operations: AtomicU64::new(0),
            search_latency_sum: AtomicU64::new(0),
            search_latency_count: AtomicU64::new(0),
            index_latency_sum: AtomicU64::new(0),
            index_latency_count: AtomicU64::new(0),
            memory_usage_bytes: AtomicU64::new(0),
            peak_memory_bytes: AtomicU64::new(0),
        }
    }

    pub fn record_search(&self, duration: Duration, cache_hit: bool) {
        self.search_count.fetch_add(1, Ordering::Relaxed);
        self.search_latency_sum.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        self.search_latency_count.fetch_add(1, Ordering::Relaxed);

        if cache_hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_memory_usage(&self, bytes: u64) {
        self.memory_usage_bytes.store(bytes, Ordering::Relaxed);

        // Update peak memory
        let mut current_peak = self.peak_memory_bytes.load(Ordering::Relaxed);
        while bytes > current_peak {
            match self.peak_memory_bytes.compare_exchange_weak(
                current_peak,
                bytes,
                Ordering::Relaxed,
                Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => current_peak = x,
            }
        }
    }

    pub fn get_stats(&self) -> PerformanceStats {
        let search_count = self.search_count.load(Ordering::Relaxed);
        let search_latency_sum = self.search_latency_sum.load(Ordering::Relaxed);
        let search_latency_count = self.search_latency_count.load(Ordering::Relaxed);

        let avg_search_latency = if search_latency_count > 0 {
            Duration::from_micros(search_latency_sum / search_latency_count)
        } else {
            Duration::ZERO
        };

        let cache_hit_rate = if search_count > 0 {
            self.cache_hits.load(Ordering::Relaxed) as f64 / search_count as f64
        } else {
            0.0
        };

        PerformanceStats {
            total_searches: search_count,
            average_search_latency: avg_search_latency,
            cache_hit_rate,
            current_memory_mb: self.memory_usage_bytes.load(Ordering::Relaxed) / (1024 * 1024),
            peak_memory_mb: self.peak_memory_bytes.load(Ordering::Relaxed) / (1024 * 1024),
        }
    }
}

#[derive(Debug)]
pub struct PerformanceStats {
    pub total_searches: u64,
    pub average_search_latency: Duration,
    pub cache_hit_rate: f64,
    pub current_memory_mb: u64,
    pub peak_memory_mb: u64,
}
```

## Zero-Copy Optimization

### String and Buffer Management

**Avoid Unnecessary Allocations**

```rust
use std::borrow::Cow;
use std::ops::Range;

/// Zero-copy document parser
pub struct DocumentParser<'a> {
    content: &'a str,
    current_pos: usize,
}

impl<'a> DocumentParser<'a> {
    pub fn new(content: &'a str) -> Self {
        Self { content, current_pos: 0 }
    }

    /// Extract title without allocating new string
    pub fn extract_title(&mut self) -> Option<&'a str> {
        let start = self.find_pattern("# ")?;
        let end = self.find_line_end(start)?;

        Some(&self.content[start + 2..end])
    }

    /// Extract body sections as string slices
    pub fn extract_sections(&mut self) -> Vec<Section<'a>> {
        let mut sections = Vec::new();

        while let Some(section) = self.next_section() {
            sections.push(section);
        }

        sections
    }

    fn next_section(&mut self) -> Option<Section<'a>> {
        let start = self.find_pattern("## ")?;
        let title_end = self.find_line_end(start)?;
        let content_start = title_end + 1;
        let content_end = self.find_next_section_or_end(content_start);

        let section = Section {
            title: &self.content[start + 3..title_end],
            content: &self.content[content_start..content_end],
            range: start..content_end,
        };

        self.current_pos = content_end;
        Some(section)
    }
}

#[derive(Debug)]
pub struct Section<'a> {
    pub title: &'a str,
    pub content: &'a str,
    pub range: Range<usize>,
}

/// Zero-copy query result
pub struct SearchHit<'a> {
    pub document_id: u64,
    pub title: Cow<'a, str>,
    pub snippet: &'a str,
    pub score: f32,
}

impl<'a> SearchHit<'a> {
    /// Create search hit without copying strings when possible
    pub fn new_borrowed(
        document_id: u64,
        title: &'a str,
        snippet: &'a str,
        score: f32,
    ) -> Self {
        Self {
            document_id,
            title: Cow::Borrowed(title),
            snippet,
            score,
        }
    }

    /// Create search hit with highlighting (requires allocation)
    pub fn new_highlighted(
        document_id: u64,
        title: String,
        snippet: &'a str,
        score: f32,
    ) -> Self {
        Self {
            document_id,
            title: Cow::Owned(title),
            snippet,
            score,
        }
    }
}
```

### Memory Pool Pattern

**Reuse Allocations**

```rust
use std::collections::VecDeque;
use std::sync::Mutex;

/// Memory pool for reusing buffers
pub struct BufferPool {
    small_buffers: Mutex<VecDeque<Vec<u8>>>,   // < 1KB
    medium_buffers: Mutex<VecDeque<Vec<u8>>>,  // 1-64KB
    large_buffers: Mutex<VecDeque<Vec<u8>>>,   // > 64KB
}

impl BufferPool {
    pub fn new() -> Self {
        Self {
            small_buffers: Mutex::new(VecDeque::new()),
            medium_buffers: Mutex::new(VecDeque::new()),
            large_buffers: Mutex::new(VecDeque::new()),
        }
    }

    /// Get a buffer of at least the specified size
    pub fn get_buffer(&self, min_size: usize) -> PooledBuffer {
        let mut buffer = if min_size <= 1024 {
            self.small_buffers.lock().unwrap().pop_front()
        } else if min_size <= 65536 {
            self.medium_buffers.lock().unwrap().pop_front()
        } else {
            self.large_buffers.lock().unwrap().pop_front()
        };

        if let Some(ref mut buf) = buffer {
            if buf.capacity() < min_size {
                buf.reserve(min_size - buf.capacity());
            }
            buf.clear();
        } else {
            buffer = Some(Vec::with_capacity(min_size));
        }

        PooledBuffer {
            buffer: buffer.unwrap(),
            pool: self,
        }
    }
}

/// RAII wrapper that returns buffer to pool on drop
pub struct PooledBuffer<'a> {
    buffer: Vec<u8>,
    pool: &'a BufferPool,
}

impl<'a> PooledBuffer<'a> {
    pub fn as_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }
}

impl Drop for PooledBuffer<'_> {
    fn drop(&mut self) {
        // Return buffer to appropriate pool
        let capacity = self.buffer.capacity();

        if capacity <= 1024 {
            let mut pool = self.pool.small_buffers.lock().unwrap();
            if pool.len() < 10 { // Limit pool size
                pool.push_back(std::mem::take(&mut self.buffer));
            }
        } else if capacity <= 65536 {
            let mut pool = self.pool.medium_buffers.lock().unwrap();
            if pool.len() < 5 {
                pool.push_back(std::mem::take(&mut self.buffer));
            }
        } else {
            let mut pool = self.pool.large_buffers.lock().unwrap();
            if pool.len() < 2 {
                pool.push_back(std::mem::take(&mut self.buffer));
            }
        }
    }
}

// Usage in search operations
impl SearchIndex {
    pub async fn search_with_pooled_buffers(
        &self,
        query: &str,
        buffer_pool: &BufferPool,
    ) -> CacheResult<SearchResults> {
        let mut result_buffer = buffer_pool.get_buffer(8192);
        let mut temp_buffer = buffer_pool.get_buffer(4096);

        // Use pooled buffers for intermediate results
        self.execute_search_with_buffers(query, result_buffer.as_mut(), temp_buffer.as_mut()).await
    }
}
```

## Caching Strategies

### Multi-Level Caching

**LRU Cache with Size Limits**

```rust
use std::collections::HashMap;
use std::hash::Hash;
use std::ptr::NonNull;

/// Node in the LRU linked list
struct LruNode<K, V> {
    key: K,
    value: V,
    prev: Option<NonNull<LruNode<K, V>>>,
    next: Option<NonNull<LruNode<K, V>>>,
}

/// LRU cache with size and memory limits
pub struct LruCache<K, V> {
    map: HashMap<K, NonNull<LruNode<K, V>>>,
    head: Option<NonNull<LruNode<K, V>>>,
    tail: Option<NonNull<LruNode<K, V>>>,
    capacity: usize,
    max_memory_bytes: usize,
    current_memory_bytes: usize,
    size_fn: fn(&V) -> usize,
}

impl<K: Hash + Eq + Clone, V> LruCache<K, V> {
    pub fn new(capacity: usize, max_memory_bytes: usize, size_fn: fn(&V) -> usize) -> Self {
        Self {
            map: HashMap::new(),
            head: None,
            tail: None,
            capacity,
            max_memory_bytes,
            current_memory_bytes: 0,
            size_fn,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(&node_ptr) = self.map.get(key) {
            // Move to front
            self.move_to_front(node_ptr);

            unsafe {
                Some(&(*node_ptr.as_ptr()).value)
            }
        } else {
            None
        }
    }

    pub fn put(&mut self, key: K, value: V) {
        let value_size = (self.size_fn)(&value);

        // Check if key exists
        if let Some(&existing_ptr) = self.map.get(&key) {
            unsafe {
                let old_size = (self.size_fn)(&(*existing_ptr.as_ptr()).value);
                (*existing_ptr.as_ptr()).value = value;
                self.current_memory_bytes = self.current_memory_bytes - old_size + value_size;
            }
            self.move_to_front(existing_ptr);
            return;
        }

        // Evict items if necessary
        while self.map.len() >= self.capacity ||
              self.current_memory_bytes + value_size > self.max_memory_bytes {
            if !self.evict_lru() {
                break; // Cache is empty
            }
        }

        // Create new node
        let node = Box::new(LruNode {
            key: key.clone(),
            value,
            prev: None,
            next: None,
        });

        let node_ptr = NonNull::new(Box::into_raw(node)).unwrap();
        self.map.insert(key, node_ptr);
        self.current_memory_bytes += value_size;

        // Add to front
        self.add_to_front(node_ptr);
    }

    fn evict_lru(&mut self) -> bool {
        if let Some(tail_ptr) = self.tail {
            unsafe {
                let tail_key = (*tail_ptr.as_ptr()).key.clone();
                let tail_size = (self.size_fn)(&(*tail_ptr.as_ptr()).value);

                self.remove_node(tail_ptr);
                self.map.remove(&tail_key);
                self.current_memory_bytes -= tail_size;

                let _ = Box::from_raw(tail_ptr.as_ptr());
                true
            }
        } else {
            false
        }
    }

    unsafe fn move_to_front(&mut self, node_ptr: NonNull<LruNode<K, V>>) {
        self.remove_node(node_ptr);
        self.add_to_front(node_ptr);
    }

    unsafe fn add_to_front(&mut self, node_ptr: NonNull<LruNode<K, V>>) {
        (*node_ptr.as_ptr()).prev = None;
        (*node_ptr.as_ptr()).next = self.head;

        if let Some(head_ptr) = self.head {
            (*head_ptr.as_ptr()).prev = Some(node_ptr);
        } else {
            self.tail = Some(node_ptr);
        }

        self.head = Some(node_ptr);
    }

    unsafe fn remove_node(&mut self, node_ptr: NonNull<LruNode<K, V>>) {
        let node = &mut *node_ptr.as_ptr();

        match (node.prev, node.next) {
            (None, None) => {
                // Only node
                self.head = None;
                self.tail = None;
            }
            (None, Some(next)) => {
                // Head node
                (*next.as_ptr()).prev = None;
                self.head = Some(next);
            }
            (Some(prev), None) => {
                // Tail node
                (*prev.as_ptr()).next = None;
                self.tail = Some(prev);
            }
            (Some(prev), Some(next)) => {
                // Middle node
                (*prev.as_ptr()).next = Some(next);
                (*next.as_ptr()).prev = Some(prev);
            }
        }

        node.prev = None;
        node.next = None;
    }

    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            entries: self.map.len(),
            capacity: self.capacity,
            memory_bytes: self.current_memory_bytes,
            max_memory_bytes: self.max_memory_bytes,
            hit_rate: 0.0, // Would need additional tracking
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub entries: usize,
    pub capacity: usize,
    pub memory_bytes: usize,
    pub max_memory_bytes: usize,
    pub hit_rate: f64,
}

// Usage with search results
fn search_result_size(result: &SearchResults) -> usize {
    std::mem::size_of::<SearchResults>() +
    result.hits.len() * std::mem::size_of::<SearchHit>() +
    result.hits.iter().map(|hit| hit.title.len() + hit.snippet.len()).sum::<usize>()
}

pub type SearchCache = LruCache<String, SearchResults>;

impl SearchIndex {
    pub fn create_search_blz() -> SearchCache {
        LruCache::new(
            1000,                    // Max 1000 cached queries
            100 * 1024 * 1024,      // Max 100MB cache
            search_result_size,      // Size function
        )
    }
}
```

### Cache Warming Strategies

**Predictive Cache Loading**

```rust
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Query popularity tracker
pub struct QueryAnalytics {
    query_counts: Arc<RwLock<BTreeMap<String, QueryStats>>>,
    cleanup_interval: Duration,
}

#[derive(Debug, Clone)]
struct QueryStats {
    count: u64,
    last_used: std::time::Instant,
    average_execution_time: Duration,
}

impl QueryAnalytics {
    pub fn new() -> Self {
        Self {
            query_counts: Arc::new(RwLock::new(BTreeMap::new())),
            cleanup_interval: Duration::from_hours(1),
        }
    }

    pub fn record_query(&self, query: &str, execution_time: Duration) {
        let mut counts = self.query_counts.write().unwrap();

        let stats = counts.entry(query.to_string()).or_insert(QueryStats {
            count: 0,
            last_used: std::time::Instant::now(),
            average_execution_time: Duration::ZERO,
        });

        stats.count += 1;
        stats.last_used = std::time::Instant::now();

        // Update running average
        if stats.count == 1 {
            stats.average_execution_time = execution_time;
        } else {
            let alpha = 0.1; // Exponential moving average factor
            let new_avg_millis = (1.0 - alpha) * stats.average_execution_time.as_millis() as f64
                               + alpha * execution_time.as_millis() as f64;
            stats.average_execution_time = Duration::from_millis(new_avg_millis as u64);
        }
    }

    pub fn get_popular_queries(&self, limit: usize) -> Vec<String> {
        let counts = self.query_counts.read().unwrap();

        let mut queries: Vec<_> = counts
            .iter()
            .map(|(query, stats)| (query.clone(), stats.count))
            .collect();

        queries.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending
        queries.into_iter().take(limit).map(|(query, _)| query).collect()
    }

    pub fn cleanup_old_entries(&self) {
        let mut counts = self.query_counts.write().unwrap();
        let cutoff = std::time::Instant::now() - Duration::from_days(7);

        counts.retain(|_, stats| stats.last_used > cutoff);
    }

    pub async fn start_cleanup_task(self: Arc<Self>) {
        let mut interval = interval(self.cleanup_interval);

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                self.cleanup_old_entries();
            }
        });
    }
}

/// Cache warmer that preloads popular queries
pub struct CacheWarmer {
    analytics: Arc<QueryAnalytics>,
    search_index: Arc<SearchIndex>,
    cache: Arc<RwLock<SearchCache>>,
}

impl CacheWarmer {
    pub fn new(
        analytics: Arc<QueryAnalytics>,
        search_index: Arc<SearchIndex>,
        cache: Arc<RwLock<SearchCache>>,
    ) -> Self {
        Self {
            analytics,
            search_index,
            cache,
        }
    }

    /// Warm cache with popular queries during off-peak hours
    pub async fn warm_blz(&self) -> Result<usize, CacheError> {
        let popular_queries = self.analytics.get_popular_queries(50);
        let mut warmed_count = 0;

        for query in popular_queries {
            // Check if already cached
            {
                let cache = self.cache.read().unwrap();
                if cache.get(&query).is_some() {
                    continue;
                }
            }

            // Execute query and cache result
            match self.search_index.search(&query, 20).await {
                Ok(results) => {
                    let mut cache = self.cache.write().unwrap();
                    cache.put(query, results);
                    warmed_count += 1;
                }
                Err(e) => {
                    warn!("Failed to warm cache for query '{}': {}", query, e);
                }
            }

            // Small delay to avoid overwhelming the system
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Ok(warmed_count)
    }

    pub async fn start_warming_task(self: Arc<Self>) {
        let mut interval = interval(Duration::from_hours(2));

        tokio::spawn(async move {
            loop {
                interval.tick().await;

                // Only warm during off-peak hours (adjust for your timezone)
                let now = chrono::Local::now();
                let hour = now.hour();

                if (2..6).contains(&hour) { // 2 AM - 6 AM
                    match self.warm_blz().await {
                        Ok(count) => {
                            info!("Warmed cache with {} popular queries", count);
                        }
                        Err(e) => {
                            error!("Cache warming failed: {}", e);
                        }
                    }
                }
            }
        });
    }
}
```

## Async Optimization

### Efficient Async Patterns

**Concurrent Operations**

```rust
use futures::future::{try_join, try_join_all};
use tokio::sync::Semaphore;
use std::sync::Arc;

/// Concurrent search across multiple indices
pub struct MultiIndexSearch {
    indices: Vec<Arc<SearchIndex>>,
    concurrency_limit: Arc<Semaphore>,
}

impl MultiIndexSearch {
    pub fn new(indices: Vec<Arc<SearchIndex>>, max_concurrent: usize) -> Self {
        Self {
            indices,
            concurrency_limit: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Search all indices concurrently and merge results
    pub async fn search_all(&self, query: &str, limit_per_index: u16) -> CacheResult<SearchResults> {
        let search_futures = self.indices.iter().map(|index| {
            let query = query.to_string();
            let index = Arc::clone(index);
            let semaphore = Arc::clone(&self.concurrency_limit);

            async move {
                let _permit = semaphore.acquire().await.unwrap();
                index.search(&query, limit_per_index).await
            }
        });

        // Execute all searches concurrently
        let results = try_join_all(search_futures).await?;

        // Merge and sort results by score
        let merged = self.merge_search_results(results, query.len() as u16 * limit_per_index);
        Ok(merged)
    }

    fn merge_search_results(&self, results: Vec<SearchResults>, total_limit: u16) -> SearchResults {
        let mut all_hits = Vec::new();
        let mut total_count = 0;
        let mut max_execution_time = Duration::ZERO;

        for result in results {
            all_hits.extend(result.hits);
            total_count += result.total_count;
            max_execution_time = max_execution_time.max(result.execution_time);
        }

        // Sort by score and limit results
        all_hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        all_hits.truncate(total_limit as usize);

        SearchResults {
            hits: all_hits,
            total_count,
            execution_time: max_execution_time,
            from_blz: false,
        }
    }
}

/// Batch processing with backpressure
pub struct BatchProcessor<T> {
    batch_size: usize,
    flush_interval: Duration,
    processor_fn: Arc<dyn Fn(Vec<T>) -> BoxFuture<'static, Result<(), CacheError>> + Send + Sync>,
    sender: mpsc::Sender<T>,
}

impl<T: Send + 'static> BatchProcessor<T> {
    pub fn new<F, Fut>(
        batch_size: usize,
        flush_interval: Duration,
        processor_fn: F,
    ) -> Self
    where
        F: Fn(Vec<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), CacheError>> + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel(batch_size * 2);

        let processor = Arc::new(move |batch| {
            Box::pin(processor_fn(batch)) as BoxFuture<'static, Result<(), CacheError>>
        });

        Self::start_batch_worker(receiver, batch_size, flush_interval, Arc::clone(&processor));

        Self {
            batch_size,
            flush_interval,
            processor_fn: processor,
            sender,
        }
    }

    pub async fn send(&self, item: T) -> Result<(), CacheError> {
        self.sender.send(item).await
            .map_err(|_| CacheError::ResourceUnavailable {
                resource: "batch_processor_queue".to_string(),
                source: None,
            })
    }

    fn start_batch_worker(
        mut receiver: mpsc::Receiver<T>,
        batch_size: usize,
        flush_interval: Duration,
        processor: Arc<dyn Fn(Vec<T>) -> BoxFuture<'static, Result<(), CacheError>> + Send + Sync>,
    ) {
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(batch_size);
            let mut flush_timer = interval(flush_interval);

            loop {
                tokio::select! {
                    // Receive new items
                    item = receiver.recv() => {
                        match item {
                            Some(item) => {
                                batch.push(item);

                                // Flush if batch is full
                                if batch.len() >= batch_size {
                                    let current_batch = std::mem::take(&mut batch);
                                    if let Err(e) = processor(current_batch).await {
                                        error!("Batch processing failed: {}", e);
                                    }
                                }
                            }
                            None => {
                                // Channel closed, flush remaining items
                                if !batch.is_empty() {
                                    let _ = processor(batch).await;
                                }
                                break;
                            }
                        }
                    }

                    // Periodic flush
                    _ = flush_timer.tick() => {
                        if !batch.is_empty() {
                            let current_batch = std::mem::take(&mut batch);
                            if let Err(e) = processor(current_batch).await {
                                error!("Batch processing failed: {}", e);
                            }
                        }
                    }
                }
            }
        });
    }
}

// Usage for indexing operations
impl SearchIndex {
    pub fn create_indexing_processor(&self) -> BatchProcessor<Document> {
        let index = Arc::clone(&self.index);

        BatchProcessor::new(
            100, // Batch size
            Duration::from_secs(5), // Flush interval
            move |documents| {
                let index = Arc::clone(&index);
                async move {
                    // Process batch of documents
                    let mut writer = index.writer(50_000_000)?; // 50MB heap

                    for doc in documents {
                        writer.add_document(doc)?;
                    }

                    writer.commit()?;
                    Ok(())
                }
            }
        )
    }
}
```

## Performance Anti-Patterns

### Avoid These Patterns

**Common Performance Mistakes**

```rust
// ❌ Unnecessary cloning in hot paths
pub fn format_results(results: &[SearchHit]) -> Vec<String> {
    results.iter()
        .map(|hit| hit.clone()) // Expensive clone!
        .map(|hit| format!("{}: {}", hit.title, hit.snippet))
        .collect()
}

// ✅ Work with references
pub fn format_results(results: &[SearchHit]) -> Vec<String> {
    results.iter()
        .map(|hit| format!("{}: {}", hit.title, hit.snippet))
        .collect()
}

// ❌ Allocating in loops
pub fn process_queries(queries: &[&str]) -> Vec<String> {
    let mut results = Vec::new();
    for query in queries {
        let mut processed = String::new(); // New allocation each time
        processed.push_str("processed: ");
        processed.push_str(query);
        results.push(processed);
    }
    results
}

// ✅ Pre-allocate and reuse
pub fn process_queries(queries: &[&str]) -> Vec<String> {
    let mut results = Vec::with_capacity(queries.len());
    let mut buffer = String::with_capacity(100); // Reuse buffer

    for query in queries {
        buffer.clear();
        buffer.push_str("processed: ");
        buffer.push_str(query);
        results.push(buffer.clone());
    }
    results
}

// ❌ Blocking in async context
pub async fn search_and_log(cache: &SearchCache, query: &str) -> CacheResult<SearchResults> {
    let results = cache.search(query, 10).await?;

    // This blocks the async runtime!
    std::fs::write("search.log", format!("Query: {}\n", query))?;

    Ok(results)
}

// ✅ Use async file operations
pub async fn search_and_log(cache: &SearchCache, query: &str) -> CacheResult<SearchResults> {
    let results = cache.search(query, 10).await?;

    // Non-blocking file write
    tokio::fs::write("search.log", format!("Query: {}\n", query)).await?;

    Ok(results)
}
```

Remember: Performance optimization is an iterative process. Always profile before optimizing, focus on the bottlenecks that matter most, and measure the impact of your changes. The best optimization is often the code you don't need to write.
