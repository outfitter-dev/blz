# Contributing to blz

Thank you for your interest in contributing to blz! This guide will help you get started.

## Code of Conduct

By participating in this project, you agree to abide by our code of conduct: be respectful, constructive, and collaborative.

## How to Contribute

### Reporting Issues

1. **Search existing issues** to avoid duplicates
2. **Use issue templates** when available
3. **Provide context**: Include error messages, system info, and steps to reproduce
4. **Be specific**: Clear titles and descriptions help us address issues faster

### Suggesting Features

1. **Open a discussion** first for major features
2. **Explain the use case**: Why is this feature needed?
3. **Consider alternatives**: What other solutions did you consider?
4. **Be patient**: Features take time to design and implement properly

### Submitting Code

We use [Graphite](https://graphite.dev) for stacked PRs, which allows you to break large changes into smaller, reviewable pieces.

#### Setup Graphite

```bash
# Install Graphite CLI
brew install withgraphite/tap/graphite
# or: npm install -g @withgraphite/graphite-cli

# Initialize in the repo
gt init

# Configure your preferences
gt config
```

#### Development Workflow

1. **Sync with main**:
   ```bash
   gt sync --no-interactive
   ```

2. **Create a feature branch**:
   ```bash
   gt create -m "feat: add new feature"
   ```

3. **Make your changes**:
   - Write tests first (TDD)
   - Implement the feature
   - Ensure all tests pass
   - Run quality checks

4. **Stack additional changes** (if needed):
   ```bash
   gt create -m "test: add comprehensive tests"
   gt create -m "docs: update documentation"
   ```

5. **Submit your stack**:
   ```bash
   gt submit --no-interactive
   ```

#### Code Quality Standards

Before submitting, ensure your code meets these standards:

```bash
# Format code
cargo fmt

# Run linter
cargo clippy --workspace --all-targets -- -D warnings

# Run tests
cargo test --workspace

# Check for security issues
cargo deny check

# Check for unused dependencies
cargo shear
```

#### Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Formatting, missing semicolons, etc.
- `refactor`: Code restructuring without behavior change
- `perf`: Performance improvements
- `test`: Adding or modifying tests
- `ci`: CI/CD changes
- `chore`: Maintenance tasks

**Examples:**
```bash
git commit -m "feat(cli): add shell completion support"
git commit -m "fix(#42): prevent panic in search pagination"
git commit -m "docs: update contributing guide"
```

### Pull Request Process

1. **Keep PRs small**: Aim for <300 lines of meaningful changes
2. **One concern per PR**: Don't mix features, fixes, and refactoring
3. **Update documentation**: Include relevant doc updates
4. **Add tests**: New features need tests
5. **Address feedback**: Respond to review comments promptly

#### PR Review Automation

Our repository uses automated tools to help review PRs:

- **Claude Code Review**: Automatically reviews PRs for code quality
- **CodeRabbit**: Provides additional code review insights
- **Dependency Review**: Checks for security issues in dependencies

#### What Happens Next?

1. **Automated checks run**: CI/CD pipeline validates your changes
2. **Claude reviews**: AI provides initial feedback
3. **Human review**: Maintainers review the code
4. **Feedback iteration**: Address any requested changes
5. **Merge**: Once approved, your PR will be merged

## Development Environment

### Required Tools

```bash
# Rust toolchain (1.85+ required, edition 2024)
rustup default stable
rustup component add clippy rustfmt rust-src

# Security and dependency management tools
cargo install cargo-deny    # License and vulnerability checking
cargo install cargo-shear   # Unused dependency detection

# Git hooks
brew install lefthook  # or see other installation methods
lefthook install

# Or use the Makefile/justfile for convenience
make install-tools   # or: just install-tools
```

### Recommended Tools

```bash
# File watching for auto-rebuild
cargo install cargo-watch

# Code coverage
cargo install cargo-llvm-cov

# Performance profiling
cargo install flamegraph

# Benchmarking
cargo install hyperfine
```

### IDE Setup

We recommend VS Code with these extensions:
- rust-analyzer
- Even Better TOML
- GitLens
- Graphite

## Testing

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Benchmarks
cargo bench
```

### Writing Tests

- Place unit tests in the same file as the code
- Use descriptive test names
- Test edge cases and error conditions
- Aim for >80% code coverage on new code

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_valid_query() {
        let query = "rust programming";
        let result = parse_query(query);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().terms.len(), 2);
    }

    #[test]
    fn should_reject_empty_query() {
        let result = parse_query("");
        assert!(result.is_err());
    }
}
```

## Style Guide

### Rust Code

- Follow standard Rust naming conventions
- Use `rustfmt` for formatting
- Keep functions small and focused
- Prefer explicit over implicit
- Document public APIs

### Error Handling

- Use `Result<T, E>` for recoverable errors
- Use `anyhow` for application errors
- Use `thiserror` for library errors
- Never use `unwrap()` or `expect()` in production code
- Always provide context for errors

### Performance

- Profile before optimizing
- Document performance-critical code
- Prefer zero-copy operations
- Use benchmarks to validate improvements

#### Performance Requirements

All changes must maintain or improve performance:

- **Search latency**: P50 < 10ms on standard hardware
- **Index build**: < 150ms per MB of markdown
- **Zero unnecessary allocations** in hot paths
- **Benchmark changes**: Run `hyperfine` to verify performance

```bash
# Example performance test
./target/release/blz add bun https://bun.sh/llms.txt
hyperfine --warmup 10 --min-runs 50 \
  './target/release/blz search "test" --alias bun'
# Expected: Mean < 10ms
```

## Getting Help

### Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tantivy Documentation](https://docs.rs/tantivy/)
- [Graphite Documentation](https://graphite.dev/docs)
- Project-specific rules in `.agents/rules/`

### Communication

- **Questions**: Open a [GitHub Discussion](https://github.com/outfitter-dev/blz/discussions)
- **Bugs**: File an [Issue](https://github.com/outfitter-dev/blz/issues)
- **Security**: Email security concerns privately

## Recognition

Contributors are recognized in several ways:
- Listed in release notes
- Mentioned in the changelog
- GitHub contributor badge
- Our sincere thanks! 🙏

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).