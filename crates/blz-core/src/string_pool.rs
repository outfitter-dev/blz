//! String interning and zero-copy helpers.
//!
//! [`StringPool`] interns frequently repeated strings (aliases, headings,
//! field names) to reduce allocation and enable cheap clones via `Arc<str>`.
//! Batch APIs favor fewer lock acquisitions on hot paths.
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// String interner for commonly used strings to reduce memory allocations
/// 
/// This pool maintains a cache of frequently used strings to avoid repeated allocations.
/// Particularly useful for field names, aliases, and commonly occurring terms.
pub struct StringPool {
    /// Interned strings mapped by their content
    strings: RwLock<HashMap<String, Arc<str>>>,
    
    /// Usage counters for eviction policy
    usage_counts: RwLock<HashMap<Arc<str>, usize>>,
    
    /// Maximum number of strings to keep in pool
    max_size: usize,
}

impl StringPool {
    /// Create a new string pool with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            strings: RwLock::new(HashMap::with_capacity(max_size / 2)),
            usage_counts: RwLock::new(HashMap::with_capacity(max_size / 2)),
            max_size,
        }
    }

    /// Intern a string, returning a reference-counted shared string
    /// 
    /// If the string is already interned, returns the existing Arc<str>.
    /// Otherwise, interns the string and returns a new Arc<str>.
    pub async fn intern(&self, s: &str) -> Arc<str> {
        // Fast path: check if already interned
        {
            let strings = self.strings.read().await;
            if let Some(interned) = strings.get(s) {
                // Update usage count
                let mut usage_counts = self.usage_counts.write().await;
                *usage_counts.entry(Arc::clone(interned)).or_insert(0) += 1;
                return Arc::clone(interned);
            }
        }

        // Slow path: need to intern the string
        let mut strings = self.strings.write().await;
        let mut usage_counts = self.usage_counts.write().await;

        // Double-check in case another thread interned while we waited for lock
        if let Some(interned) = strings.get(s) {
            *usage_counts.entry(Arc::clone(interned)).or_insert(0) += 1;
            return Arc::clone(interned);
        }

        // Check if we need to evict
        if strings.len() >= self.max_size {
            self.evict_lru(&mut strings, &mut usage_counts);
        }

        // Intern the new string
        let arc_str: Arc<str> = Arc::from(s);
        strings.insert(s.to_string(), Arc::clone(&arc_str));
        usage_counts.insert(Arc::clone(&arc_str), 1);

        arc_str
    }

    /// Intern multiple strings at once for better batching efficiency
    pub async fn intern_batch(&self, strings: &[&str]) -> Vec<Arc<str>> {
        let mut result = Vec::with_capacity(strings.len());
        
        // First, try to get as many as possible from the read lock
        {
            let pool_strings = self.strings.read().await;
            let mut found = Vec::new();
            let mut missing = Vec::new();

            for (i, s) in strings.iter().enumerate() {
                if let Some(interned) = pool_strings.get(*s) {
                    found.push((i, Arc::clone(interned)));
                } else {
                    missing.push((i, *s));
                }
            }

            result.resize(strings.len(), Arc::from(""));
            
            // Update usage counts for found strings
            if !found.is_empty() {
                let mut usage_counts = self.usage_counts.write().await;
                for (i, interned) in found {
                    *usage_counts.entry(Arc::clone(&interned)).or_insert(0) += 1;
                    result[i] = interned;
                }
            }

            // If we found everything, we're done
            if missing.is_empty() {
                return result;
            }

            // Otherwise, continue with the missing strings
            drop(pool_strings); // Release read lock
        }

        // Handle missing strings with write lock
        let mut pool_strings = self.strings.write().await;
        let mut usage_counts = self.usage_counts.write().await;

        for s in strings.iter() {
            if result.iter().any(|r| r.as_ref() == *s && !r.is_empty()) {
                continue; // Already processed
            }

            let index = strings.iter().position(|&x| x == *s).unwrap();

            // Double-check in case another thread interned while we waited
            if let Some(interned) = pool_strings.get(*s) {
                *usage_counts.entry(Arc::clone(interned)).or_insert(0) += 1;
                result[index] = Arc::clone(interned);
                continue;
            }

            // Check if we need to evict
            if pool_strings.len() >= self.max_size {
                self.evict_lru(&mut pool_strings, &mut usage_counts);
            }

            // Intern the new string
            let arc_str: Arc<str> = Arc::from(*s);
            pool_strings.insert(s.to_string(), Arc::clone(&arc_str));
            usage_counts.insert(Arc::clone(&arc_str), 1);
            result[index] = arc_str;
        }

        result
    }

    /// Evict least recently used strings to make room
    fn evict_lru(
        &self,
        strings: &mut HashMap<String, Arc<str>>,
        usage_counts: &mut HashMap<Arc<str>, usize>,
    ) {
        let target_size = self.max_size * 3 / 4; // Evict to 75% capacity
        let evict_count = strings.len().saturating_sub(target_size);

        if evict_count == 0 {
            return;
        }

        // Find strings with lowest usage counts
        let mut candidates: Vec<_> = usage_counts
            .iter()
            .map(|(arc_str, &count)| (count, Arc::clone(arc_str)))
            .collect();
        
        candidates.sort_by_key(|(count, _)| *count);

        // Evict the least used strings
        for (_, arc_str) in candidates.into_iter().take(evict_count) {
            let key = arc_str.as_ref().to_string();
            strings.remove(&key);
            usage_counts.remove(&arc_str);
        }
    }

    /// Get statistics about the string pool
    pub async fn stats(&self) -> StringPoolStats {
        let strings = self.strings.read().await;
        let usage_counts = self.usage_counts.read().await;

        let total_usage: usize = usage_counts.values().sum();
        let unique_strings = strings.len();
        let total_memory = strings
            .keys()
            .map(|s| s.len())
            .sum::<usize>()
            + unique_strings * std::mem::size_of::<String>()
            + usage_counts.len() * std::mem::size_of::<(Arc<str>, usize)>();

        StringPoolStats {
            unique_strings,
            total_usage,
            memory_bytes: total_memory,
            hit_rate: if total_usage > unique_strings {
                (total_usage - unique_strings) as f64 / total_usage as f64
            } else {
                0.0
            },
        }
    }

    /// Clear all interned strings (useful for testing)
    pub async fn clear(&self) {
        self.strings.write().await.clear();
        self.usage_counts.write().await.clear();
    }
}

