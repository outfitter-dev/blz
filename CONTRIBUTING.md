# Contributing to @outfitter/cache

Thank you for your interest in contributing to @outfitter/cache! This document provides guidelines and instructions for contributing.

## Development Setup

### Prerequisites

- Rust 1.75+ (edition 2021)
- Cargo
- Git

### Building

```bash
# Clone the repository
git clone https://github.com/outfitter-dev/cache
cd cache

# Build all crates
cargo build --release

# Run tests
cargo test

# Run with verbose output
cargo run -- --verbose search "test"
```

## Project Structure

```
cache/
├── crates/
│   ├── cache-core/      # Core functionality (fetcher, parser, index, storage)
│   ├── cache-cli/       # CLI binary
│   └── cache-mcp/       # MCP server (JSON-RPC)
├── scripts/             # Shell completions and utilities
└── .agent/              # Development documentation
```

## Making Changes

### Code Style

- Run `cargo fmt` before committing
- Fix all warnings with `cargo clippy`
- Ensure clean build with `cargo build --release`

### Testing

```bash
# Run all tests
cargo test

# Test specific crate
cargo test -p cache-core

# Run benchmarks
hyperfine './target/release/cache search "test" --alias bun'
```

### Performance Requirements

All changes must maintain or improve performance:
- Search latency: P50 < 10ms on standard hardware
- Index build: < 150ms per MB of markdown
- Zero unnecessary allocations in hot paths

## Adding Features

### New Commands

1. Add the command to `Commands` enum in `crates/cache-cli/src/main.rs`
2. Implement the handler function
3. Update shell completions by rebuilding
4. Add tests

### New Search Features

1. Modify `SearchIndex` in `crates/cache-core/src/index.rs`
2. Update schema if needed
3. Ensure backward compatibility or add migration
4. Benchmark the changes

## Commit Messages

Follow conventional commits:
- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation only
- `perf:` Performance improvement
- `refactor:` Code change that neither fixes a bug nor adds a feature
- `test:` Adding missing tests
- `chore:` Changes to build process or auxiliary tools

## Pull Requests

1. Fork and create a feature branch
2. Make your changes
3. Ensure all tests pass
4. Update documentation if needed
5. Run benchmarks if touching performance-critical code
6. Submit PR with clear description

### PR Checklist

- [ ] Tests pass (`cargo test`)
- [ ] No warnings (`cargo build --release`)
- [ ] Documentation updated
- [ ] Performance maintained/improved
- [ ] Conventional commit messages

## Performance Testing

When modifying search or indexing code:

```bash
# Add test document
./target/release/cache add bun https://bun.sh/llms.txt

# Benchmark search
hyperfine --warmup 10 --min-runs 50 \
  './target/release/cache search "test" --alias bun'

# Expected: Mean < 10ms
```

## Documentation

- Update README.md for user-facing changes
- Update PERFORMANCE.md if benchmarks change
- Document new functions and modules with doc comments
- Keep .agent/PRD.md aligned with implementation

## Questions?

Open an issue on GitHub for questions or discussions about contributions.