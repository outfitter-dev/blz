#![allow(unsafe_code)] // Core module requires unsafe for performance-critical LRU cache operations
#![deny(unsafe_op_in_unsafe_fn)]
// SAFETY: This module follows the project unsafe policy:
// .agents/rules/conventions/rust/unsafe-policy.md
//! Multi-level caching with LRU eviction and TTL-based entries.
//!
//! The cache is split into a fast L1 LRU layer and a larger L2 TTL layer with
//! size limits. [`MultiLevelCache`] coordinates lookups, promotions, and
//! evictions while tracking [`CacheStats`] for observability. Callers configure
//! cache behavior with [`CacheConfig`] and provide a size function for accurate
//! memory accounting.
//!
//! This module is performance-critical and uses a hand-rolled LRU
//! implementation with unsafe pointer operations to minimize overhead.
use crate::{Error, Result, SearchHit};
use chrono::Utc;
use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info, warn};

/// Multi-level cache with LRU eviction, TTL, and size limits
pub struct MultiLevelCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    /// L1 cache: Fast, small, in-memory
    l1_cache: Arc<RwLock<LruCache<K, CacheEntry<V>>>>,
    
    /// L2 cache: Larger, with TTL
    l2_cache: Arc<RwLock<TtlCache<K, V>>>,
    
    /// Cache configuration
    config: CacheConfig,
    
    /// Statistics
    stats: Arc<CacheStats>,
    
    /// Background cleanup task handle
    _cleanup_task: tokio::task::JoinHandle<()>,
}

/// Configuration for cache behavior
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// L1 cache maximum entries
    pub l1_max_entries: usize,
    
    /// L1 cache maximum memory (bytes)
    pub l1_max_memory: usize,
    
    /// L2 cache maximum entries
    pub l2_max_entries: usize,
    
    /// L2 cache maximum memory (bytes)
    pub l2_max_memory: usize,
    
    /// Default TTL for L2 cache entries
    pub default_ttl: Duration,
    
    /// Cleanup interval
    pub cleanup_interval: Duration,
    
    /// Function to calculate entry size
    pub size_fn: fn(&V) -> usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_max_entries: 1000,
            l1_max_memory: 10 * 1024 * 1024, // 10MB
            l2_max_entries: 10000,
            l2_max_memory: 100 * 1024 * 1024, // 100MB
            default_ttl: Duration::from_secs(3600), // 1 hour
            cleanup_interval: Duration::from_secs(60), // 1 minute
            size_fn: |_| 1, // Default: count entries, not size
        }
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    created_at: Instant,
    last_accessed: Instant,
    access_count: u64,
    size: usize,
}

impl<V> CacheEntry<V> {
    fn new(value: V, size: usize) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            size,
        }
    }
    
    fn access(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }
}

/// TTL cache entry
#[derive(Debug, Clone)]
struct TtlEntry<V> {
    value: V,
    expires_at: Instant,
    size: usize,
}

