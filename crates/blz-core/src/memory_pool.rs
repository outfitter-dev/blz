// Memory pool for efficient buffer reuse and allocation management
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Memory pool that manages reusable buffers of different sizes
/// 
/// This pool maintains separate queues for different buffer size classes
/// to minimize allocation overhead and memory fragmentation.
pub struct MemoryPool {
    /// Small buffers (< 1KB)
    small_buffers: Arc<Mutex<VecDeque<Vec<u8>>>>,
    
    /// Medium buffers (1KB - 64KB)
    medium_buffers: Arc<Mutex<VecDeque<Vec<u8>>>>,
    
    /// Large buffers (> 64KB)
    large_buffers: Arc<Mutex<VecDeque<Vec<u8>>>>,
    
    /// String buffers for text operations
    string_buffers: Arc<Mutex<VecDeque<String>>>,
    
    /// Maximum number of buffers per pool
    max_buffers_per_pool: usize,
    
    /// Current memory usage in bytes
    current_usage: AtomicUsize,
    
    /// Maximum memory usage in bytes
    max_usage: usize,
    
    /// Statistics
    stats: Arc<MemoryPoolStats>,
}

/// Thread-safe statistics for memory pool
#[derive(Default)]
pub struct MemoryPoolStats {
    pub allocations: AtomicUsize,
    pub deallocations: AtomicUsize,
    pub cache_hits: AtomicUsize,
    pub cache_misses: AtomicUsize,
    pub peak_usage: AtomicUsize,
}

impl MemoryPool {
    /// Create a new memory pool with specified limits
    pub fn new(max_buffers_per_pool: usize, max_memory_mb: usize) -> Self {
        Self {
            small_buffers: Arc::new(Mutex::new(VecDeque::new())),
            medium_buffers: Arc::new(Mutex::new(VecDeque::new())),
            large_buffers: Arc::new(Mutex::new(VecDeque::new())),
            string_buffers: Arc::new(Mutex::new(VecDeque::new())),
            max_buffers_per_pool,
            current_usage: AtomicUsize::new(0),
            max_usage: max_memory_mb * 1024 * 1024,
            stats: Arc::new(MemoryPoolStats::default()),
        }
    }

    /// Get a byte buffer of at least the specified size
    pub async fn get_buffer(&self, min_size: usize) -> PooledBuffer<'_> {
        self.stats.allocations.fetch_add(1, Ordering::Relaxed);

        let pool = match self.classify_size(min_size) {
            BufferSize::Small => &self.small_buffers,
            BufferSize::Medium => &self.medium_buffers,
            BufferSize::Large => &self.large_buffers,
        };

        let mut buffer = {
            let mut pool_guard = pool.lock().await;
            pool_guard.pop_front()
        };

        if let Some(ref mut buf) = buffer {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            
            // Ensure buffer has sufficient capacity
            if buf.capacity() < min_size {
                buf.reserve(min_size - buf.capacity());
            }
            buf.clear();
        } else {
            self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
            buffer = Some(Vec::with_capacity(min_size.max(self.get_default_capacity(min_size))));
        }

        let buffer = buffer.expect("Buffer should be available");
        let capacity = buffer.capacity();

        // Update memory usage
        let new_usage = self.current_usage.fetch_add(capacity, Ordering::Relaxed) + capacity;
        self.update_peak_usage(new_usage);

