//! Comprehensive benchmarks for performance optimizations

use blz_core::{
    HeadingBlock,
    PerformanceMetrics,
    SearchHit,
    // Future optimizations - modules not yet implemented:
    // cache::{CacheConfig, MultiLevelCache, SearchCache},
    // memory_pool::MemoryPool,
    // optimized_index::OptimizedSearchIndex,
    // string_pool::StringPool,
    SearchIndex,
};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tempfile::TempDir;
// tokio::runtime::Runtime removed - unused in benchmarks

/// Create realistic test data for benchmarking
fn create_realistic_blocks(count: usize, content_size: usize) -> Vec<HeadingBlock> {
    let mut blocks = Vec::with_capacity(count);

    let content_templates = [
        "This is documentation about React hooks. useState allows you to add state to functional components. It returns an array with the current state value and a setter function. The setter function can accept a new value or a function that receives the previous state.",
        "Component lifecycle methods are essential for React class components. componentDidMount is called after the component is mounted. componentDidUpdate is called after updates. componentWillUnmount is called before the component is removed.",
        "TypeScript provides static type checking for JavaScript. Interfaces define the shape of objects. Generics allow you to write reusable code. Union types allow a value to be one of several types.",
        "Performance optimization is crucial for web applications. Use React.memo to prevent unnecessary re-renders. Implement code splitting with lazy loading. Optimize images and use CDNs for static assets.",
        "Database indexing improves query performance significantly. B-tree indexes are most common. Composite indexes can optimize multi-column queries. Always analyze query execution plans before adding indexes.",
        "Security best practices include input validation and sanitization. Use HTTPS everywhere. Implement proper authentication and authorization. Keep dependencies up to date and scan for vulnerabilities.",
        "Async programming patterns help handle concurrent operations. Promises provide a way to handle asynchronous operations. Async/await syntax makes asynchronous code more readable. Handle errors properly with try/catch blocks.",
        "Testing strategies ensure code quality and reliability. Unit tests verify individual components. Integration tests check component interactions. End-to-end tests validate complete user workflows.",
    ];

    for i in 0..count {
        let template_index = i % content_templates.len();
        let mut content = String::new();

        // Build content to desired size
        while content.len() < content_size {
            content.push_str(content_templates[template_index]);
            content.push(' ');

            // Add searchable keywords
            match i % 8 {
                0 => content.push_str("React hooks useState useEffect "),
                1 => content.push_str("lifecycle methods componentDidMount "),
                2 => content.push_str("TypeScript interfaces generics types "),
                3 => content.push_str("performance optimization memo lazy "),
                4 => content.push_str("database indexing B-tree composite "),
                5 => content.push_str("security authentication HTTPS validation "),
                6 => content.push_str("async promises await concurrent "),
                7 => content.push_str("testing unit integration end-to-end "),
                _ => unreachable!(),
            }
        }

        content.truncate(content_size);

        let section_num = i / 10;
        let section = format!("Section_{section_num}");
        let subsection = format!("Subsection_{i}");

        blocks.push(HeadingBlock {
            path: vec![section, subsection],
            content,
            start_line: i * 20 + 1,
            end_line: i * 20 + 15,
        });
    }

    blocks
}

/// Set up original search index
fn setup_original_index(blocks: &[HeadingBlock]) -> (TempDir, SearchIndex) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let index_path = temp_dir.path().join("original_index");

    let index = SearchIndex::create(&index_path)
        .expect("Failed to create original index")
        .with_metrics(PerformanceMetrics::default());

    index
        .index_blocks("bench", "test.md", blocks)
        .expect("Failed to index blocks");

    (temp_dir, index)
}

// /// Set up optimized search index
// async fn setup_optimized_index(blocks: &[HeadingBlock]) -> (TempDir, OptimizedSearchIndex) {
//     let temp_dir = TempDir::new().expect("Failed to create temp dir");
//     let index_path = temp_dir.path().join("optimized_index");
//
//     let index = OptimizedSearchIndex::create(&index_path)
//         .await
//         .expect("Failed to create optimized index");
//
//     index
//         .index_blocks_optimized("bench", "test.md", blocks)
//         .await
//         .expect("Failed to index blocks");
//
//     (temp_dir, index)
// }