impl<K, V> MultiLevelCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new multi-level cache
    pub fn new(config: CacheConfig) -> Self {
        let l1_cache = Arc::new(RwLock::new(
            LruCache::new(config.l1_max_entries, config.l1_max_memory)
        ));
        
        let l2_cache = Arc::new(RwLock::new(
            TtlCache::new(config.l2_max_entries, config.l2_max_memory)
        ));
        
        let stats = Arc::new(CacheStats::default());
        
        // Start cleanup task
        let cleanup_task = {
            let l1_cache = Arc::clone(&l1_cache);
            let l2_cache = Arc::clone(&l2_cache);
            let stats = Arc::clone(&stats);
            let cleanup_interval = config.cleanup_interval;
            
            tokio::spawn(async move {
                let mut interval_timer = interval(cleanup_interval);
                
                loop {
                    interval_timer.tick().await;
                    
                    // Clean up L2 cache (TTL)
                    {
                        let mut l2 = l2_cache.write().await;
                        let removed = l2.cleanup_expired();
                        if removed > 0 {
                            stats.evictions.fetch_add(removed, Ordering::Relaxed);
                            debug!("Cleaned up {} expired L2 entries", removed);
                        }
                    }
                    
                    // Optionally clean up L1 cache based on age
                    {
                        let mut l1 = l1_cache.write().await;
                        let old_cutoff = Instant::now() - Duration::from_secs(1800); // 30 minutes
                        let removed = l1.cleanup_old_entries(old_cutoff);
                        if removed > 0 {
                            stats.evictions.fetch_add(removed, Ordering::Relaxed);
                            debug!("Cleaned up {} old L1 entries", removed);
                        }
                    }
                }
            })
        };

        Self {
            l1_cache,
            l2_cache,
            config,
            stats,
            _cleanup_task: cleanup_task,
        }
    }

    /// Get value from cache (tries L1 then L2).
    ///
    /// # Errors
    ///
    /// This method does not return errors.
    pub async fn get(&self, key: &K) -> Option<V> {
        self.stats.requests.fetch_add(1, Ordering::Relaxed);

        // Try L1 cache first
        {
            let mut l1 = self.l1_cache.write().await;
            if let Some(entry) = l1.get_mut(key) {
                entry.access();
                self.stats.l1_hits.fetch_add(1, Ordering::Relaxed);
                return Some(entry.value.clone());
            }
        }

        // Avoid holding the RwLock guard across .await
        let l2_value = {
            let l2 = self.l2_cache.read().await;
            l2.get(key)
        };
        if let Some(value) = l2_value {
            self.stats.l2_hits.fetch_add(1, Ordering::Relaxed);
            // Promote to L1 cache
            self.promote_to_l1(key.clone(), value.clone()).await;
            return Some(value);
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Put value in cache (goes to L1 and L2).
    ///
    /// # Errors
    ///
    /// This method does not return errors.
    pub async fn put(&self, key: K, value: V) {
        self.put_with_ttl(key, value, self.config.default_ttl).await;
    }

    /// Put value in cache with custom TTL
    pub async fn put_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let size = (self.config.size_fn)(&value);

        // Put in L1 cache
        {
            let mut l1 = self.l1_cache.write().await;
            let entry = CacheEntry::new(value.clone(), size);
            l1.put(key.clone(), entry);
        }

        // Put in L2 cache with TTL
        {
            let mut l2 = self.l2_cache.write().await;
            let expires_at = Instant::now() + ttl;
            let entry = TtlEntry {
                value,
                expires_at,
                size,
            };
            l2.put(key, entry);
        }

        self.stats.puts.fetch_add(1, Ordering::Relaxed);
    }

    /// Remove value from both caches
    pub async fn remove(&self, key: &K) -> Option<V> {
        let l1_value = {
            let mut l1 = self.l1_cache.write().await;
            l1.remove(key).map(|entry| entry.value)
        };

        let l2_value = {
            let mut l2 = self.l2_cache.write().await;
            l2.remove(key).map(|entry| entry.value)
        };

        l1_value.or(l2_value)
    }

    /// Promote value from L2 to L1 cache
    async fn promote_to_l1(&self, key: K, value: V) {
        let size = (self.config.size_fn)(&value);
        let entry = CacheEntry::new(value, size);
        
        let mut l1 = self.l1_cache.write().await;
        l1.put(key, entry);
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStatsSummary {
        let l1_stats = {
            let l1 = self.l1_cache.read().await;
            l1.stats()
        };
        
        let l2_stats = {
            let l2 = self.l2_cache.read().await;
            l2.stats()
        };

        CacheStatsSummary {
            requests: self.stats.requests.load(Ordering::Relaxed),
            l1_hits: self.stats.l1_hits.load(Ordering::Relaxed),
            l2_hits: self.stats.l2_hits.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            puts: self.stats.puts.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            l1_entries: l1_stats.entries,
            l1_memory_bytes: l1_stats.memory_bytes,
            l2_entries: l2_stats.entries,
            l2_memory_bytes: l2_stats.memory_bytes,
            hit_rate: {
                let total_requests = self.stats.requests.load(Ordering::Relaxed);
                if total_requests > 0 {
                    let hits = self.stats.l1_hits.load(Ordering::Relaxed) 
                             + self.stats.l2_hits.load(Ordering::Relaxed);
                    hits as f64 / total_requests as f64
                } else {
                    0.0
                }
            },
        }
    }

    /// Clear all caches
    pub async fn clear(&self) {
        let mut l1 = self.l1_cache.write().await;
        let mut l2 = self.l2_cache.write().await;
        l1.clear();
        l2.clear();
    }
}

// Specialized helpers for search result caches (String keys)
impl MultiLevelCache<String, Vec<SearchHit>> {
    /// Invalidate all cached search results for a specific alias (keys prefixed by `"alias:"`).
    /// Returns the number of entries removed across L1 and L2.
    pub async fn invalidate_alias(&self, alias: &str) -> usize {
        let prefix = format!("{alias}:");
        self.remove_prefix(&prefix).await
    }

    /// Remove all entries whose keys start with the given prefix.
    /// Returns the number of entries removed across L1 and L2.
    async fn remove_prefix(&self, prefix: &str) -> usize {
        let mut removed = 0usize;

        // L1 scan + remove
        {
            let mut l1 = self.l1_cache.write().await;
            // Collect keys first to avoid mutating while iterating
            let keys: Vec<String> = l1
                .map
                .keys()
                .filter(|k| k.starts_with(prefix))
                .cloned()
                .collect();
            for k in keys {
                if l1.remove(&k).is_some() {
                    removed += 1;
                }
            }
        }

        // L2 scan + remove
        {
            let mut l2 = self.l2_cache.write().await;
            let keys: Vec<String> = l2
                .map
                .keys()
                .filter(|k| k.starts_with(prefix))
                .cloned()
                .collect();
            for k in keys {
                if l2.remove(&k).is_some() {
                    removed += 1;
                }
            }
        }

        removed
    }
}

/// LRU cache implementation with size tracking
struct LruCache<K, V> 
where
    K: Hash + Eq + Clone,
{
    map: HashMap<K, NonNull<Node<K, V>>>,
    head: Option<NonNull<Node<K, V>>>,
    tail: Option<NonNull<Node<K, V>>>,
    capacity: usize,
    max_memory: usize,
    current_memory: usize,
}

struct Node<K, V> {
    key: K,
    value: V,
    prev: Option<NonNull<Node<K, V>>>,
    next: Option<NonNull<Node<K, V>>>,
}

impl<K, V> LruCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    fn new(capacity: usize, max_memory: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity.min(1000)),
            head: None,
            tail: None,
            capacity,
            max_memory,
            current_memory: 0,
        }
    }

    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if let Some(&node_ptr) = self.map.get(key) {
           unsafe { self.move_to_front(node_ptr) };
            // SAFETY: node_ptr is valid because:
            // 1. It came from self.map which only stores valid NonNull<Node<K, V>>
            // 2. The node is owned by this cache and its lifetime is managed by the HashMap
            // 3. We have mutable access to self, ensuring no other references exist
            unsafe { Some(&mut (*node_ptr.as_ptr()).value) }
        } else {
            None
        }
    }

    fn put(&mut self, key: K, value: V) {
        let memory_size = std::mem::size_of::<Node<K, V>>();

        if let Some(&existing_ptr) = self.map.get(&key) {
            // SAFETY: existing_ptr is valid because:
            // 1. It came from self.map which only stores valid NonNull<Node<K, V>>
            // 2. The node exists and is owned by this cache
            // 3. We have exclusive access through &mut self
            unsafe {
                (*existing_ptr.as_ptr()).value = value;
            }
            unsafe { self.move_to_front(existing_ptr); }
            return;
        }

        // Evict if necessary
        while (self.map.len() >= self.capacity || 
               self.current_memory + memory_size > self.max_memory) && 
              !self.map.is_empty() {
            self.evict_lru();
        }

        // Create new node
        let node = Box::new(Node {
            key: key.clone(),
            value,
            prev: None,
            next: None,
        });

        // SAFETY: `Box::into_raw` never returns a null pointer. It converts a
        // uniquely-owned Box into a raw pointer to its heap allocation. Since the
        // allocation exists, the returned pointer is guaranteed non-null.
        // Using `new_unchecked` avoids an unnecessary `Option` and prevents
        // introducing `unwrap`/`expect` in production code per project policy.
        let node_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(node)) };
        self.map.insert(key, node_ptr);
        self.current_memory += memory_size;
        unsafe { self.add_to_front(node_ptr); }
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(&node_ptr) = self.map.get(key) {
            // SAFETY: node_ptr is valid because:
            // 1. It came from self.map which only stores valid NonNull<Node<K, V>>
            // 2. We immediately remove it from the map, ensuring no double-free
            // 3. The pointer was created by Box::into_raw in put()
            unsafe {
                let node = Box::from_raw(node_ptr.as_ptr());
                self.remove_node(node_ptr);
                self.map.remove(key);
                self.current_memory = self.current_memory.saturating_sub(
                    std::mem::size_of::<Node<K, V>>()
                );
                Some(node.value)
            }
        } else {
            None
        }
    }

    fn clear(&mut self) {
        while let Some(tail_ptr) = self.tail {
            // SAFETY: tail_ptr is valid because:
            // 1. self.tail only contains valid NonNull<Node<K, V>> from our managed nodes
            // 2. The node exists in the linked list and is owned by this cache
            // 3. We have exclusive access through &mut self
            unsafe {
                let tail_key = (*tail_ptr.as_ptr()).key.clone();
                self.remove(&tail_key);
            }
        }
    }

    fn cleanup_old_entries(&mut self, cutoff_time: Instant) -> usize {
        // This method is only meaningful for caches storing CacheEntry<T> values
        // For generic LruCache, we can't determine entry age without knowing the type
        // This functionality should be implemented at the MultiLevelCache level instead
        0
    }

    fn evict_lru(&mut self) {
        if let Some(tail_ptr) = self.tail {
            // SAFETY: tail_ptr is valid because:
            // 1. self.tail only contains valid NonNull<Node<K, V>> from our managed nodes
            // 2. The node exists in the linked list and is owned by this cache
            // 3. We have exclusive access through &mut self
            unsafe {
                let tail_key = (*tail_ptr.as_ptr()).key.clone();
                self.remove(&tail_key);
            }
        }
    }

    unsafe fn move_to_front(&mut self, node_ptr: NonNull<Node<K, V>>) {
        // SAFETY: caller guarantees node_ptr is valid and owned by this cache
        self.remove_node(node_ptr);
        self.add_to_front(node_ptr);
    }

    unsafe fn add_to_front(&mut self, node_ptr: NonNull<Node<K, V>>) {
        // SAFETY: caller guarantees node_ptr is valid and owned by this cache
        // All pointer manipulations maintain the doubly-linked list invariants
        (*node_ptr.as_ptr()).prev = None;
        (*node_ptr.as_ptr()).next = self.head;

        if let Some(head_ptr) = self.head {
            (*head_ptr.as_ptr()).prev = Some(node_ptr);
        } else {
            self.tail = Some(node_ptr);
        }

        self.head = Some(node_ptr);
    }

    unsafe fn remove_node(&mut self, node_ptr: NonNull<Node<K, V>>) {
        // SAFETY: caller guarantees node_ptr is valid and owned by this cache
        // All pointer manipulations maintain the doubly-linked list invariants
        let node = &mut *node_ptr.as_ptr();

        match (node.prev, node.next) {
            (None, None) => {
                self.head = None;
                self.tail = None;
            }
            (None, Some(next)) => {
                (*next.as_ptr()).prev = None;
                self.head = Some(next);
            }
            (Some(prev), None) => {
                (*prev.as_ptr()).next = None;
                self.tail = Some(prev);
            }
            (Some(prev), Some(next)) => {
                (*prev.as_ptr()).next = Some(next);
                (*next.as_ptr()).prev = Some(prev);
            }
        }

        node.prev = None;
        node.next = None;
    }

    fn stats(&self) -> CacheLevel1Stats {
        CacheLevel1Stats {
            entries: self.map.len(),
            memory_bytes: self.current_memory,
        }
    }
}