        PooledBuffer {
            buffer,
            pool: self,
            size_class: self.classify_size(min_size),
            capacity,
        }
    }

    /// Get a string buffer for text operations
    pub async fn get_string_buffer(&self, min_capacity: usize) -> PooledString<'_> {
        self.stats.allocations.fetch_add(1, Ordering::Relaxed);

        let mut buffer = {
            let mut pool_guard = self.string_buffers.lock().await;
            pool_guard.pop_front()
        };

        if let Some(ref mut buf) = buffer {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            
            if buf.capacity() < min_capacity {
                buf.reserve(min_capacity - buf.capacity());
            }
            buf.clear();
        } else {
            self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
            buffer = Some(String::with_capacity(min_capacity.max(256)));
        }

        let buffer = buffer.expect("String buffer should be available");
        let capacity = buffer.capacity();

        // String capacity is in bytes, but we store chars, so estimate
        let estimated_byte_capacity = capacity;
        let new_usage = self.current_usage.fetch_add(estimated_byte_capacity, Ordering::Relaxed) 
                       + estimated_byte_capacity;
        self.update_peak_usage(new_usage);

        PooledString {
            buffer,
            pool: self,
            capacity: estimated_byte_capacity,
        }
    }

    /// Classify buffer size into appropriate pool
    fn classify_size(&self, size: usize) -> BufferSize {
        if size <= 1024 {
            BufferSize::Small
        } else if size <= 65536 {
            BufferSize::Medium
        } else {
            BufferSize::Large
        }
    }

    /// Get default capacity for size class to reduce reallocations
    fn get_default_capacity(&self, min_size: usize) -> usize {
        match self.classify_size(min_size) {
            BufferSize::Small => 1024,      // 1KB
            BufferSize::Medium => 8192,     // 8KB
            BufferSize::Large => 65536,     // 64KB
        }
    }

    /// Return a buffer to the appropriate pool
    async fn return_buffer(&self, mut buffer: Vec<u8>, size_class: BufferSize) {
        self.stats.deallocations.fetch_add(1, Ordering::Relaxed);

        let pool = match size_class {
            BufferSize::Small => &self.small_buffers,
            BufferSize::Medium => &self.medium_buffers,
            BufferSize::Large => &self.large_buffers,
        };

        let mut pool_guard = pool.lock().await;
        
        // Only return to pool if we have space and buffer isn't too large
        if pool_guard.len() < self.max_buffers_per_pool && buffer.capacity() <= self.get_max_buffer_size() {
            buffer.clear(); // Clear contents but keep capacity
            pool_guard.push_back(buffer);
        }
        // Otherwise, let buffer be dropped (freed)
    }

    /// Return a string buffer to the pool
    async fn return_string_buffer(&self, mut buffer: String) {
        self.stats.deallocations.fetch_add(1, Ordering::Relaxed);

        let mut pool_guard = self.string_buffers.lock().await;
        
        // Only return to pool if we have space and buffer isn't too large
        if pool_guard.len() < self.max_buffers_per_pool && buffer.capacity() <= 1_000_000 {
            buffer.clear();
            pool_guard.push_back(buffer);
        }
    }

    /// Maximum size for a buffer to be returned to pool (to prevent memory waste)
    fn get_max_buffer_size(&self) -> usize {
        1_000_000 // 1MB max
    }

    /// Update peak memory usage
    fn update_peak_usage(&self, current: usize) {
        let mut peak = self.stats.peak_usage.load(Ordering::Relaxed);
        while current > peak {
            match self.stats.peak_usage.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }
    }

    /// Get current memory pool statistics
    pub fn get_stats(&self) -> MemoryPoolStatsSummary {
        MemoryPoolStatsSummary {
            allocations: self.stats.allocations.load(Ordering::Relaxed),
            deallocations: self.stats.deallocations.load(Ordering::Relaxed),
            cache_hits: self.stats.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.stats.cache_misses.load(Ordering::Relaxed),
            current_usage_bytes: self.current_usage.load(Ordering::Relaxed),
            peak_usage_bytes: self.stats.peak_usage.load(Ordering::Relaxed),
            hit_rate: {
                let hits = self.stats.cache_hits.load(Ordering::Relaxed);
                let misses = self.stats.cache_misses.load(Ordering::Relaxed);
                if hits + misses > 0 {
                    hits as f64 / (hits + misses) as f64
                } else {
                    0.0
                }
            },
        }
    }

    /// Clear all pools (useful for testing)
    pub async fn clear(&self) {
        let mut small = self.small_buffers.lock().await;
        let mut medium = self.medium_buffers.lock().await;
        let mut large = self.large_buffers.lock().await;
        let mut strings = self.string_buffers.lock().await;

        small.clear();
        medium.clear();
        large.clear();
        strings.clear();

        self.current_usage.store(0, Ordering::Relaxed);
    }

    /// Trim excess buffers to reduce memory usage
    pub async fn trim(&self) {
        let target_count = self.max_buffers_per_pool / 2;

        // Trim each pool to half capacity
        let pools = [&self.small_buffers, &self.medium_buffers, &self.large_buffers];
        
        for pool in &pools {
            let mut pool_guard = pool.lock().await;
            while pool_guard.len() > target_count {
                pool_guard.pop_front();
            }
        }

        // Trim string buffers
        let mut string_pool = self.string_buffers.lock().await;
        while string_pool.len() > target_count {
            string_pool.pop_front();
        }

        debug!("Memory pool trimmed to reduce memory usage");
    }
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new(100, 100) // 100 buffers per pool, 100MB max
    }
}

/// Size classification for buffers
#[derive(Debug, Clone, Copy)]
enum BufferSize {
    Small,  // < 1KB
    Medium, // 1KB - 64KB
    Large,  // > 64KB
}