/// Benchmark search performance: Original vs Optimized
fn bench_search_performance_comparison(c: &mut Criterion) {
    let block_counts = [100, 500, 1000, 2000];
    let content_size = 800;

    for &count in &block_counts {
        let blocks = create_realistic_blocks(count, content_size);
        let total_bytes = blocks.iter().map(|b| b.content.len()).sum::<usize>();

        // Setup indices
        let (_temp_dir_orig, original_index) = setup_original_index(&blocks);
        // TODO: Uncomment when optimized implementation is ready
        // let (_temp_dir_opt, optimized_index) = rt.block_on(setup_optimized_index(&blocks));

        let mut group = c.benchmark_group(format!("search_performance_{}_docs", count));
        group.throughput(Throughput::Bytes(total_bytes as u64));
        group.measurement_time(Duration::from_secs(15));

        // Benchmark original implementation
        group.bench_function("original", |b| {
            b.iter(|| {
                let query = black_box("React hooks");
                let results = original_index
                    .search(query, Some("bench"), 10)
                    .expect("Search failed");
                black_box(results);
            });
        });

        // TODO: Uncomment when optimized implementation is ready
        // // Benchmark optimized implementation
        // group.bench_function("optimized", |b| {
        //     b.iter(|| {
        //         rt.block_on(async {
        //             let query = black_box("React hooks");
        //             optimized_index
        //                 .search_optimized(query, Some("bench"), 10)
        //                 .await
        //                 .expect("Search failed")
        //         })
        //     });
        // });

        group.finish();
    }
}