/// TTL cache with automatic expiration
struct TtlCache<K, V>
where
    K: Hash + Eq + Clone,
{
    map: HashMap<K, TtlEntry<V>>,
    max_entries: usize,
    max_memory: usize,
    current_memory: usize,
}

impl<K, V> TtlCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    fn new(max_entries: usize, max_memory: usize) -> Self {
        Self {
            map: HashMap::with_capacity(max_entries.min(10000)),
            max_entries,
            max_memory,
            current_memory: 0,
        }
    }

    fn get(&self, key: &K) -> Option<V> {
        if let Some(entry) = self.map.get(key) {
            if Instant::now() < entry.expires_at {
                Some(entry.value.clone())
            } else {
                None // Expired
            }
        } else {
            None
        }
    }

    fn put(&mut self, key: K, entry: TtlEntry<V>) {
        let entry_size = std::mem::size_of::<TtlEntry<V>>() + 
                        std::mem::size_of::<K>() + 
                        entry.size;

        // Remove existing entry if present
        if let Some(old_entry) = self.map.remove(&key) {
            self.current_memory = self.current_memory.saturating_sub(
                std::mem::size_of::<TtlEntry<V>>() + std::mem::size_of::<K>() + old_entry.size
            );
        }

        // Evict if necessary
        while (self.map.len() >= self.max_entries || 
               self.current_memory + entry_size > self.max_memory) && 
              !self.map.is_empty() {
            self.evict_random();
        }

        self.current_memory += entry_size;
        self.map.insert(key, entry);
    }

    fn remove(&mut self, key: &K) -> Option<TtlEntry<V>> {
        if let Some(entry) = self.map.remove(key) {
            self.current_memory = self.current_memory.saturating_sub(
                std::mem::size_of::<TtlEntry<V>>() + std::mem::size_of::<K>() + entry.size
            );
            Some(entry)
        } else {
            None
        }
    }

    fn cleanup_expired(&mut self) -> usize {
        let now = Instant::now();
        let mut expired_keys = Vec::new();

        for (key, entry) in &self.map {
            if now >= entry.expires_at {
                expired_keys.push(key.clone());
            }
        }

        let removed_count = expired_keys.len();
        for key in expired_keys {
            self.remove(&key);
        }

        removed_count
    }

    fn evict_random(&mut self) {
        if let Some(key) = self.map.keys().next().cloned() {
            self.remove(&key);
        }
    }

    fn clear(&mut self) {
        self.map.clear();
        self.current_memory = 0;
    }

    fn stats(&self) -> CacheLevel2Stats {
        CacheLevel2Stats {
            entries: self.map.len(),
            memory_bytes: self.current_memory,
        }
    }
}