/// RAII wrapper for pooled byte buffer
pub struct PooledBuffer<'a> {
    buffer: Vec<u8>,
    pool: &'a MemoryPool,
    size_class: BufferSize,
    capacity: usize,
}

impl<'a> PooledBuffer<'a> {
    /// Get mutable access to the buffer
    pub fn as_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    /// Get immutable access to the buffer
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    /// Get the buffer's capacity
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Consume this wrapper and return the inner buffer
    /// 
    /// Note: This prevents the buffer from being returned to the pool
    pub fn into_inner(mut self) -> Vec<u8> {
        // Update memory usage since buffer won't be returned
        self.pool.current_usage.fetch_sub(self.capacity, Ordering::Relaxed);
        
        // Take buffer to prevent drop from returning it to pool
        std::mem::take(&mut self.buffer)
    }
}

impl Drop for PooledBuffer<'_> {
    fn drop(&mut self) {
        // Return buffer to pool
        let buffer = std::mem::take(&mut self.buffer);
        let pool = self.pool;
        let size_class = self.size_class;
        let capacity = self.capacity;

        // Update memory usage
        pool.current_usage.fetch_sub(capacity, Ordering::Relaxed);

        // Spawn task to return buffer (can't await in Drop)
        tokio::spawn(async move {
            pool.return_buffer(buffer, size_class).await;
        });
    }
}

/// RAII wrapper for pooled string buffer
pub struct PooledString<'a> {
    buffer: String,
    pool: &'a MemoryPool,
    capacity: usize,
}

impl<'a> PooledString<'a> {
    /// Get mutable access to the string
    pub fn as_mut(&mut self) -> &mut String {
        &mut self.buffer
    }

    /// Get string slice
    pub fn as_str(&self) -> &str {
        &self.buffer
    }

    /// Get the string's capacity
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Consume wrapper and return inner string
    pub fn into_inner(mut self) -> String {
        // Update memory usage since string won't be returned
        self.pool.current_usage.fetch_sub(self.capacity, Ordering::Relaxed);
        
        std::mem::take(&mut self.buffer)
    }
}

impl Drop for PooledString<'_> {
    fn drop(&mut self) {
        // Return string to pool
        let buffer = std::mem::take(&mut self.buffer);
        let pool = self.pool;
        let capacity = self.capacity;

        // Update memory usage
        pool.current_usage.fetch_sub(capacity, Ordering::Relaxed);

        // Spawn task to return buffer
        tokio::spawn(async move {
            pool.return_string_buffer(buffer).await;
        });
    }
}

/// Summary of memory pool statistics
#[derive(Debug, Clone)]
pub struct MemoryPoolStatsSummary {
    pub allocations: usize,
    pub deallocations: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub current_usage_bytes: usize,
    pub peak_usage_bytes: usize,
    pub hit_rate: f64,
}

/// Arena allocator for temporary allocations within a scope
/// 
/// This allocator is useful for operations that need many small allocations
/// that all have the same lifetime and can be freed together.
pub struct Arena {
    /// Current buffer being allocated from
    current_buffer: Vec<u8>,
    
    /// Position in current buffer
    position: usize,
    
    /// All buffers allocated by this arena
    buffers: Vec<Vec<u8>>,
    
    /// Default buffer size
    buffer_size: usize,
}

impl Arena {
    /// Create a new arena with specified buffer size
    pub fn new(buffer_size: usize) -> Self {
        Self {
            current_buffer: Vec::with_capacity(buffer_size),
            position: 0,
            buffers: Vec::new(),
            buffer_size,
        }
    }

    /// Allocate space for a value of type T
    pub fn alloc<T>(&mut self, value: T) -> &mut T {
        let layout = std::alloc::Layout::new::<T>();
        let ptr = self.alloc_raw(layout);
        
        unsafe {
            let typed_ptr = ptr as *mut T;
            std::ptr::write(typed_ptr, value);
            &mut *typed_ptr
        }
    }

    /// Allocate raw bytes with specified alignment
    pub fn alloc_raw(&mut self, layout: std::alloc::Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        // Align current position
        let aligned_pos = (self.position + align - 1) & !(align - 1);
        
        // Check if we need a new buffer
        if aligned_pos + size > self.current_buffer.capacity() {
            // Move current buffer to storage and create new one
            let old_buffer = std::mem::replace(
                &mut self.current_buffer, 
                Vec::with_capacity(size.max(self.buffer_size))
            );
            
            if !old_buffer.is_empty() {
                self.buffers.push(old_buffer);
            }
            
            self.position = 0;
            let aligned_pos = 0;
        }

        // Allocate from current buffer
        let ptr = unsafe {
            self.current_buffer.as_mut_ptr().add(aligned_pos)
        };
        
        self.position = aligned_pos + size;
        
        // Update buffer length if needed
        if self.position > self.current_buffer.len() {
            unsafe {
                self.current_buffer.set_len(self.position);
            }
        }

        ptr
    }

