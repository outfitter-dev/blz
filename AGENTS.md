# blz Repository Instructions

This file provides guidance to AI agents when working with code in this repository.

## Agent Quick Start

### First Steps for New Agents
1. **Read Core Rules**: Follow @./.agents/rules/CORE.md for engineering principles
2. **Check Async Patterns**: See @./.agents/rules/ASYNC-PATTERNS.md for common async/await pain points  
3. **Use Compiler Loop**: Run `./scripts/agent-check.sh --fix` for automated diagnostics and fixes
4. **Read Crate Context**: Check crate-specific AGENTS.md files for module guidance

### Common Agent Pain Points & Solutions
- **Send + Sync + 'static bounds**: See ASYNC-PATTERNS.md section 1
- **Borrowing across .await**: See ASYNC-PATTERNS.md section 2  
- **Compiler errors**: Use `./scripts/agent-check.sh --verbose` for JSON diagnostics
- **Macro debugging**: Use `./scripts/agent-check.sh --expand` for macro expansion

### Quick Development Loop
```bash
# 1. Check and fix common issues
./scripts/agent-check.sh --fix

# 2. Run tests
cargo test --workspace

# 3. Check performance (for blz-core changes)
cargo bench -p blz-core

# 4. Verify linting
cargo clippy --workspace -- -D warnings
```

## Important

- Follow the @./.agents/rules/CORE.md rules

## Repository Overview

`blz` is a local-first search cache for llms.txt documentation ecosystems. Built with Rust and Tantivy, it provides millisecond-latency search with exact line citations for cached documentation.

## Architecture

The codebase is organized as a Rust workspace with three main crates:

- **`blz-core`**: Core functionality including fetcher, parser, indexer, and storage
- **`blz-cli`**: Command-line interface binary
- **`blz-mcp`**: MCP server implementation (in development)

Key components:

- **Fetcher**: HTTP client with ETag support for conditional fetching
- **Parser**: Tree-sitter-based markdown parser for structured document parsing
- **Index**: Tantivy-powered full-text search with BM25 ranking
- **Registry**: Source management and configuration
- **Storage**: Local filesystem storage with archive support

## Common Development Commands

### Building & Testing

```bash
# Build all crates in release mode
cargo build --release

# Run all tests
cargo test

# Run tests for specific crate
cargo test -p blz-core
cargo test -p blz-cli

# Run with verbose output for debugging
RUST_LOG=debug cargo run -- search "test"
```

### Code Quality

```bash
# Format code
cargo fmt

# Run Clippy linting (configured with strict rules)
cargo clippy --all-targets --all-features -- -D warnings

# Run the lint script (filters known acceptable warnings)
./scripts/lint.sh

# Auto-fix some Clippy issues
cargo clippy --fix
```

### Performance Testing

```bash
# Run benchmarks
cargo bench

# Profile with hyperfine (after building release)
hyperfine --warmup 10 --min-runs 50 './target/release/blz search "test" --alias bun'

# Run search performance benchmark
cargo bench --bench search_performance
```

### CLI Development

```bash
# Install locally for testing
cargo install --path crates/blz-cli

# Generate shell completions
blz completions fish > ~/.config/fish/completions/blz.fish
blz completions bash > ~/.local/share/bash-completion/completions/blz
blz completions zsh > ~/.zsh/completions/_blz
```

## Key Implementation Details

### Error Handling

- Uses `anyhow::Result` throughout for error propagation
- No `unwrap()`, `expect()`, `panic!()`, `todo!()`, or `unimplemented!()` allowed (enforced by Clippy)
- Custom error types in `blz-core/src/error.rs`

### Performance Constraints

- Search latency target: P50 < 10ms
- Index build: < 150ms per MB of markdown
- Zero unnecessary allocations in hot paths
- Conditional fetching with ETag to minimize bandwidth

### Storage Layout

```text
~/.outfitter/blz/
  global.toml                 # Global configuration
  <alias>/
    llms.txt                  # Latest upstream text
    llms.json                 # Parsed TOC + line map
    .index/                   # Tantivy search index
    .archive/                 # Historical snapshots
    settings.toml             # Per-source overrides
```

### Testing Approach

- Unit tests alongside implementation files
- Integration tests for CLI commands
- Performance benchmarks for search operations
- Use `tempfile` for test directories

## Rust-Specific Guidelines

### Linting Configuration
The project uses strict Clippy rules (see `clippy.toml` and workspace lints in `Cargo.toml`):

- All pedantic and nursery lints enabled
- Documentation warnings enabled
- Dangerous patterns (`unwrap`, `panic`, etc.) are denied

### Dependencies

- Workspace dependencies defined in root `Cargo.toml`
- Internal crates use workspace versioning
- Key external deps: tantivy (search), tree-sitter (parsing), tokio (async), reqwest (HTTP)

### Build Features

- `flamegraph` feature for profiling support
- Release builds use LTO for optimization