/// Cache statistics
#[derive(Default)]
struct CacheStats {
    requests: AtomicUsize,
    l1_hits: AtomicUsize,
    l2_hits: AtomicUsize,
    misses: AtomicUsize,
    puts: AtomicUsize,
    evictions: AtomicUsize,
}

/// Cache statistics summary
#[derive(Debug, Clone)]
pub struct CacheStatsSummary {
    /// Total cache lookups across all levels.
    pub requests: usize,
    /// Hits served from the L1 LRU cache.
    pub l1_hits: usize,
    /// Hits served from the L2 TTL cache.
    pub l2_hits: usize,
    /// Lookups that missed both cache levels.
    pub misses: usize,
    /// Cache insertions.
    pub puts: usize,
    /// Evictions across both cache tiers.
    pub evictions: usize,
    /// Current L1 entry count.
    pub l1_entries: usize,
    /// Current L1 memory usage in bytes.
    pub l1_memory_bytes: usize,
    /// Current L2 entry count.
    pub l2_entries: usize,
    /// Current L2 memory usage in bytes.
    pub l2_memory_bytes: usize,
    /// Hit rate as `(l1_hits + l2_hits) / requests`.
    pub hit_rate: f64,
}

#[derive(Debug, Clone)]
struct CacheLevel1Stats {
    entries: usize,
    memory_bytes: usize,
}

