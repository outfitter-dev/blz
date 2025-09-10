//! Benchmarks for registry search performance
//!
//! Tests search performance under various conditions including:
//! - Different query sizes
//! - Large registries
//! - Fuzzy matching performance
//! - Concurrent search operations

use blz_core::Registry;
use blz_core::registry::RegistryEntry;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::sync::Arc;
// use std::time::Duration;  // Unused import

/// Create a small test registry matching the default size
fn create_small_registry() -> Registry {
    Registry::new()
}

/// Create a large test registry with many entries
fn create_large_registry() -> Registry {
    let mut entries = Vec::new();

    // Add the default entries
    let default_registry = Registry::new();
    entries.extend(default_registry.all_entries().iter().cloned());

    // Add many synthetic entries to test performance
    for i in 0..1000 {
        let entry = RegistryEntry::new(
            &format!("Framework {i}"),
            &format!("framework-{i}"),
            &format!("A synthetic framework for testing performance - entry {i}"),
            &format!("https://framework-{i}.com/llms.txt"),
        )
        .with_aliases(&[
            &format!("framework-{i}"),
            &format!("fw{i}"),
            &format!("f{i}"),
        ]);
        entries.push(entry);
    }

    // Add entries with similar names to test fuzzy matching performance
    let similar_names = vec![
        "react",
        "reactor",
        "reactive",
        "react-dom",
        "react-native",
        "angular",
        "angularjs",
        "angular2",
        "angular-cli",
        "angular-core",
        "vue",
        "vuejs",
        "vue-cli",
        "vue-router",
        "vuex",
        "node",
        "nodejs",
        "nodemon",
        "node-sass",
        "node-fetch",
        "javascript",
        "js",
        "jsdom",
        "json",
        "jest",
        "typescript",
        "ts",
        "ts-node",
        "tsc",
        "tsconfig",
        "rust",
        "rustc",
        "rustup",
        "rust-analyzer",
        "rusty",
        "python",
        "py",
        "python3",
        "pip",
        "pipenv",
        "poetry",
        "go",
        "golang",
        "gofmt",
        "go-mod",
        "go-test",
    ];

    for (i, name) in similar_names.iter().enumerate() {
        let alt_name = format!("{}-alt", name);
        let entry = RegistryEntry::new(
            &format!("{} Framework", name.to_uppercase()),
            &format!("similar-{}", i),
            &format!("Similar name testing framework: {}", name),
            &format!("https://{}.example.com/llms.txt", name),
        )
        .with_aliases(&[name, alt_name.as_str()]);
        entries.push(entry);
    }

    // Create registry with all the synthetic entries
    Registry::from_entries(entries)
}

fn bench_search_query_sizes(c: &mut Criterion) {
    let registry = create_small_registry();

    let mut group = c.benchmark_group("search_query_sizes");

    let query_sizes = vec![
        (1, "r"),
        (5, "react"),
        (10, "javascript"),
        (20, "javascript framework"),
        (
            50,
            "modern javascript framework for building user interfaces",
        ),
        (
            100,
            "a very long search query that contains many words to test the performance of the fuzzy matching algorithm when dealing with large query strings that might be encountered in real world usage",
        ),
    ];

    for (size, query) in query_sizes {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::new("query_size", size), &query, |b, query| {
            b.iter(|| {
                let results = registry.search(black_box(query));
                black_box(results)
            })
        });
    }

    group.finish();
}

fn bench_search_result_sizes(c: &mut Criterion) {
    let registry = create_small_registry();

    let mut group = c.benchmark_group("search_result_sizes");

    // Different queries that return different numbers of results
    let queries = vec![
        ("no_results", "nonexistentframeworkxyz123"),
        ("few_results", "claude"),
        ("many_results", "javascript"),
        ("all_results", ""),
    ];

    for (name, query) in queries {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::new("result_size", name), &query, |b, query| {
            b.iter(|| {
                let results = registry.search(black_box(query));
                black_box(results)
            })
        });
    }

    group.finish();
}