/// Benchmark string operations: Regular vs Zero-Copy
fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    let test_strings = vec![
        "simple query",
        "query with (parentheses) and \"quotes\"",
        "complex query with [brackets] {braces} and ^special~ chars",
        "very long query that would require multiple allocations in the regular case because it contains many special characters like () [] {} ^~ and \"quotes\"",
    ];

    for (i, test_str) in test_strings.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("regular_sanitization", i),
            test_str,
            |b, &test_str| {
                b.iter(|| {
                    // Regular implementation with multiple allocations
                    let mut sanitized = String::new();
                    for ch in test_str.chars() {
                        match ch {
                            '\\' => sanitized.push_str("\\\\"),
                            '"' => sanitized.push_str("\\\""),
                            '(' => sanitized.push_str("\\("),
                            ')' => sanitized.push_str("\\)"),
                            '[' => sanitized.push_str("\\["),
                            ']' => sanitized.push_str("\\]"),
                            '{' => sanitized.push_str("\\{"),
                            '}' => sanitized.push_str("\\}"),
                            '^' => sanitized.push_str("\\^"),
                            '~' => sanitized.push_str("\\~"),
                            _ => sanitized.push(ch),
                        }
                    }
                    black_box(sanitized)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("zero_copy_sanitization", i),
            test_str,
            |b, &test_str| {
                b.iter(|| {
                    // TODO: Uncomment when string_pool module is implemented
                    // use blz_core::string_pool::ZeroCopyStrings;
                    // black_box(ZeroCopyStrings::sanitize_query_single_pass(test_str))
                    black_box(test_str)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory pool vs regular allocation
fn bench_memory_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    let buffer_sizes = [64, 256, 1024, 4096, 16384];

    for &size in &buffer_sizes {
        group.throughput(Throughput::Bytes(size as u64));

        // Regular allocation
        group.bench_with_input(
            BenchmarkId::new("regular_allocation", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let mut buffer = Vec::with_capacity(size);
                    buffer.resize(size, 0u8);
                    buffer.clear();
                    black_box(buffer)
                });
            },
        );

        // Memory pool allocation
        // TODO: Uncomment when MemoryPool is implemented
        // let pool = MemoryPool::default();
        /* group.bench_with_input(
            BenchmarkId::new("pool_allocation", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut buffer = pool.get_buffer(size).await;
                        buffer.as_mut().resize(size, 0u8);
                        buffer.as_mut().clear();
                        black_box(())
                    })
                });
            },
        ); */
    }

    group.finish();
}

/// Benchmark string interning performance
fn bench_string_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_interning");

    let test_strings = vec![
        "alias1",
        "alias2",
        "alias3",
        "alias1",
        "alias2", // Repeated strings
        "file1.md",
        "file2.md",
        "file1.md",
        "file3.md",
        "file1.md",
        "React",
        "TypeScript",
        "React",
        "JavaScript",
        "React",
    ];

    // Regular string operations (cloning)
    group.bench_function("regular_strings", |b| {
        b.iter(|| {
            let mut stored_strings = Vec::new();
            for s in &test_strings {
                stored_strings.push(s.to_string()); // Always allocate new string
            }
            black_box(stored_strings)
        });
    });

    // String interning
    // TODO: Uncomment when StringPool is implemented
    // let pool = StringPool::default();
    // group.bench_function("interned_strings", |b| {
    //     b.iter(|| {
    //         rt.block_on(async {
    //             let mut interned_strings = Vec::new();
    //             for s in &test_strings {
    //                 interned_strings.push(pool.intern(s).await);
    //             }
    //             black_box(interned_strings)
    //         })
    //     });
    // });

    // Batch interning
    // TODO: Uncomment when StringPool is implemented
    // group.bench_function("batch_interned_strings", |b| {
    //     b.iter(|| {
    //         rt.block_on(async {
    //             let string_refs: Vec<&str> = test_strings.iter().copied().collect();
    //             let interned = pool.intern_batch(&string_refs).await;
    //             black_box(interned)
    //         })
    //     });
    // });

    group.finish();
}

/// Benchmark caching strategies
fn bench_caching_strategies(c: &mut Criterion) {
    let mut group = c.benchmark_group("caching_strategies");
    group.measurement_time(Duration::from_secs(10));

    // Create test search results
    let create_search_results = |count: usize| -> Vec<SearchHit> {
        (0..count)
            .map(|i| SearchHit {
                alias: format!("alias_{}", i % 5),
                file: format!("file_{}.md", i % 10),
                heading_path: vec![format!("Section_{}", i), format!("Subsection_{}", i)],
                lines: format!("{}-{}", i * 10, i * 10 + 5),
                snippet: format!("This is test content for result {}", i),
                score: (0.95 - (i as f32 * 0.01)).max(0.0),
                source_url: Some(format!("https://example.com/{}", i)),
                checksum: format!("checksum_{}", i),
            })
            .collect()
    };

    let result_counts = [10, 50, 100];

    for &count in &result_counts {
        // No caching - always recreate results
        group.bench_with_input(BenchmarkId::new("no_cache", count), &count, |b, &count| {
            b.iter(|| {
                let results = create_search_results(count);
                black_box(results)
            });
        });

        // TODO: Multi-level cache (not yet implemented)
        // let cache = SearchCache::new_search_cache();
        // group.bench_with_input(
        //     BenchmarkId::new("multi_level_cache", count),
        //     &results,
        //     |b, results| {
        //         b.to_async(&rt).iter(|| async {
        //             // Cache results
        //             cache
        //                 .cache_search_results("test_query", Some("test"), results.clone())
        //                 .await;
        //             // Retrieve from cache
        //             let cached = cache.get_cached_results("test_query", Some("test")).await;
        //             black_box(cached)
        //         });
        //     },
        // );
    }

    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");
    group.measurement_time(Duration::from_secs(20));

    let _blocks = create_realistic_blocks(500, 1000);

    let concurrent_levels = [1, 2, 4, 8, 16];

    for &concurrency in &concurrent_levels {
        group.bench_with_input(
            BenchmarkId::new("concurrent_searches", concurrency),
            &concurrency,
            |b, &concurrency| {
                let queries: Vec<String> = (0..concurrency)
                    .map(|i| {
                        match i % 4 {
                            0 => "React hooks",
                            1 => "TypeScript interfaces",
                            2 => "performance optimization",
                            3 => "testing strategies",
                            _ => unreachable!(),
                        }
                        .to_string()
                    })
                    .collect();

                b.iter(|| {
                    // Simulate per-query CPU work deterministically
                    for query in &queries {
                        let mut hasher = DefaultHasher::new();
                        query.hash(&mut hasher);
                        let h = hasher.finish();
                        black_box(h);
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark indexing performance: Original vs Optimized
fn bench_indexing_performance(c: &mut Criterion) {
    let block_counts = [50, 100, 250, 500];
    let content_size = 1000;

    for &count in &block_counts {
        let blocks = create_realistic_blocks(count, content_size);
        let total_bytes = blocks.iter().map(|b| b.content.len()).sum::<usize>();

        let mut group = c.benchmark_group(format!("indexing_performance_{}_docs", count));
        group.throughput(Throughput::Bytes(total_bytes as u64));
        group.measurement_time(Duration::from_secs(15));

        // Original indexing
        group.bench_function("original_indexing", |b| {
            b.iter_with_setup(
                || {
                    let temp_dir = TempDir::new().expect("Failed to create temp dir");
                    let index_path = temp_dir.path().join("original_index");
                    let index = SearchIndex::create(&index_path).expect("Failed to create index");
                    (temp_dir, index)
                },
                |(temp_dir, index)| {
                    index
                        .index_blocks("bench", "test.md", black_box(&blocks))
                        .expect("Failed to index blocks");
                    drop(temp_dir);
                },
            );
        });

        // TODO: Optimized indexing - uncomment when OptimizedSearchIndex is available
        // group.bench_function("optimized_indexing", |b| {
        //     b.iter_with_setup(
        //         || {
        //             let temp_dir = TempDir::new().expect("Failed to create temp dir");
        //             let index_path = temp_dir.path().join("optimized_index");
        //             // TODO: Use OptimizedSearchIndex when available
        //             let index = SearchIndex::create(&index_path)
        //                 .expect("Failed to create index");
        //             (temp_dir, index)
        //         },
        //         |(temp_dir, mut index)| {
        //             for block in &blocks {
        //                 index.add_document(block);
        //             }
        //             drop(temp_dir);
        //         },
        //     );
        // });

        group.finish();
    }
}

/// Benchmark snippet extraction optimizations
fn bench_snippet_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("snippet_extraction");

    let test_content = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. React hooks provide a way to use state and lifecycle methods in functional components. useState returns an array with the current state value and a setter function. The setter function can be used to update the state. When state changes, the component re-renders automatically.";
    let query = "React hooks";

    // Regular snippet extraction
    group.bench_function("regular_extraction", |b| {
        b.iter(|| {
            let query_lower = query.to_lowercase();
            let content_lower = test_content.to_lowercase();

            if let Some(pos) = content_lower.find(&query_lower) {
                let context_before = 50;
                let context_after = 50;
                let start = pos.saturating_sub(context_before);
                let end = (pos + query.len() + context_after).min(test_content.len());

                let mut snippet = String::new();
                if start > 0 {
                    snippet.push_str("...");
                }
                snippet.push_str(&test_content[start..end]);
                if end < test_content.len() {
                    snippet.push_str("...");
                }
                black_box(snippet)
            } else {
                black_box(String::new())
            }
        });
    });

    // Optimized snippet extraction (would use the optimized version from the index)
    group.bench_function("optimized_extraction", |b| {
        b.iter(|| {
            let query_lower = query.to_lowercase();
            let content_lower = test_content.to_lowercase();

            if let Some(pos) = content_lower.find(&query_lower) {
                let context_before = 50;
                let context_after = 50;

                // Find safe UTF-8 boundaries
                let byte_start = pos.saturating_sub(context_before);
                let byte_end = (pos + query.len() + context_after).min(test_content.len());

                let start = test_content
                    .char_indices()
                    .take_while(|(i, _)| *i <= byte_start)
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);

                let end = test_content
                    .char_indices()
                    .find(|(i, _)| *i >= byte_end)
                    .map(|(i, _)| i)
                    .unwrap_or(test_content.len());

                let mut snippet = String::with_capacity(end - start + 6);
                if start > 0 {
                    snippet.push_str("...");
                }
                snippet.push_str(&test_content[start..end]);
                if end < test_content.len() {
                    snippet.push_str("...");
                }

                black_box(snippet)
            } else {
                black_box(String::new())
            }
        });
    });

    group.finish();
}

/// Benchmark cache hit rates and effectiveness
fn bench_cache_effectiveness(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_effectiveness");
    group.measurement_time(Duration::from_secs(15));

    let _blocks = create_realistic_blocks(200, 800);

    // Simulate realistic query patterns with repetition
    let query_patterns = vec![
        "React hooks",
        "React hooks",
        "React hooks", // High frequency
        "TypeScript interfaces",
        "TypeScript interfaces",    // Medium frequency
        "performance optimization", // Low frequency
        "testing strategies",
        "testing strategies",
        "database indexing",
        "React hooks", // Repeat high frequency
        "security authentication",
        "TypeScript interfaces", // Repeat medium frequency
    ];

    group.bench_function("cache_effectiveness_simulation", |b| {
        b.iter(|| {
            let mut total_time = Duration::ZERO;

            for query in &query_patterns {
                let start = std::time::Instant::now();
                // TODO: Use optimized_index.search_optimized when available
                // For now, simulate work deterministically without sleeping
                let work: usize = query.chars().map(|ch| (ch as u32 % 13) as usize).sum();
                black_box(work);
                total_time += start.elapsed();
            }

            black_box(total_time)
        });
    });

    group.finish();
}

criterion_group!(
    performance_benchmarks,
    bench_search_performance_comparison,
    bench_string_operations,
    bench_memory_pool,
    bench_string_interning,
    bench_caching_strategies,
    bench_concurrent_operations,
    bench_indexing_performance,
    bench_snippet_extraction,
    bench_cache_effectiveness
);

criterion_main!(performance_benchmarks);
