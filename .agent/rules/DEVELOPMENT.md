# Development Practices

## Core Rules

### Standards & Conventions
- **@PERFORMANCE.md** - Performance requirements and benchmarking
- **@SECURITY.md** - Security practices and dependency management  
- **@TESTING.md** - Testing standards and coverage requirements
- **@conventions/** - Language and tool-specific conventions
  - `commits.md` - Git commit message format
  - `cargo.md` - Cargo workspace configuration
  - `rust.md` - Rust coding standards
  - `tantivy.md` - Search index best practices

### Development Guides
- **@ENVIRONMENT.md** - Development environment setup and tools
- **@WORKFLOW.md** - Development workflow and processes
- **@CODE-ORGANIZATION.md** - Module structure and code organization
- **@QUALITY.md** - Quality assurance and review practices

## Documentation Maintenance

### Keep AGENTS.md/CLAUDE.md in sync

When making significant changes:
1. Update `./AGENTS.md` with new patterns or architecture changes
2. Run `./.agent/scripts/sync-agents-md.sh` to sync to CLAUDE.md files
3. AGENTS.md is the source of truth - always edit it, not CLAUDE.md

## Quick Start

```bash
# Setup environment (see @ENVIRONMENT.md for details)
rustup default stable
cargo install cargo-deny cargo-shear

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