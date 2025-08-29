//! Benchmarks for search performance

use blz_core::{HeadingBlock, PerformanceMetrics, SearchIndex};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::time::Duration;
use tempfile::TempDir;

// Create realistic test data
fn create_test_blocks(count: usize, content_size: usize) -> Vec<HeadingBlock> {
    let mut blocks = Vec::new();

    let base_content = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                       Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                       Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris \
                       nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor \
                       in reprehenderit in voluptate velit esse cillum dolore eu fugiat \
                       nulla pariatur. Excepteur sint occaecat cupidatat non proident, \
                       sunt in culpa qui officia deserunt mollit anim id est laborum.";

    for i in 0..count {
        let section_name = format!("Section_{}", i % 10); // Simulate sections
        let subsection = format!("Subsection_{i}");

        // Create content of desired size
        let mut content = String::new();
        while content.len() < content_size {
            content.push_str(base_content);
            content.push(' ');
            // Add some keywords that can be searched for
            if i % 5 == 0 {
                content.push_str("React hooks useState useEffect ");
            } else if i % 5 == 1 {
                content.push_str("TypeScript interface generic types ");
            } else if i % 5 == 2 {
                content.push_str("performance optimization cache memory ");
            } else if i % 5 == 3 {
                content.push_str("database query SQL index optimization ");
            } else {
                content.push_str("authentication security JWT tokens ");
            }
        }
        content.truncate(content_size);

        blocks.push(HeadingBlock {
            path: vec![section_name, subsection],
            content,
            start_line: i * 20 + 1,
            end_line: i * 20 + 15,
        });
    }

    blocks
}

fn setup_index_with_blocks(blocks: &[HeadingBlock]) -> (TempDir, SearchIndex) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let index_path = temp_dir.path().join("bench_index");

    let index = SearchIndex::create(&index_path)
        .expect("Failed to create index")
        .with_metrics(PerformanceMetrics::default());

    index
        .index_blocks("bench", "test.md", blocks)
        .expect("Failed to index blocks");

    (temp_dir, index)
}

fn bench_search_scaling(c: &mut Criterion) {
    let block_counts = [10, 50, 100, 500, 1000];
    let content_size = 500; // 500 characters per block

    let mut group = c.benchmark_group("search_scaling");

    for &count in &block_counts {
        let blocks = create_test_blocks(count, content_size);
        let total_bytes = blocks.iter().map(|b| b.content.len()).sum::<usize>();

        group.throughput(Throughput::Bytes(total_bytes as u64));
        group.measurement_time(Duration::from_secs(10));

        let (_temp_dir, index) = setup_index_with_blocks(&blocks);

        group.bench_with_input(BenchmarkId::new("blocks", count), &count, |b, _| {
            b.iter(|| {
                let query = black_box("React hooks");
                index
                    .search(query, Some("bench"), 10)
                    .expect("Search failed")
            });
        });
    }

    group.finish();
}

fn bench_query_complexity(c: &mut Criterion) {
    let blocks = create_test_blocks(100, 1000);
    let (_temp_dir, index) = setup_index_with_blocks(&blocks);

    let mut group = c.benchmark_group("query_complexity");

    let queries = [
        ("simple", "React"),
        ("two_terms", "React hooks"),
        ("three_terms", "React hooks useState"),
        ("complex", "React hooks useState useEffect performance"),
        ("phrase", "\"React hooks\""),
        ("wildcard", "React*"),
    ];

    for (name, query) in &queries {
        group.bench_with_input(BenchmarkId::new("query", name), query, |b, query| {
            b.iter(|| {
                index
                    .search(black_box(query), Some("bench"), 20)
                    .expect("Search failed")
            });
        });
    }

    group.finish();
}

fn bench_result_limits(c: &mut Criterion) {
    let blocks = create_test_blocks(500, 800);
    let (_temp_dir, index) = setup_index_with_blocks(&blocks);

    let mut group = c.benchmark_group("result_limits");

    let limits = [1, 5, 10, 20, 50, 100];

    for &limit in &limits {
        group.bench_with_input(BenchmarkId::new("limit", limit), &limit, |b, &limit| {
            b.iter(|| {
                index
                    .search(black_box("performance"), Some("bench"), limit)
                    .expect("Search failed")
            });
        });
    }

    group.finish();
}

