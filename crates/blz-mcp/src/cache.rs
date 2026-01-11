//! Index cache operations with double-checked locking

use std::sync::Arc;

use blz_core::{SearchIndex, Storage};

use crate::{error::McpResult, types::IndexCache};

/// Get or load an index from cache using a double-checked locking pattern.
///
/// This minimizes contention by allowing concurrent reads while ensuring only
/// one task updates the cache on a miss. Concurrent cache misses may still
/// perform redundant loads; only one result is retained.
///
/// # Errors
///
/// Returns an error if the index directory cannot be resolved or opened.
#[tracing::instrument(skip(cache, storage))]
pub async fn get_or_load_index(
    cache: &IndexCache,
    storage: &Storage,
    source: &str,
) -> McpResult<Arc<SearchIndex>> {
    // Fast path: check if already cached
    {
        let read_lock = cache.read().await;
        if let Some(index) = read_lock.get(source) {
            tracing::debug!(source, "index cache hit");
            return Ok(Arc::clone(index));
        }
    }

    // Slow path: load and cache
    tracing::debug!(source, "index cache miss, loading");
    let index_path = storage.index_dir(source)?;
    let index = SearchIndex::open(&index_path)?;
    let index_arc = Arc::new(index);

    {
        let mut write_lock = cache.write().await;
        // Double-check in case another task loaded it while we were waiting
        if let Some(existing) = write_lock.get(source) {
            tracing::debug!(source, "index loaded by another task");
            return Ok(Arc::clone(existing));
        }
        write_lock.insert(source.to_string(), Arc::clone(&index_arc));
    }

    tracing::debug!(source, "index loaded and cached");
    Ok(index_arc)
}

/// Invalidate cache entry for a source.
///
/// This removes the cached index so the next access reloads it from disk.
#[tracing::instrument(skip(cache))]
pub async fn invalidate_cache(cache: &IndexCache, source: &str) {
    let mut write_lock = cache.write().await;
    if write_lock.remove(source).is_some() {
        tracing::debug!(source, "index cache invalidated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::RwLock;

    /// Helper to create a test index for testing cache behavior
    fn create_test_index() -> Result<(TempDir, SearchIndex), anyhow::Error> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("test_index");
        let index = SearchIndex::create(&index_path)?;
        Ok((temp_dir, index))
    }

    #[tokio::test]
    async fn test_cache_hit_returns_same_arc() {
        // Test the cache hit path: multiple lookups return the same Arc
        let (_temp, index) = create_test_index().expect("Failed to create test index");
        let cache: IndexCache = Arc::new(RwLock::new(HashMap::new()));

        // Pre-populate cache
        let index_arc = Arc::new(index);
        {
            let mut write_lock = cache.write().await;
            write_lock.insert("test-source".to_string(), Arc::clone(&index_arc));
        }

        // First lookup
        let result1 = {
            let read_lock = cache.read().await;
            read_lock.get("test-source").map(Arc::clone)
        };

        // Second lookup
        let result2 = {
            let read_lock = cache.read().await;
            read_lock.get("test-source").map(Arc::clone)
        };

        // Both should return Some and point to the same Arc
        assert!(result1.is_some() && result2.is_some());
        if let (Some(r1), Some(r2)) = (&result1, &result2) {
            assert!(
                Arc::ptr_eq(r1, r2),
                "Cache hits should return the same Arc instance"
            );
        }
    }

    #[tokio::test]
    async fn test_cache_miss_returns_none() {
        // Test the cache miss path: lookup of non-existent key returns None
        let cache: IndexCache = Arc::new(RwLock::new(HashMap::new()));

        let result = {
            let read_lock = cache.read().await;
            read_lock.get("nonexistent").map(Arc::clone)
        };

        assert!(result.is_none(), "Cache miss should return None");
    }

    #[tokio::test]
    async fn test_invalidate_removes_entry() {
        // Test cache invalidation
        let (_temp, index) = create_test_index().expect("Failed to create test index");
        let cache: IndexCache = Arc::new(RwLock::new(HashMap::new()));

        // Pre-populate cache
        {
            let mut write_lock = cache.write().await;
            write_lock.insert("test-source".to_string(), Arc::new(index));
        }

        // Verify it's cached
        assert!(
            cache.read().await.contains_key("test-source"),
            "Entry should be cached"
        );

        // Invalidate
        invalidate_cache(&cache, "test-source").await;

        // Verify it's removed
        assert!(
            !cache.read().await.contains_key("test-source"),
            "Entry should be removed after invalidation"
        );
    }

    #[tokio::test]
    async fn test_concurrent_cache_reads() {
        // Test that concurrent reads of the same entry work correctly
        let (_temp, index) = create_test_index().expect("Failed to create test index");
        let cache: IndexCache = Arc::new(RwLock::new(HashMap::new()));

        // Pre-populate cache
        let index_arc = Arc::new(index);
        {
            let mut write_lock = cache.write().await;
            write_lock.insert("test-source".to_string(), Arc::clone(&index_arc));
        }

        // Spawn multiple concurrent reads
        let cache_clone1 = Arc::clone(&cache);
        let cache_clone2 = Arc::clone(&cache);
        let cache_clone3 = Arc::clone(&cache);

        let handle1 = tokio::spawn(async move {
            let read_lock = cache_clone1.read().await;
            read_lock.get("test-source").map(Arc::clone)
        });
        let handle2 = tokio::spawn(async move {
            let read_lock = cache_clone2.read().await;
            read_lock.get("test-source").map(Arc::clone)
        });
        let handle3 = tokio::spawn(async move {
            let read_lock = cache_clone3.read().await;
            read_lock.get("test-source").map(Arc::clone)
        });

        // Wait for all to complete
        let result1 = handle1.await.expect("Task 1 should complete");
        let result2 = handle2.await.expect("Task 2 should complete");
        let result3 = handle3.await.expect("Task 3 should complete");

        // All should return the same Arc
        assert!(result1.is_some() && result2.is_some() && result3.is_some());
        if let (Some(r1), Some(r2), Some(r3)) = (&result1, &result2, &result3) {
            assert!(
                Arc::ptr_eq(r1, r2) && Arc::ptr_eq(r2, r3),
                "Concurrent reads should return the same Arc instance"
            );
        }
    }

    #[tokio::test]
    async fn test_get_or_load_index_with_nonexistent_source() {
        // Test the error path when trying to load a non-existent source
        let storage = Storage::new().expect("Failed to create storage");
        let cache: IndexCache = Arc::new(RwLock::new(HashMap::new()));

        let result = get_or_load_index(&cache, &storage, "nonexistent-source").await;
        assert!(
            result.is_err(),
            "get_or_load_index should return error for nonexistent source"
        );

        // Cache should remain empty after failed load
        assert!(
            cache.read().await.is_empty(),
            "Cache should be empty after failed load"
        );
    }
}