#[derive(Debug, Clone)]
struct CacheLevel2Stats {
    entries: usize,
    memory_bytes: usize,
}

/// Search result cache specialized for `SearchHit` values.
pub type SearchCache = MultiLevelCache<String, Vec<SearchHit>>;

impl SearchCache {
    fn build_cache_key(
        query: &str,
        alias: Option<&str>,
        version: Option<&str>,
    ) -> String {
        let alias_key = alias.filter(|a| !a.is_empty()).unwrap_or("~");
        let version_key = version.filter(|v| !v.is_empty()).unwrap_or("v0");

        format!(
            "a:{alias}|v:{version}|q:{query}",
            alias = alias_key,
            version = version_key,
            query = query
        )
    }

    /// Create a new search result cache with optimized configuration
    pub fn new_search_cache() -> Self {
        let config = CacheConfig {
            l1_max_entries: 500,
            l1_max_memory: 5 * 1024 * 1024, // 5MB
            l2_max_entries: 5000,
            l2_max_memory: 50 * 1024 * 1024, // 50MB
            default_ttl: Duration::from_secs(1800), // 30 minutes
            cleanup_interval: Duration::from_secs(120), // 2 minutes
            size_fn: search_result_size,
        };
        
        MultiLevelCache::new(config)
    }

    /// Cache search results with query as key
    pub async fn cache_search_results(
        &self,
        query: &str,
        alias: Option<&str>,
        results: Vec<SearchHit>,
    ) {
        let cache_key = Self::build_cache_key(query, alias, None);
        self.put(cache_key, results).await;
    }

