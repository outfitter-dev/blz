# blz Repository Instructions for AI Agents

This file provides comprehensive guidance to AI agents when working with Rust code in this repository.

## Important

- Follow the @./.agents/rules/IMPORTANT.md rules
- Check @.agents/logs/ for relevant work logs and session notes

## Use blz

- You're working on blz, so you should use it!
- For docs search, use the `blz` CLI tool: @.agents/instructions/use-blz.md

## 🚀 Quick Start for Agents

### Before You Begin

1. **Run the agent check script**: `./scripts/agent-check.sh` for compiler diagnostics and automated fixes
2. **Check current build status**: `cargo check --message-format=json` for machine-readable errors
3. **Review unsafe code policy**: See `.agents/rules/conventions/rust/unsafe-policy.md` if working with unsafe blocks

### Common Agent Pain Points & Solutions

- **Async confusion**: Read `.agents/rules/conventions/rust/async-patterns.md` to learn correct async/await patterns
- **Compiler errors**: Use `.agents/rules/conventions/rust/compiler-loop.md` for JSON diagnostics and macro expansion
- **Error handling**: Follow patterns in crate-specific AGENTS.md files

## 📂 Directory-Specific Guidance

Each crate has its own AGENTS.md with specialized patterns:

- `crates/blz-core/AGENTS.md` - Performance-critical library code, unsafe policy
- `crates/blz-cli/AGENTS.md` - User-facing CLI patterns, error messages
- `crates/blz-mcp/AGENTS.md` - MCP protocol compliance, JSON-RPC handling

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
cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged
```

### Performance Testing

```bash
# Run benchmarks
cargo bench

# Profile with hyperfine (after building release)
hyperfine --warmup 10 --min-runs 50 './target/release/blz search "test" --source bun'

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
# Determined via directories::ProjectDirs::from("dev", "outfitter", "blz")
# Config (XDG, all platforms):
#   $XDG_CONFIG_HOME/blz/ (or ~/.config/blz/)
# Data (platform data dir):
#   macOS:   ~/Library/Application Support/dev.outfitter.blz/
#   Linux:   ~/.local/share/dev.outfitter.blz/
#   Windows: %APPDATA%\dev.outfitter.blz\
#
# Within Data:
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
