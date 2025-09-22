// Integration example showing how to use the performance optimizations
#![allow(dead_code, unused_imports)] // This is an example file

use crate::{
    cache::{CacheConfig, SearchCache},
    memory_pool::MemoryPool,
    optimized_index::OptimizedSearchIndex,
    string_pool::StringPool,
    async_io::{ConnectionPool, AsyncFileOps},
    HeadingBlock, Result,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

/// Example of a high-performance search system using all optimizations
pub struct HighPerformanceSearchSystem {
    /// Optimized search index with all performance features
    index: OptimizedSearchIndex,
    
    /// Connection pool for HTTP operations
    connection_pool: Arc<ConnectionPool>,
    
    /// Memory pool for buffer reuse
    memory_pool: Arc<MemoryPool>,
    
    /// String pool for interning common strings
    string_pool: Arc<StringPool>,
}

impl HighPerformanceSearchSystem {
    /// Create a new high-performance search system
    pub async fn new(index_path: &std::path::Path) -> Result<Self> {
        // Create optimized search index
        let index = OptimizedSearchIndex::create(index_path).await?;
        
        // Create connection pool with optimized settings
        let connection_pool = Arc::new(ConnectionPool::new(50, 10)?); // 50 total, 10 per domain
        
        // Create memory pool with reasonable limits
        let memory_pool = Arc::new(MemoryPool::new(200, 100)); // 200 buffers, 100MB max
        
        // Create string pool for common strings
        let string_pool = Arc::new(StringPool::new(5000)); // 5000 unique strings max
        
        Ok(Self {
            index,
            connection_pool,
            memory_pool,
            string_pool,
        })
    }
    
    /// Index documents from a URL with full optimization pipeline
    pub async fn index_from_url(&self, alias: &str, url: &str) -> Result<()> {
        info!("Starting optimized indexing for alias: {}", alias);
        
        // Fetch content using optimized HTTP client
        let content = self.connection_pool.get(url).await?;
        
        // Parse content into blocks (simplified for example)
        let blocks = self.parse_content_optimized(&content).await?;
        
        // Index with all optimizations enabled
        self.index.index_blocks_optimized(alias, url, &blocks).await?;
        
        info!("Completed indexing {} blocks for {}", blocks.len(), alias);
        Ok(())
    }
    
    /// Perform optimized search with full pipeline
    pub async fn search(
        &self,
        query: &str,
        alias: Option<&str>,
        flavor: Option<&str>,
        limit: usize,
    ) -> Result<Vec<crate::SearchHit>> {
        // Use the fully optimized search pipeline
        self.index
            .search_optimized(query, alias, flavor, limit)
            .await
    }
    
    /// Batch index multiple sources concurrently
    pub async fn index_multiple_sources(&self, sources: &[(String, String)]) -> Result<()> {
        let mut index_tasks = Vec::new();
        
        for (alias, url) in sources {
            let task = self.index_from_url(alias, url);
            index_tasks.push(task);
        }
        
        // Use the optimized parallel indexing
        futures::future::try_join_all(index_tasks).await?;
        Ok(())
    }
    
    /// Warm up the system with common queries
    pub async fn warm_up(
        &self,
        common_queries: &[(&str, Option<&str>, Option<&str>)],
    ) -> Result<()> {
        info!("Warming up system with {} queries", common_queries.len());
        self.index.warm_up(common_queries).await
    }
    
    /// Get comprehensive performance statistics
    pub async fn get_performance_stats(&self) -> PerformanceStatsSummary {
        let index_stats = self.index.get_stats().await;
        let connection_stats = self.connection_pool.get_stats();
        let memory_stats = self.memory_pool.get_stats();
        let string_stats = self.string_pool.stats().await;
        
        PerformanceStatsSummary {
            searches_total: index_stats.searches,
            avg_search_time_ms: index_stats.avg_search_time_ms,
            cache_hit_rate: index_stats.cache_hit_rate,
            memory_pool_hit_rate: memory_stats.hit_rate,
            string_pool_hit_rate: string_stats.hit_rate,
            connection_pool_hit_rate: connection_stats.hit_rate,
            total_memory_mb: memory_stats.current_usage_bytes / (1024 * 1024),
            peak_memory_mb: memory_stats.peak_usage_bytes / (1024 * 1024),
        }
    }
    
    /// Example of parsing content with optimization
    async fn parse_content_optimized(&self, content: &str) -> Result<Vec<HeadingBlock>> {
        // Use pooled string buffer for parsing
        let mut parse_buffer = self.memory_pool.get_string_buffer(content.len()).await;
        
        // Simplified parsing logic (in real implementation, this would use the markdown parser)
        let mut blocks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        let mut current_section = Vec::new();
        let mut line_num = 1;
        
        for line in lines {
            if line.starts_with('#') {
                // Process previous section if it exists
                if !current_section.is_empty() {
                    parse_buffer.as_mut().clear();
                    for section_line in &current_section {
                        parse_buffer.as_mut().push_str(section_line);
                        parse_buffer.as_mut().push('\n');
                    }
                    
                    blocks.push(HeadingBlock {
                        path: vec!["Section".to_string()],
                        content: parse_buffer.as_str().to_string(),
                        start_line: line_num - current_section.len(),
                        end_line: line_num - 1,
                    });
                    
                    current_section.clear();
                }
            } else if !line.trim().is_empty() {
                current_section.push(line);
            }
            line_num += 1;
        }
        
        // Handle final section
        if !current_section.is_empty() {
            parse_buffer.as_mut().clear();
            for section_line in &current_section {
                parse_buffer.as_mut().push_str(section_line);
                parse_buffer.as_mut().push('\n');
            }
            
            blocks.push(HeadingBlock {
                path: vec!["Section".to_string()],
                content: parse_buffer.as_str().to_string(),
                start_line: line_num - current_section.len(),
                end_line: line_num - 1,
            });
        }
        
        Ok(blocks)
    }
}

/// Summary of performance statistics across all components
#[derive(Debug, Clone)]
pub struct PerformanceStatsSummary {
    pub searches_total: usize,
    pub avg_search_time_ms: usize,
    pub cache_hit_rate: f64,
    pub memory_pool_hit_rate: f64,
    pub string_pool_hit_rate: f64,
    pub connection_pool_hit_rate: f64,
    pub total_memory_mb: usize,
    pub peak_memory_mb: usize,
}

impl PerformanceStatsSummary {
    /// Check if performance is within acceptable thresholds
    pub fn is_healthy(&self) -> bool {
        self.avg_search_time_ms < 20 &&       // < 20ms average search time
        self.cache_hit_rate > 0.75 &&         // > 75% cache hit rate
        self.memory_pool_hit_rate > 0.70 &&   // > 70% memory pool efficiency
        self.total_memory_mb < 500             // < 500MB memory usage
    }
    
    /// Get performance grade (A-F)
    pub fn get_performance_grade(&self) -> char {
        let mut score = 0;
        
        // Search time (0-25 points)
        score += match self.avg_search_time_ms {
            0..=5 => 25,
            6..=10 => 20,
            11..=15 => 15,
            16..=20 => 10,
            21..=30 => 5,
            _ => 0,
        };
        
        // Cache hit rate (0-25 points)
        score += (self.cache_hit_rate * 25.0) as i32;
        
        // Memory efficiency (0-25 points)
        score += (self.memory_pool_hit_rate * 25.0) as i32;
        
        // Memory usage (0-25 points)
        score += match self.total_memory_mb {
            0..=50 => 25,
            51..=100 => 20,
            101..=200 => 15,
            201..=300 => 10,
            301..=500 => 5,
            _ => 0,
        };
        
        match score {
            90..=100 => 'A',
            80..=89 => 'B',
            70..=79 => 'C',
            60..=69 => 'D',
            _ => 'F',
        }
    }
}

/// Example usage function
#[allow(dead_code)]
async fn example_usage() -> Result<()> {
    // Create high-performance search system
    let search_system = HighPerformanceSearchSystem::new(
        std::path::Path::new("./optimized_index")
    ).await?;
    
    // Index multiple sources concurrently
    let sources = vec![
        ("react".to_string(), "https://react.dev/llms.txt".to_string()),
        ("typescript".to_string(), "https://typescriptlang.org/llms.txt".to_string()),
    ];
    
    search_system.index_multiple_sources(&sources).await?;
    
    // Warm up with common queries
    let common_queries = &[
        ("React hooks", Some("react"), None),
        ("TypeScript interfaces", Some("typescript"), None),
        ("useState", Some("react"), None),
    ];
    
    search_system.warm_up(common_queries).await?;
    
    // Perform optimized searches
    let results = search_system.search("React hooks useState", Some("react"), 10).await?;
    info!("Found {} results for React hooks search", results.len());
    
    // Check performance statistics
    let stats = search_system.get_performance_stats().await;
    info!("System performance grade: {}", stats.get_performance_grade());
    info!("Average search time: {}ms", stats.avg_search_time_ms);
    info!("Cache hit rate: {:.1}%", stats.cache_hit_rate * 100.0);
    
    if stats.is_healthy() {
        info!("✅ System performance is healthy");
    } else {
        info!("⚠️ System performance needs attention");
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio_test;
    
    #[tokio::test]
    async fn test_high_performance_system_creation() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index");
        
        let result = HighPerformanceSearchSystem::new(&index_path).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_performance_stats_grading() {
        let excellent_stats = PerformanceStatsSummary {
            searches_total: 1000,
            avg_search_time_ms: 5,
            cache_hit_rate: 0.95,
            memory_pool_hit_rate: 0.90,
            string_pool_hit_rate: 0.85,
            connection_pool_hit_rate: 0.80,
            total_memory_mb: 30,
            peak_memory_mb: 45,
        };
        
        assert_eq!(excellent_stats.get_performance_grade(), 'A');
        assert!(excellent_stats.is_healthy());
        
        let poor_stats = PerformanceStatsSummary {
            searches_total: 100,
            avg_search_time_ms: 50,
            cache_hit_rate: 0.40,
            memory_pool_hit_rate: 0.30,
            string_pool_hit_rate: 0.20,
            connection_pool_hit_rate: 0.10,
            total_memory_mb: 600,
            peak_memory_mb: 800,
        };
        
        assert_eq!(poor_stats.get_performance_grade(), 'F');
        assert!(!poor_stats.is_healthy());
    }
    
    #[tokio::test]
    async fn test_content_parsing_optimization() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index");
        
        let system = HighPerformanceSearchSystem::new(&index_path).await.unwrap();
        
        let test_content = "# Heading 1\nContent for section 1\n\n# Heading 2\nContent for section 2";
        let blocks = system.parse_content_optimized(test_content).await.unwrap();
        
        assert!(!blocks.is_empty());
        assert!(blocks.iter().any(|b| b.content.contains("section 1")));
        assert!(blocks.iter().any(|b| b.content.contains("section 2")));
    }
}
