# Development Practices

## Core Rules

### Standards

- @PERFORMANCE.md - Performance requirements and benchmarking
- @SECURITY.md - Security practices and dependency management
- @TESTING.md - Testing standards and coverage requirements

## Conventions

- @SOURCE-CONTROL.md - Source control, commits, and PR guidelines
- @conventions/cargo.md - Cargo workspace configuration
- @conventions/rust.md - Rust coding standards
- @conventions/tantivy.md - Search index best practices

### Development Guides

- @ENVIRONMENT.md - Development environment setup and tools
- @WORKFLOW.md - Development workflow and processes
- @CODE-ORGANIZATION.md - Module structure and code organization
- @QUALITY.md - Quality assurance and review practices

## Quick Start

First, run `just --list` or `make help` to discover available targets

```bash
# Setup environment (see @ENVIRONMENT.md for details)
rustup default stable
cargo install cargo-deny cargo-shear

# Coverage tooling
cargo install cargo-llvm-cov --locked

# Daily workflow (see @WORKFLOW.md for details)
cargo test --workspace
cargo clippy -- -D warnings
cargo fmt

# Quality checks (see @QUALITY.md for details)
make ci  # or: just ci
```

## Key Principles

1. **Test First** - Write tests before implementation (TDD)
2. **Small PRs** - Keep changes focused and reviewable
3. **No Unsafe** - Avoid `unsafe` code without thorough review
4. **Document Public APIs** - All public interfaces must be documented
5. **Performance Matters** - Profile and benchmark critical paths
6. **Security by Default** - Validate inputs, handle errors properly

## Common Tasks

| Task | Command | Details |
|------|---------|---------|
| Run tests | `cargo test` | See @TESTING.md |
| Check code | `cargo clippy` | See @QUALITY.md |
| Format code | `cargo fmt` | Automatic formatting |
| Security audit | `cargo deny check` | See @SECURITY.md |
| Benchmarks | `cargo bench` | See @PERFORMANCE.md |
| Documentation | `cargo doc --open` | Generate and view docs |
| Coverage | `cargo llvm-cov --html` | See @QUALITY.md |

## Getting Help

- Check the specific guide files for detailed information
- Run `make help` or `just --list` for available commands
- See `CONTRIBUTING.md` for contribution guidelines
- Review existing code for patterns and examples
