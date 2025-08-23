# Performance Benchmarks

## TL;DR

**6ms search latency on real documentation.** That's not a typo.

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
    './target/release/cache search "test concurrency" --alias bun --limit 5'

Time (mean ± σ):       5.9 ms ±   0.4 ms
Range (min … max):     4.8 ms …   8.8 ms    518 runs
```

## Performance vs Requirements

| Metric | PRD Target | Actual Performance | Improvement |
|--------|------------|-------------------|-------------|
| P50 Search | < 80ms | 6ms | **13x faster** |
| P95 Search | < 150ms | < 10ms | **15x faster** |
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
./target/release/cache add bun https://bun.sh/llms.txt

# Benchmark
hyperfine --warmup 10 --min-runs 50 \
  './target/release/cache search "test" --alias bun --limit 5' \
  './target/release/cache search "http server" --alias bun --limit 5'
```

## System Specs

Benchmarks run on:
- Platform: macOS Darwin 24.4.0
- Date: 2025-08-23
- Rust: 1.80+ (edition 2021)
- Build: Release mode with optimizations

## Next Steps

Even with these numbers, we can go faster:
- [ ] Parallel search across sources
- [ ] Memory-mapped index files
- [ ] Optional SIMD acceleration
- [ ] Query result caching

But honestly? 6ms is already faster than most network round-trips.