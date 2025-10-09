# Development Guide

Welcome to the BLZ development documentation. This guide covers our development process, tools, and best practices.

## 📚 Documentation

- [CI/CD Pipeline](ci_cd.md) - Continuous integration and deployment setup
- [Testing Guide](testing.md) - Testing strategies and tools
- [Contributing](./contributing.md) - How to contribute to the project
- [Development Workflow](./workflow.md) - Our development process and tools

## 🚀 Quick Start

### Prerequisites

1. **Rust**: Install via [rustup](https://rustup.rs/)

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Graphite CLI**: For stacked PRs

   ```bash
   brew install withgraphite/tap/graphite
   # or: npm install -g @withgraphite/graphite-cli
   ```

3. **Development Tools**:

   ```bash
   # Required
   cargo install cargo-deny cargo-shear
   
   # Optional but recommended
   cargo install cargo-watch cargo-llvm-cov flamegraph
   ```

### Setup

1. Clone the repository:

   ```bash
   git clone https://github.com/outfitter-dev/blz.git
   cd blz
   ```

2. Install git hooks:

   ```bash
   lefthook install
   ```

3. Build the project:

   ```bash
   cargo build --release
   ```

4. Run tests:

   ```bash
   cargo test --workspace
   ```

5. Validate documentation links:

   ```bash
   just link-check
   ```

## 🔧 Development Stack

### Core Technologies

- **Language**: Rust 1.85+ (stable channel, 2024 edition)
- **Search Engine**: Tantivy
- **Async Runtime**: Tokio
- **CLI Framework**: Clap
- **Testing**: Built-in Rust testing + Criterion for benchmarks

### Development Tools

- **Version Control**: Git with Graphite for stacked PRs
- **CI/CD**: GitHub Actions with Graphite optimization
- **Code Quality**: Clippy, rustfmt, cargo-deny
- **Docs QA**: Lychee link checker (`just link-check`)
- **Git Hooks**: Lefthook for pre-commit checks (see "Local Hooks + Nextest" in docs/development/ci_cd.md; quick start: `just bootstrap-fast`)
- **AI Assistance**: Claude for code reviews and development

## 📋 Project Structure

```text
blz/
├── crates/              # Workspace crates
│   ├── blz-core/       # Core functionality
│   ├── blz-cli/        # CLI application
│   └── blz-mcp/        # MCP server (future)
├── docs/               # Documentation
│   └── development/    # Development guides
├── .github/            # GitHub Actions workflows
├── .agents/            # AI agent configuration
└── tests/              # Integration tests
```text

## 🏗️ Architecture Principles

1. **Workspace Organization**: Modular crates for separation of concerns
2. **Error Handling**: Using `anyhow` for application errors, `thiserror` for library errors
3. **Performance**: Zero-copy operations where possible, efficient caching
4. **Security**: No unsafe code, comprehensive input validation
5. **Testing**: Unit tests alongside code, integration tests in `tests/`

## 🔬 Local Development Setup

This section covers how to run `blz` from source without disturbing an existing release installation using the opt-in developer profile (`blz-dev`).

### When To Use The Dev Profile

Use the `blz-dev` binary when you want to:

- Test changes locally while keeping the stable `blz` binary on your PATH
- Maintain isolated config/cache data (`blz-dev` writes to `~/.blz-dev/` or platform equivalents)
- Exercise new functionality without touching production indexes

The dev profile is gated behind the `dev-profile` cargo feature and never ships with release artifacts. You must install it manually.

### Installing `blz-dev`

```bash
# From the repository root
./install-dev.sh --root "$HOME/.local/share/blz-dev"
```text

The script wraps `cargo install --features dev-profile --bin blz-dev --path crates/blz-cli` and passes through any extra flags you supply (`--root`, `--force`, `--locked`, etc).

After installation, add the target `bin` directory to your PATH *ahead* of other blz binaries:

```bash
export PATH="$HOME/.local/share/blz-dev/bin:$PATH"
```text

Alternatively, call the binary directly via absolute path.

### Hydrating From Existing Installation

If you already have sources configured in your production `blz` installation, you can copy them to `blz-dev` to start testing immediately:

```bash
# Copy everything (config + sources)
./hydrate-dev.sh

# Preview what would be copied
./hydrate-dev.sh --dry-run

# Copy only configuration files
./hydrate-dev.sh --config-only

# Copy only source data and indices
./hydrate-dev.sh --sources-only

# Overwrite existing blz-dev data
./hydrate-dev.sh --force
```text

The script is XDG-aware and handles both macOS and Linux paths automatically. It copies:

- **Config files**: `config.toml`, `data.json`, `history.jsonl`
- **Source data**: All cached `llms.txt` files and search indices

This is particularly useful when:

- Testing migrations or upgrades against real data
- Benchmarking performance with your actual source set
- Developing features that depend on existing indices

### Where Data Lives

The profile-aware path logic in `blz-core` puts dev metadata under dedicated directories:

- **Config**: `$XDG_CONFIG_HOME/blz-dev/` or `~/.blz-dev/` (non-XDG)
- **Data/indexes**: `$XDG_DATA_HOME/blz-dev/` or `~/.blz-dev/`
- **Preferences/history**: same root as config

These locations are separate from the stable profile (`blz`) to avoid cross-contamination.

### Building & Testing

| Task | Command |
| --- | --- |
| Format | `cargo fmt` |
| Check primary binary | `cargo check -p blz-cli` |
| Check dev binary | `cargo check -p blz-cli --features dev-profile --bin blz-dev` |
| Run tests | `cargo test --workspace` |

No additional features are required to run the standard test suite, but the second `cargo check` ensures the dev entrypoint compiles.

### Cleaning Up

Remove the dev installation by deleting the install root and the profile directories:

```bash
rm -rf "$HOME/.local/share/blz-dev"
rm -rf "${XDG_CONFIG_HOME:-$HOME/.blz-dev}"
rm -rf "${XDG_DATA_HOME:-$HOME/.blz-dev}"
```text

Be careful to double-check paths before running the commands above.

## 🤝 Getting Help

- **Issues**: [GitHub Issues](https://github.com/outfitter-dev/blz/issues)
- **Issues**: [GitHub Issues](https://github.com/outfitter-dev/blz/issues)
- **Documentation**: Check `.agents/rules/` for detailed development rules

## 📜 License

This project is licensed under MIT OR Apache-2.0. See [LICENSE](../../LICENSE) for details.
