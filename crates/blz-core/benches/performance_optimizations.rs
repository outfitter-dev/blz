//! Benchmarks for performance optimizations
//!
//! Note: Some optimizations are placeholders for future features

use blz_core::{HeadingBlock, SearchIndex};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::time::Duration;
use tempfile::TempDir;

/// Create realistic test data for benchmarking
fn create_realistic_blocks(count: usize, content_size: usize) -> Vec<HeadingBlock> {
    let mut blocks = Vec::with_capacity(count);

    let content_templates = [
        "This is documentation about React hooks. useState allows you to add state to functional components.",
        "Component lifecycle methods are essential for React class components. componentDidMount is called after mounting.",
        "TypeScript provides static type checking for JavaScript. Interfaces define the shape of objects.",
        "Performance optimization is crucial for web applications. Use React.memo to prevent unnecessary re-renders.",
        "Database indexing improves query performance significantly. B-tree indexes are most common.",
        "Security best practices include input validation and sanitization. Use HTTPS everywhere.",
        "Async programming patterns help handle concurrent operations. Promises handle asynchronous operations.",
        "Testing strategies ensure code quality and reliability. Unit tests verify individual components.",
    ];

    for i in 0..count {
        let template_index = i % content_templates.len();
        let mut content = String::new();

        // Build content to desired size
        while content.len() < content_size {
            content.push_str(content_templates[template_index]);
            content.push(' ');
        }
        content.truncate(content_size);

        blocks.push(HeadingBlock {
            path: vec![format!("Section_{}", i / 10), format!("Subsection_{i}")],
            content,
            start_line: i * 20 + 1,
            end_line: i * 20 + 15,
        });
    }

    blocks
}

/// Setup a test index with realistic data
fn setup_test_index(blocks: &[HeadingBlock]) -> (TempDir, Arc<SearchIndex>) {
    let temp_dir = TempDir::new().unwrap();
    let index_path = temp_dir.path().join("test_index");

    let index = SearchIndex::create(&index_path).expect("Failed to create index");

    index
        .index_blocks("test_source", "test.md", blocks)
        .expect("Failed to index blocks");

    (temp_dir, Arc::new(index))
}

/// Benchmark basic search operations
fn bench_basic_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("basic_search");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    let blocks = create_realistic_blocks(1000, 500);
    let (_temp_dir, index) = setup_test_index(&blocks);

    let queries = [
        "React hooks",
        "TypeScript interfaces",
        "performance optimization",
        "database indexing",
        "security authentication",
    ];

    for query in &queries {
        group.bench_with_input(BenchmarkId::new("query", query), query, |b, &query| {
            b.iter(|| {
                let _ = index.search(black_box(query), None, black_box(10));
            });
        });
    }

    group.finish();
}

/// Benchmark indexing performance
fn bench_indexing(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexing");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    for block_count in [100, 500, 1000, 2000] {
        let blocks = create_realistic_blocks(block_count, 500);

        group.throughput(Throughput::Elements(block_count as u64));
        group.bench_with_input(
            BenchmarkId::new("blocks", block_count),
            &blocks,
            |b, blocks| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let index_path = temp_dir.path().join("bench_index");

                    let mut index =
                        SearchIndex::create(&index_path).expect("Failed to create index");

                    index
                        .index_blocks("bench_source", "bench.md", black_box(blocks))
                        .expect("Failed to index blocks");
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_basic_search, bench_indexing,);
criterion_main!(benches);
