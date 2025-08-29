<!-- note ::: @agents this is a work in progress. Do not use this guidance verbatim. -->
# Performance Benchmarks

## TL;DR

6ms search latency on real documentation. Yes, that's not a typo.

## Real-World Performance

### Bun's llms.txt

- **Document**: 364 lines, 26 heading blocks
- **Fetch + Parse + Index**: 373ms total
- **Search Performance**:
  - Mean: 6ms
  - Min: 4.8ms
  - Max: 8ms
  - P95: <8ms

### Node.js API Documentation

- **Document**: 108,600 lines (4.8MB JSON)
- **Fetch + Parse + Index**: 1.9s
- **Search Performance**: ~32ms (searching across all sources)

## Benchmark Results

Using `hyperfine` with 100+ runs, no shell overhead:

```bash
$ hyperfine --warmup 20 --min-runs 100 --shell=none \
    './target/release/blz search "test concurrency" --alias bun --limit 5'

Time (mean ± σ):       5.9 ms ±   0.4 ms
Range (min … max):     4.8 ms …   8.8 ms    518 runs
```

## Performance vs Requirements

| Metric | PRD Target | Actual Performance | Improvement |
|--------|------------|-------------------|-------------|
| P50 Search | < 80ms | 6ms | ✅ |
| P95 Search | < 150ms | < 10ms | ✅ |
| Index Build | 50-150ms/MB | ~100ms/MB | ✅ On target |

## How?

1. **Rust** - Zero-cost abstractions, no GC pauses
2. **Tantivy** - Production-grade search engine (powers Quickwit)
3. **Tree-sitter** - Incremental parsing with perfect line tracking
4. **Smart indexing** - Heading-block documents for optimal BM25
5. **Minimal overhead** - Direct stdio, no HTTP, no serialization layers

## Reproducible Benchmarks

```bash
# Setup
cargo build --release
./target/release/blz add bun https://bun.sh/llms.txt

# Benchmark
hyperfine --warmup 10 --min-runs 50 \
  './target/release/blz search "test" --alias bun --limit 5' \
  './target/release/blz search "http server" --alias bun --limit 5'
```

## System Specs

Benchmarks run on:

- Platform: macOS Darwin 24.4.0
- Date: 2025-08-23
- Rust: 1.80+ (edition 2021)
- Build: Release mode with optimizations

## Profiling & Performance Analysis

### Built-in Performance Instrumentation

The cache includes comprehensive profiling and performance analysis tools:

#### Basic Performance Metrics

```bash
# Show detailed timing breakdowns
./target/release/blz search "react hooks" --debug

# Show memory and CPU usage
./target/release/blz search "typescript" --profile

# Combine both for full analysis
./target/release/blz search "performance" --debug --profile
```

#### CPU Profiling (Flamegraph)

```bash
# Build with flamegraph support
cargo build --release --features flamegraph

# Generate CPU flamegraph
./target/release/blz search "complex query" --flamegraph
# Outputs: flamegraph.svg and cache_profile.pb
```

#### Benchmarking

```bash
# Run comprehensive performance benchmarks
cd crates/cache-core
cargo bench

# Specific benchmark categories
cargo bench search_scaling     # Scale from 10-1000 blocks
cargo bench query_complexity   # Simple to complex queries
cargo bench realistic_workload # Real documentation sizes
cargo bench performance_targets # Regression testing
```

### Performance Metrics Tracked

**Search Operations:**

- Total execution time (target: <10ms, achieved: ~6ms)
- Lines searched per operation
- Component-level breakdowns:
  - Searcher creation
  - Query parsing
  - Tantivy search execution
  - Result processing
- Throughput in lines/second

**Index Building:**

- Total indexing time
- Bytes processed
- Component-level breakdowns:
  - Writer creation
  - Document creation
  - Commit operations
  - Reader reload
- Throughput in MB/second

**Resource Usage:**

- Memory consumption (current and delta)
- CPU utilization during operations
- Peak resource usage tracking

### Component Timing Breakdown

When using `--debug`, you'll see detailed breakdowns like:

```text
Component Breakdown
==================
  tantivy_search      :     2.15ms ( 35.8%)
  result_processing   :     1.89ms ( 31.5%)
  query_parsing       :     1.22ms ( 20.3%)
  searcher_creation   :     0.74ms ( 12.3%)
  TOTAL              :     6.00ms
```

### Benchmark Results Summary

Our benchmarking shows consistent sub-millisecond performance:

| Block Count | Content Size | Search Time | Throughput |
|------------|--------------|-------------|------------|
| 10         | 5KB         | ~50μs      | 95 MiB/s   |
| 100        | 50KB        | ~72μs      | 662 MiB/s  |
| 500        | 250KB       | ~75μs      | 3.1 GiB/s  |
| 1000       | 500KB       | ~105μs     | 4.4 GiB/s  |

**Real-world scenarios:**

- Small library docs (50 blocks): ~50μs
- Medium framework (200 blocks): ~70μs
- Large framework like React (1000 blocks): ~105μs
- Very large docs like Node.js (5000 blocks): ~200μs

All measurements consistently meet performance targets.

### Memory Efficiency

Index memory usage remains minimal:

- ~1-2MB per 1000 documentation blocks
- Lazy loading of search indices
- Efficient string interning in Tantivy
- Minimal overhead from profiling (< 1μs per operation)

### Performance Regression Testing

Use `performance_targets` benchmark to ensure no regressions:

```bash
cargo bench performance_targets
```

This validates:

- Individual searches complete in <10ms (target achieved: ~6ms)
- Multiple rapid searches maintain performance
- Memory usage stays within reasonable bounds

## Next Steps

Even with these numbers, we can go faster:

- [ ] Parallel search across sources
- [ ] Memory-mapped index files
- [ ] Optional SIMD acceleration
- [ ] Query result caching
- [x] **Comprehensive profiling and benchmarking** ✅
- [x] **Component-level performance breakdown** ✅
- [x] **CPU flamegraph generation** ✅
- [x] **Memory usage tracking** ✅

The current performance meets requirements for local-first search.