    /// Get cached search results
    pub async fn get_cached_results(
        &self,
        query: &str,
        alias: Option<&str>,
    ) -> Option<Vec<SearchHit>> {
        let cache_key = Self::build_cache_key(query, alias, None);
        self.get(&cache_key).await
    }

    /// Versioned cache put: embeds a version token into the cache key so that
    /// updates invalidate old keys without explicit deletion.
    pub async fn cache_search_results_v(
        &self,
        query: &str,
        alias: Option<&str>,
        version: Option<&str>,
        results: Vec<SearchHit>,
    ) {
        let cache_key = Self::build_cache_key(query, alias, version);
        self.put(cache_key, results).await;
    }

    /// Versioned cache get: must use the same version token as used to put.
    pub async fn get_cached_results_v(
        &self,
        query: &str,
        alias: Option<&str>,
        version: Option<&str>,
    ) -> Option<Vec<SearchHit>> {
        let cache_key = Self::build_cache_key(query, alias, version);
        self.get(&cache_key).await
    }
}

/// Calculate approximate size of search results for caching
fn search_result_size(results: &Vec<SearchHit>) -> usize {
    std::mem::size_of::<Vec<SearchHit>>() + 
    results.len() * std::mem::size_of::<SearchHit>() +
    results.iter().map(|hit| {
        hit.source.len() +
        hit.file.len() +
        hit.heading_path.iter().map(|s| s.len()).sum::<usize>() +
        hit.lines.len() +
        hit.line_numbers.as_ref().map(|v| v.len() * std::mem::size_of::<usize>()).unwrap_or(0) +
        hit.snippet.len() +
        hit.source_url.as_ref().map(|s| s.len()).unwrap_or(0) +
        hit.checksum.len() +
        hit.context.as_ref().map(|ctx| {
            ctx.lines.len()
                + ctx.line_numbers.len() * std::mem::size_of::<usize>()
                + ctx.content.len()
        }).unwrap_or(0)
    }).sum::<usize>()
}

/// Query result caching with intelligent prefetching
pub struct QueryCache {
    /// Main cache for results
    cache: SearchCache,
    
    /// Query pattern analyzer for prefetching
    query_analyzer: Arc<RwLock<QueryAnalyzer>>,
    
    /// Background prefetch task
    _prefetch_task: tokio::task::JoinHandle<()>,
}

impl QueryCache {
    /// Create new query cache with prefetching
    pub fn new() -> Self {
        let cache = SearchCache::new_search_cache();
        let query_analyzer = Arc::new(RwLock::new(QueryAnalyzer::new()));

        // Start prefetch task
        let prefetch_task = {
            let analyzer = Arc::clone(&query_analyzer);
            tokio::spawn(async move {
                let mut interval_timer = interval(Duration::from_secs(300)); // 5 minutes
                
                loop {
                    interval_timer.tick().await;
                    
                    let analyzer_guard = analyzer.read().await;
                    let popular_queries = analyzer_guard.get_popular_queries(10);
                    drop(analyzer_guard);
                    
                    // Here you would trigger prefetch operations for popular queries
                    // This is left as a placeholder for actual search index integration
                    if !popular_queries.is_empty() {
                        debug!("Popular queries for prefetch: {:?}", popular_queries);
                    }
                }
            })
        };

        Self {
            cache,
            query_analyzer,
            _prefetch_task: prefetch_task,
        }
    }

    /// Get cached results and update query analytics
    pub async fn get(
        &self,
        query: &str,
        alias: Option<&str>,
    ) -> Option<Vec<SearchHit>> {
        {
            let mut analyzer = self.query_analyzer.write().await;
            analyzer.record_query(query);
        }

        self.cache.get_cached_results(query, alias).await
    }