impl Default for StringPool {
    fn default() -> Self {
        Self::new(10000) // Default to 10K strings
    }
}

/// Statistics about string pool usage
#[derive(Debug, Clone)]
pub struct StringPoolStats {
    /// Number of unique interned strings.
    pub unique_strings: usize,
    /// Total usage count across all interned strings.
    pub total_usage: usize,
    /// Estimated memory usage in bytes.
    pub memory_bytes: usize,
    /// Cache hit rate as `hits / (hits + misses)`.
    pub hit_rate: f64,
}

/// Zero-copy string operations for search operations.
///
/// These helpers favor `Cow` and preallocated buffers to avoid allocations in
/// tight search loops.
pub struct ZeroCopyStrings;

impl ZeroCopyStrings {
    /// Extract a substring without allocation when possible
    pub fn substring_cow(s: &str, start: usize, len: usize) -> Cow<'_, str> {
        if start == 0 && len >= s.len() {
            // Return the whole string
            Cow::Borrowed(s)
        } else if start + len <= s.len() {
            // Find safe UTF-8 boundaries
            let byte_start = s
                .char_indices()
                .nth(start)
                .map(|(i, _)| i)
                .unwrap_or(s.len());

            let byte_end = s
                .char_indices()
                .nth(start + len)
                .map(|(i, _)| i)
                .unwrap_or(s.len());

            Cow::Borrowed(&s[byte_start..byte_end])
        } else {
            // Fallback to owned string for invalid ranges
            Cow::Owned(
                s.chars()
                    .skip(start)
                    .take(len)
                    .collect::<String>()
            )
        }
    }

    /// Split string efficiently with iterator
    pub fn split_no_alloc(s: &str, delimiter: char) -> impl Iterator<Item = &str> {
        s.split(delimiter)
    }

    /// Join strings with minimal allocations
    pub fn join_with_capacity<'a>(
        strings: impl IntoIterator<Item = &'a str>, 
        delimiter: &str
    ) -> String {
        let iter = strings.into_iter();
        let (size_hint, _) = iter.size_hint();
        
        // Pre-calculate capacity to avoid reallocations
        let mut capacity = size_hint.saturating_sub(1) * delimiter.len();
        capacity += iter.clone().map(|s| s.len()).sum::<usize>();
        
        let mut result = String::with_capacity(capacity);
        let mut first = true;
        
        for s in iter {
            if !first {
                result.push_str(delimiter);
            }
            result.push_str(s);
            first = false;
        }
        
        result
    }

    /// Sanitize query string in single pass with minimal allocations
    pub fn sanitize_query_single_pass(query: &str) -> Cow<'_, str> {
        // First pass: check if we need to escape anything
        let needs_escaping = query.chars().any(|c| {
            matches!(c, '\\' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '~' | ':')
        });

        if !needs_escaping {
            return Cow::Borrowed(query);
        }

        // Second pass: escape characters with pre-allocated capacity
        let mut sanitized = String::with_capacity(query.len() * 2);
        
        for ch in query.chars() {
            match ch {
                '\\' => sanitized.push_str("\\\\"),
                '(' => sanitized.push_str("\\("),
                ')' => sanitized.push_str("\\)"),
                '[' => sanitized.push_str("\\["),
                ']' => sanitized.push_str("\\]"),
                '{' => sanitized.push_str("\\{"),
                '}' => sanitized.push_str("\\}"),
                '^' => sanitized.push_str("\\^"),
                '~' => sanitized.push_str("\\~"),
                ':' => sanitized.push_str("\\:"),
                _ => sanitized.push(ch),
            }
        }
        
        Cow::Owned(sanitized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_string_pool_basic() {
        let pool = StringPool::new(100);
        
        let s1 = pool.intern("test").await;
        let s2 = pool.intern("test").await;
        
        assert_eq!(s1.as_ref(), "test");
        assert_eq!(s2.as_ref(), "test");
        assert!(Arc::ptr_eq(&s1, &s2)); // Should be the same Arc
    }

    #[tokio::test]
    async fn test_string_pool_batch() {
        let pool = StringPool::new(100);
        
        let strings = ["test1", "test2", "test1", "test3"];
        let interned = pool.intern_batch(&strings).await;
        
        assert_eq!(interned.len(), 4);
        assert_eq!(interned[0].as_ref(), "test1");
        assert_eq!(interned[2].as_ref(), "test1");
        assert!(Arc::ptr_eq(&interned[0], &interned[2])); // Same string should be same Arc
    }

    #[tokio::test]
    async fn test_string_pool_eviction() {
        let pool = StringPool::new(3); // Very small pool
        
        // Fill beyond capacity
        let s1 = pool.intern("string1").await;
        let s2 = pool.intern("string2").await;
        let s3 = pool.intern("string3").await;
        let s4 = pool.intern("string4").await; // Should trigger eviction
        
        let stats = pool.stats().await;
        assert!(stats.unique_strings <= 3);
        
        // Use s1 again to increase its usage count
        let s1_again = pool.intern("string1").await;
        
        // s1 might have been evicted, so this could be a new Arc or the same one
        assert_eq!(s1_again.as_ref(), "string1");
    }

    #[tokio::test]
    async fn test_string_pool_stats() {
        let pool = StringPool::new(100);
        
        pool.intern("test1").await;
        pool.intern("test2").await;
        pool.intern("test1").await; // Duplicate
        
        let stats = pool.stats().await;
        assert_eq!(stats.unique_strings, 2);
        assert_eq!(stats.total_usage, 3);
        assert!(stats.hit_rate > 0.0);
    }

    #[test]
    fn test_zero_copy_substring() {
        let s = "hello world";
        
        // Should borrow the whole string
        let sub1 = ZeroCopyStrings::substring_cow(s, 0, 20);
        assert!(matches!(sub1, Cow::Borrowed(_)));
        assert_eq!(sub1, "hello world");
        
        // Should borrow a substring
        let sub2 = ZeroCopyStrings::substring_cow(s, 6, 5);
        assert!(matches!(sub2, Cow::Borrowed(_)));
        assert_eq!(sub2, "world");
    }

    #[test]
    fn test_zero_copy_join() {
        let strings = vec!["hello", "world", "test"];
        let joined = ZeroCopyStrings::join_with_capacity(strings.iter().copied(), " ");
        assert_eq!(joined, "hello world test");
    }

    #[test]
    fn test_sanitize_query_no_escaping() {
        let query = "simple query";
        let sanitized = ZeroCopyStrings::sanitize_query_single_pass(query);
        assert!(matches!(sanitized, Cow::Borrowed(_)));
        assert_eq!(sanitized, "simple query");
    }

    #[test]
    fn test_sanitize_query_with_escaping() {
        let query = "query with (parens) and :colons";
        let sanitized = ZeroCopyStrings::sanitize_query_single_pass(query);
        assert!(matches!(sanitized, Cow::Owned(_)));
        assert_eq!(sanitized, "query with \\(parens\\) and \\:colons");
    }

    #[test]
    fn test_split_no_alloc() {
        let text = "a,b,c,d";
        let parts: Vec<_> = ZeroCopyStrings::split_no_alloc(text, ',').collect();
        assert_eq!(parts, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_unicode_safety() {
        let unicode_text = "Hello 👋 World 🌍!";
        
        // Test substring with Unicode
        let sub = ZeroCopyStrings::substring_cow(unicode_text, 6, 2);
        assert_eq!(sub, "👋 ");
        
        // Test sanitization with Unicode
        let unicode_query = "search 👋 (test)";
        let sanitized = ZeroCopyStrings::sanitize_query_single_pass(unicode_query);
        assert_eq!(sanitized, "search 👋 \\(test\\)");
    }
}