fn bench_content_size_impact(c: &mut Criterion) {
    let content_sizes = [100, 500, 1000, 2000, 5000];

    let mut group = c.benchmark_group("content_size_impact");

    for &size in &content_sizes {
        let blocks = create_test_blocks(100, size);
        let total_bytes = blocks.iter().map(|b| b.content.len()).sum::<usize>();

        group.throughput(Throughput::Bytes(total_bytes as u64));

        let (_temp_dir, index) = setup_index_with_blocks(&blocks);

        group.bench_with_input(BenchmarkId::new("content_size", size), &size, |b, _| {
            b.iter(|| {
                index
                    .search(black_box("authentication"), Some("bench"), 10)
                    .expect("Search failed")
            });
        });
    }

    group.finish();
}

fn bench_index_building(c: &mut Criterion) {
    let block_counts = [10, 50, 100, 500];

    let mut group = c.benchmark_group("index_building");

    for &count in &block_counts {
        let blocks = create_test_blocks(count, 1000);
        let total_bytes = blocks.iter().map(|b| b.content.len()).sum::<usize>();

        group.throughput(Throughput::Bytes(total_bytes as u64));
        group.measurement_time(Duration::from_secs(15));

        group.bench_with_input(BenchmarkId::new("blocks", count), &blocks, |b, blocks| {
            b.iter_with_setup(
                || {
                    let temp_dir = TempDir::new().expect("Failed to create temp dir");
                    let index_path = temp_dir.path().join("bench_index");
                    let index = SearchIndex::create(&index_path).expect("Failed to create index");
                    (temp_dir, index)
                },
                |(temp_dir, mut index)| {
                    index
                        .index_blocks("bench", "test.md", black_box(blocks))
                        .expect("Failed to index blocks");
                    // Keep temp_dir alive
                    drop(temp_dir);
                },
            );
        });
    }

    group.finish();
}

fn bench_realistic_workload(c: &mut Criterion) {
    // Simulate realistic documentation sizes
    let scenarios = [
        ("small_doc", 50, 800),    // Small library docs
        ("medium_doc", 200, 1200), // Medium framework docs
        ("large_doc", 1000, 1500), // Large framework docs like React/Next.js
        ("huge_doc", 5000, 2000),  // Very large docs like Node.js API
    ];

    let mut group = c.benchmark_group("realistic_workload");
    group.measurement_time(Duration::from_secs(20));

    for (name, block_count, content_size) in &scenarios {
        let blocks = create_test_blocks(*block_count, *content_size);
        let total_mb =
            (blocks.iter().map(|b| b.content.len()).sum::<usize>() as f64) / (1024.0 * 1024.0);

        group.throughput(Throughput::Elements(*block_count as u64));

        let (_temp_dir, index) = setup_index_with_blocks(&blocks);

        group.bench_with_input(
            BenchmarkId::new("workload", format!("{}_{}MB", name, total_mb as u32)),
            &(*block_count, *content_size),
            |b, _| {
                b.iter(|| {
                    // Simulate a typical search session with multiple queries
                    let queries = [
                        "React",
                        "hooks useState",
                        "performance",
                        "TypeScript interface",
                    ];
                    for query in &queries {
                        let _results = index
                            .search(black_box(query), Some("bench"), 10)
                            .expect("Search failed");
                    }
                });
            },
        );
    }

    group.finish();
}

// Performance regression tests - ensure we maintain the 6ms target
fn bench_performance_targets(c: &mut Criterion) {
    let blocks = create_test_blocks(100, 1000); // Typical size
    let (_temp_dir, index) = setup_index_with_blocks(&blocks);

    let mut group = c.benchmark_group("performance_targets");
    group.measurement_time(Duration::from_secs(30));

    // Target: <10ms for typical search (we achieve 6ms currently)
    group.bench_function("target_search_10ms", |b| {
        b.iter(|| {
            let result = index
                .search(black_box("React hooks"), Some("bench"), 10)
                .expect("Search failed");
            assert!(!result.is_empty());
        });
    });

    // Target: Multiple queries should still be fast
    group.bench_function("target_multi_search_50ms", |b| {
        b.iter(|| {
            let queries = [
                "React",
                "hooks",
                "TypeScript",
                "performance",
                "authentication",
            ];
            for query in &queries {
                let _result = index
                    .search(black_box(query), Some("bench"), 5)
                    .expect("Search failed");
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_search_scaling,
    bench_query_complexity,
    bench_result_limits,
    bench_content_size_impact,
    bench_index_building,
    bench_realistic_workload,
    bench_performance_targets
);
criterion_main!(benches);