    /// Cache results and update analytics
    pub async fn put(
        &self,
        query: &str,
        alias: Option<&str>,
        results: Vec<SearchHit>,
    ) {
        self.cache
            .cache_search_results(query, alias, results)
            .await;
    }

    /// Get cache statistics including query analytics
    pub async fn stats(&self) -> QueryCacheStats {
        let cache_stats = self.cache.stats().await;
        let analyzer_stats = {
            let analyzer = self.query_analyzer.read().await;
            analyzer.stats()
        };

        QueryCacheStats {
            cache_stats,
            total_queries: analyzer_stats.total_queries,
            unique_queries: analyzer_stats.unique_queries,
            popular_queries: analyzer_stats.popular_queries,
        }
    }
}

/// Query pattern analyzer for prefetching optimization
struct QueryAnalyzer {
    query_counts: HashMap<String, QueryPattern>,
    total_queries: usize,
}

#[derive(Debug, Clone)]
struct QueryPattern {
    count: usize,
    first_seen: Instant,
    last_seen: Instant,
    avg_interval: Duration,
}

impl QueryAnalyzer {
    fn new() -> Self {
        Self {
            query_counts: HashMap::new(),
            total_queries: 0,
        }
    }

    fn record_query(&mut self, query: &str) {
        self.total_queries += 1;
        let now = Instant::now();

        match self.query_counts.get_mut(query) {
            Some(pattern) => {
                let interval = now.duration_since(pattern.last_seen);
                pattern.avg_interval = if pattern.count == 1 {
                    interval
                } else {
                    Duration::from_nanos(
                        (pattern.avg_interval.as_nanos() as f64 * 0.8 + 
                         interval.as_nanos() as f64 * 0.2) as u64
                    )
                };
                pattern.count += 1;
                pattern.last_seen = now;
            }
            None => {
                self.query_counts.insert(query.to_string(), QueryPattern {
                    count: 1,
                    first_seen: now,
                    last_seen: now,
                    avg_interval: Duration::from_secs(0),
                });
            }
        }

        // Clean up old queries periodically
        if self.total_queries % 1000 == 0 {
            self.cleanup_old_queries();
        }
    }

    fn get_popular_queries(&self, limit: usize) -> Vec<String> {
        let mut queries: Vec<_> = self.query_counts.iter()
            .map(|(query, pattern)| (pattern.count, query.clone()))
            .collect();
        
        queries.sort_by(|a, b| b.0.cmp(&a.0));
        queries.into_iter().take(limit).map(|(_, query)| query).collect()
    }

    fn cleanup_old_queries(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(3600); // 1 hour
        self.query_counts.retain(|_, pattern| pattern.last_seen > cutoff);
    }