    /// Get total bytes allocated
    pub fn total_allocated(&self) -> usize {
        self.buffers.iter().map(|b| b.capacity()).sum::<usize>() 
            + self.current_buffer.capacity()
    }

    /// Reset arena, freeing all allocations
    pub fn reset(&mut self) {
        self.buffers.clear();
        self.current_buffer.clear();
        self.position = 0;
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new(8192) // 8KB default buffer size
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        // All allocations are automatically freed when arena is dropped
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_memory_pool_basic() {
        let pool = MemoryPool::new(10, 10);
        
        {
            let mut buffer = pool.get_buffer(100).await;
            buffer.as_mut().extend_from_slice(b"test data");
            assert_eq!(buffer.as_slice(), b"test data");
        } // Buffer returned to pool here
        
        let stats = pool.get_stats();
        assert_eq!(stats.allocations, 1);
        assert_eq!(stats.deallocations, 1);
    }

    #[tokio::test]
    async fn test_memory_pool_reuse() {
        let pool = MemoryPool::new(10, 10);
        
        // Allocate and return a buffer
        {
            let _buffer = pool.get_buffer(100).await;
        }
        
        // Allocate again - should reuse
        {
            let _buffer = pool.get_buffer(100).await;
        }
        
        let stats = pool.get_stats();
        assert!(stats.cache_hits > 0);
    }

    #[tokio::test]
    async fn test_string_pool_basic() {
        let pool = MemoryPool::new(10, 10);
        
        {
            let mut str_buf = pool.get_string_buffer(50).await;
            str_buf.as_mut().push_str("test string");
            assert_eq!(str_buf.as_str(), "test string");
        }
        
        let stats = pool.get_stats();
        assert_eq!(stats.allocations, 1);
    }

    #[tokio::test]
    async fn test_memory_pool_size_classes() {
        let pool = MemoryPool::new(10, 10);
        
        // Test different size classes
        let small = pool.get_buffer(500).await;      // Small
        let medium = pool.get_buffer(5000).await;    // Medium
        let large = pool.get_buffer(100000).await;   // Large
        
        assert!(small.capacity() >= 500);
        assert!(medium.capacity() >= 5000);
        assert!(large.capacity() >= 100000);
    }

    #[tokio::test]
    async fn test_memory_pool_trim() {
        let pool = MemoryPool::new(10, 10);
        
        // Allocate several buffers and return them
        for _ in 0..5 {
            let _buffer = pool.get_buffer(100).await;
        }
        
        pool.trim().await;
        
        let stats = pool.get_stats();
        // Should have trimmed some buffers
        assert!(stats.deallocations > 0);
    }

    #[test]
    fn test_arena_basic() {
        let mut arena = Arena::new(1024);
        
        let value = arena.alloc(42i32);
        assert_eq!(*value, 42);
        
        let str_val = arena.alloc("hello");
        assert_eq!(*str_val, "hello");
    }

    #[test]
    fn test_arena_multiple_buffers() {
        let mut arena = Arena::new(64); // Small buffer to force multiple
        
        // Allocate many values to trigger buffer expansion
        for i in 0..100 {
            let value = arena.alloc(i);
            assert_eq!(*value, i);
        }
        
        assert!(arena.total_allocated() > 64);
    }

    #[test]
    fn test_arena_reset() {
        let mut arena = Arena::new(1024);
        
        for i in 0..10 {
            arena.alloc(i);
        }
        
        let allocated_before = arena.total_allocated();
        assert!(allocated_before > 0);
        
        arena.reset();
        assert_eq!(arena.total_allocated(), 1024); // One buffer remains with capacity
    }

    #[tokio::test]
    async fn test_memory_pool_stats() {
        let pool = MemoryPool::new(10, 10);
        
        {
            let _buf1 = pool.get_buffer(100).await;
            let _buf2 = pool.get_buffer(100).await;
        }
        
        {
            let _buf3 = pool.get_buffer(100).await; // Should be cache hit
        }
        
        let stats = pool.get_stats();
        assert!(stats.hit_rate > 0.0);
        assert!(stats.peak_usage_bytes > 0);
    }
}