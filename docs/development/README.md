# Development Guide

Welcome to the BLZ development documentation. This guide covers our development process, tools, and best practices.

## 📚 Documentation

- [CI/CD Pipeline](./ci-cd.md) - Continuous integration and deployment setup
- [Contributing](./contributing.md) - How to contribute to the project
- [Local Development Setup](./local-development.md) - Run `blz-dev` alongside the stable CLI
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
- **Git Hooks**: Lefthook for pre-commit checks (see “Local Hooks + Nextest” in docs/development/ci-cd.md; quick start: `just bootstrap-fast`)
- **AI Assistance**: Claude for code reviews and development

## 📋 Project Structure

```
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
```

## 🏗️ Architecture Principles

1. **Workspace Organization**: Modular crates for separation of concerns
2. **Error Handling**: Using `anyhow` for application errors, `thiserror` for library errors
3. **Performance**: Zero-copy operations where possible, efficient caching
4. **Security**: No unsafe code, comprehensive input validation
5. **Testing**: Unit tests alongside code, integration tests in `tests/`

## 🤝 Getting Help

- **Issues**: [GitHub Issues](https://github.com/outfitter-dev/blz/issues)
- **Discussions**: [GitHub Discussions](https://github.com/outfitter-dev/blz/discussions)
- **Documentation**: Check `.agents/rules/` for detailed development rules

## 📜 License

This project is licensed under MIT OR Apache-2.0. See [LICENSE](../../LICENSE) for details.