    fn stats(&self) -> QueryAnalyzerStats {
        QueryAnalyzerStats {
            total_queries: self.total_queries,
            unique_queries: self.query_counts.len(),
            popular_queries: self.get_popular_queries(5),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryAnalyzerStats {
    /// Total number of queries observed.
    pub total_queries: usize,
    /// Number of unique query strings seen.
    pub unique_queries: usize,
    /// Most frequent queries, ordered by count.
    pub popular_queries: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct QueryCacheStats {
    /// Snapshot of the underlying cache statistics.
    pub cache_stats: CacheStatsSummary,
    /// Total number of queries observed.
    pub total_queries: usize,
    /// Number of unique query strings seen.
    pub unique_queries: usize,
    /// Most frequent queries, ordered by count.
    pub popular_queries: Vec<String>,
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_multi_level_cache_basic() {
        let config = CacheConfig::default();
        let cache = MultiLevelCache::new(config);
        
        // Test basic put/get
        cache.put("key1".to_string(), "value1".to_string()).await;
        let result = cache.get(&"key1".to_string()).await;
        assert_eq!(result, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_cache_promotion() {
        let config = CacheConfig {
            l1_max_entries: 1, // Very small L1 to force L2 usage
            ..Default::default()
        };
        let cache = MultiLevelCache::new(config);
        
        // Fill L1 and push to L2
        cache.put("key1".to_string(), "value1".to_string()).await;
        cache.put("key2".to_string(), "value2".to_string()).await; // Should evict key1 from L1
        
        // Access key1 - should promote from L2 to L1
        let result = cache.get(&"key1".to_string()).await;
        assert_eq!(result, Some("value1".to_string()));
        
        let stats = cache.stats().await;
        assert!(stats.l2_hits > 0);
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let config = CacheConfig {
            default_ttl: Duration::from_millis(100), // Very short TTL
            cleanup_interval: Duration::from_millis(50),
            ..Default::default()
        };
        let cache = MultiLevelCache::new(config);
        
        cache.put("key1".to_string(), "value1".to_string()).await;
        
        // Should be available immediately
        let result1 = cache.get(&"key1".to_string()).await;
        assert_eq!(result1, Some("value1".to_string()));
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Should be expired now
        let result2 = cache.get(&"key1".to_string()).await;
        assert_eq!(result2, None);
    }

    #[tokio::test]
    async fn test_search_cache() {
        let cache = SearchCache::new_search_cache();
        
        let results = vec![SearchHit {
            source: "test".to_string(),
            file: "test.md".to_string(),
            heading_path: vec!["Test".to_string()],
            raw_heading_path: Some(vec!["Test".to_string()]),
            level: 1,
            lines: "1-10".to_string(),
            line_numbers: Some(vec![1, 10]),
            snippet: "test snippet".to_string(),
            score: 0.95,
            source_url: Some("https://test.com".to_string()),
            fetched_at: Some(Utc::now()),
            is_stale: false,
            checksum: "abc123".to_string(),
            anchor: None,
            context: None,
        }];

        cache
            .cache_search_results("test query", Some("test"), results.clone())
            .await;

        let cached = cache
            .get_cached_results("test query", Some("test"))
            .await;
        assert_eq!(cached, Some(results));
    }

    #[tokio::test]
    async fn test_query_cache_analytics() {
        let cache = QueryCache::new();

        // Record some queries
        cache.get("test query", Some("alias")).await;
        cache.get("test query", Some("alias")).await;
        cache.get("another query", None).await;
        
        let stats = cache.stats().await;
        assert_eq!(stats.total_queries, 3);
        assert_eq!(stats.unique_queries, 2);
        assert!(stats.popular_queries.contains(&"test query".to_string()));
    }

    #[tokio::test]
    async fn test_cache_memory_limits() {
        let config = CacheConfig {
            l1_max_memory: 1000, // Very small memory limit
            l2_max_memory: 2000,
            size_fn: |s: &String| s.len(),
            ..Default::default()
        };
        let cache = MultiLevelCache::new(config);
        
        // Add items that exceed memory limit
        let large_value = "x".repeat(500);
        cache.put("key1".to_string(), large_value.clone()).await;
        cache.put("key2".to_string(), large_value.clone()).await;
        cache.put("key3".to_string(), large_value.clone()).await; // Should cause eviction
        
        let stats = cache.stats().await;
        assert!(stats.l1_memory_bytes <= 1000);
        assert!(stats.l2_memory_bytes <= 2000);
    }

    #[test]
    fn test_search_result_size() {
        let results = vec![SearchHit {
            source: "test".to_string(),
            file: "test.md".to_string(),
            heading_path: vec!["Test".to_string()],
            raw_heading_path: Some(vec!["Test".to_string()]),
            level: 1,
            lines: "1-10".to_string(),
            line_numbers: Some(vec![1, 10]),
            snippet: "test snippet".to_string(),
            score: 0.95,
            source_url: Some("https://test.com".to_string()),
            fetched_at: Some(Utc::now()),
            is_stale: false,
            checksum: "abc123".to_string(),
            anchor: None,
            context: None,
        }];

        let size = search_result_size(&results);
        assert!(size > 0);
    }

    #[test]
    fn test_query_analyzer() {
        let mut analyzer = QueryAnalyzer::new();
        
        analyzer.record_query("popular query");
        analyzer.record_query("popular query");
        analyzer.record_query("rare query");
        
        let popular = analyzer.get_popular_queries(2);
        assert_eq!(popular[0], "popular query");
        
        let stats = analyzer.stats();
        assert_eq!(stats.total_queries, 3);
        assert_eq!(stats.unique_queries, 2);
    }
}