fn bench_fuzzy_matching_types(c: &mut Criterion) {
    let registry = create_small_registry();

    let mut group = c.benchmark_group("fuzzy_matching_types");

    let fuzzy_cases = vec![
        ("exact_match", "react"),
        ("case_mismatch", "REACT"),
        ("typo_single", "reac"),
        ("typo_double", "raect"),
        ("missing_char", "react"),
        ("extra_char", "reactt"),
        ("partial_match", "rea"),
        ("alias_match", "reactjs"),
        ("description_match", "javascript library"),
        ("no_match", "completelydifferent"),
    ];

    for (case_name, query) in fuzzy_cases {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(
            BenchmarkId::new("fuzzy_type", case_name),
            &query,
            |b, query| {
                b.iter(|| {
                    let results = registry.search(black_box(query));
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

fn bench_concurrent_searches(c: &mut Criterion) {
    let registry = Arc::new(create_small_registry());

    let mut group = c.benchmark_group("concurrent_searches");

    let concurrency_levels = vec![1, 2, 4, 8, 16];
    let queries = vec![
        "react",
        "node",
        "vue",
        "angular",
        "javascript",
        "typescript",
        "python",
        "rust",
    ];

    for concurrency in concurrency_levels {
        group.throughput(Throughput::Elements(concurrency as u64));

        group.bench_with_input(
            BenchmarkId::new("concurrent", concurrency),
            &concurrency,
            |b, &concurrency| {
                let rt = tokio::runtime::Runtime::new().unwrap();

                b.to_async(&rt).iter(|| async {
                    let registry = Arc::clone(&registry);
                    let mut handles = Vec::new();

                    for i in 0..concurrency {
                        let query = queries[i % queries.len()];
                        let registry_clone = Arc::clone(&registry);

                        handles.push(tokio::spawn(async move {
                            registry_clone.search(black_box(query))
                        }));
                    }

                    let results: Vec<_> = futures::future::join_all(handles)
                        .await
                        .into_iter()
                        .map(|r| r.unwrap())
                        .collect();

                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

fn bench_search_patterns(c: &mut Criterion) {
    let registry = create_small_registry();

    let mut group = c.benchmark_group("search_patterns");

    // Test different search patterns
    let patterns = vec![
        ("single_word", "react"),
        ("multi_word", "javascript runtime"),
        ("partial_word", "java"),
        ("with_punctuation", "node.js"),
        ("with_numbers", "vue3"),
        ("camel_case", "JavaScript"),
        ("snake_case", "next_js"),
        ("kebab_case", "vue-js"),
        ("mixed_case", "ReAcT"),
        ("unicode", "日本語"), // This might not match anything, testing Unicode handling
    ];

    for (pattern_name, query) in patterns {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(
            BenchmarkId::new("pattern", pattern_name),
            &query,
            |b, query| {
                b.iter(|| {
                    let results = registry.search(black_box(query));
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

fn bench_repeated_searches(c: &mut Criterion) {
    let registry = create_small_registry();

    let mut group = c.benchmark_group("repeated_searches");

    // Test if there's any caching effect or performance degradation
    let queries = vec!["react", "node", "vue", "angular"];

    group.bench_function("repeated_same_query", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let results = registry.search(black_box("react"));
                black_box(results);
            }
        })
    });

    group.bench_function("repeated_different_queries", |b| {
        b.iter(|| {
            for i in 0..100 {
                let query = queries[i % queries.len()];
                let results = registry.search(black_box(query));
                black_box(results);
            }
        })
    });

    group.finish();
}

fn bench_search_memory_usage(c: &mut Criterion) {
    let registry = create_small_registry();

    let mut group = c.benchmark_group("memory_usage");

    // Measure memory allocation patterns
    group.bench_function("search_allocations", |b| {
        b.iter(|| {
            let results = registry.search(black_box("javascript runtime"));
            // Force results to be used to prevent optimization
            assert!(!results.is_empty() || results.is_empty());
            black_box(results)
        })
    });

    // Test large result sets
    group.bench_function("large_result_set", |b| {
        b.iter(|| {
            // This query should match many entries
            let results = registry.search(black_box("a"));
            black_box(results)
        })
    });

    group.finish();
}

fn bench_edge_case_queries(c: &mut Criterion) {
    let registry = create_small_registry();

    let mut group = c.benchmark_group("edge_cases");

    let long_repeated = "javascript ".repeat(20);
    let edge_cases = vec![
        ("empty_string", ""),
        ("whitespace_only", "   "),
        ("very_short", "a"),
        ("special_chars", "!@#$%^&*()"),
        ("numbers_only", "12345"),
        ("mixed_special", "node.js-v18+"),
        ("repeated_chars", "aaaaaaaaaa"),
        ("long_repeated", long_repeated.as_str()),
        ("unicode_mixed", "react🚀"),
        ("newlines", "react\nnode"),
    ];

    for (case_name, query) in edge_cases {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(
            BenchmarkId::new("edge_case", case_name),
            &query,
            |b, query| {
                b.iter(|| {
                    let results = registry.search(black_box(query));
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

fn bench_registry_size_impact(c: &mut Criterion) {
    let small_registry = create_small_registry();
    let large_registry = create_large_registry(); // Note: currently same as small due to implementation

    let mut group = c.benchmark_group("registry_size_impact");

    let query = "javascript";

    group.bench_function("small_registry", |b| {
        b.iter(|| {
            let results = small_registry.search(black_box(query));
            black_box(results)
        })
    });

    group.bench_function("large_registry", |b| {
        b.iter(|| {
            let results = large_registry.search(black_box(query));
            black_box(results)
        })
    });

    group.finish();
}

fn bench_search_field_types(c: &mut Criterion) {
    let registry = create_small_registry();

    let mut group = c.benchmark_group("field_types");

    // Test which fields are being matched
    let field_tests = vec![
        ("name_match", "React"),                           // Should match name field
        ("slug_match", "react"),                           // Should match slug field
        ("alias_match", "reactjs"),                        // Should match alias field
        ("description_match", "building user interfaces"), // Should match description
        ("multi_field", "javascript"),                     // Should match multiple fields
    ];

    for (field_type, query) in field_tests {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::new("field", field_type), &query, |b, query| {
            b.iter(|| {
                let results = registry.search(black_box(query));
                black_box(results)
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_search_query_sizes,
    bench_search_result_sizes,
    bench_fuzzy_matching_types,
    bench_concurrent_searches,
    bench_search_patterns,
    bench_repeated_searches,
    bench_search_memory_usage,
    bench_edge_case_queries,
    bench_registry_size_impact,
    bench_search_field_types,
);

criterion_main!(benches);
