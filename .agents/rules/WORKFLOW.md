# Development Workflow

## Daily Development

### Start of Day

```bash
# Update dependencies and tools
rustup update
cargo update

# Check for security issues
cargo deny check advisories

# Run full test suite
cargo test --workspace

# Check code quality
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

## Feature Development Process

### 1. Branch Creation

```bash
git checkout -b feature/your-feature-name
```

Follow branch naming conventions:
- `feature/` - New features
- `fix/` - Bug fixes
- `refactor/` - Code refactoring
- `docs/` - Documentation updates
- `perf/` - Performance improvements

### 2. Write Tests First (TDD)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_functionality() {
        // Write failing test first
        let result = new_function("input");
        assert_eq!(result, expected_output);
    }
}
```

### 3. Implement Minimal Solution

- Write the simplest code that makes tests pass
- Don't optimize prematurely
- Focus on correctness first

### 4. Refactor

- Improve code structure
- Add error handling
- Optimize if needed
- Ensure all tests still pass

### 5. Quality Gates

Before committing:

```bash
# Run all checks
make ci  # or: just ci

# Or manually:
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt
cargo deny check
cargo shear
```

## Code Review Process

### Self-Review Checklist

- [ ] All tests pass locally
- [ ] Clippy passes with zero warnings
- [ ] Code is formatted with rustfmt
- [ ] Documentation is updated
- [ ] No `unwrap()` or `expect()` in production code
- [ ] Error handling is comprehensive
- [ ] Performance implications considered
- [ ] Security implications reviewed

### Commit Guidelines

Follow conventional commits (see @conventions/commits.md):

```bash
# Good commit messages
git commit -m "feat: add search result caching"
git commit -m "fix: resolve index corruption on concurrent writes"
git commit -m "perf: optimize search query parsing"
git commit -m "docs: update API documentation for search module"
```

## Debugging Practices

### Logging Strategy

```rust
use tracing::{debug, info, warn, error, instrument};

#[instrument(skip(self), fields(query_len = query.len()))]
pub async fn search(&self, query: &str) -> Result<SearchResults> {
    debug!("Starting search operation");
    
    match self.execute_search(query).await {
        Ok(results) => {
            info!(result_count = results.len(), "Search completed");
            Ok(results)
        }
        Err(e) => {
            error!(error = %e, "Search failed");
            Err(e)
        }
    }
}
```

### Debug Builds

```rust
// Use debug assertions in development
debug_assert!(query.len() > 0, "Query cannot be empty");
debug_assert!(limit <= MAX_RESULTS, "Limit exceeds maximum");

// Conditional compilation for debug info
#[cfg(debug_assertions)]
fn debug_query_info(query: &ParsedQuery) {
    eprintln!("Query AST: {:#?}", query);
}
```

### Performance Debugging

```bash
# Profile with flamegraph
cargo flamegraph --bin blz -- search "rust"

# Benchmark specific operations
cargo bench --bench search_performance

# Memory profiling
DHAT_HEAP_PROFILING=1 cargo run --features dhat-heap
```

## Continuous Integration

See `.github/workflows/` for CI configuration.

Local CI simulation:

```bash
# Run full CI pipeline locally
make ci

# Or with just:
just ci
```

## Release Process

1. Update version in Cargo.toml files
2. Update CHANGELOG.md
3. Run full test suite
4. Create release PR
5. After merge, tag release:

```bash
git tag -a v0.1.0 -m "Release version 0.1.0"
git push origin v0.1.0
```

## Troubleshooting

### Common Issues

**Clippy failures:**
```bash
# Auto-fix some issues
cargo clippy --fix

# Check specific workspace member
cargo clippy -p blz-core
```

**Test failures:**
```bash
# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name -- --exact
```

**Compilation errors:**
```bash
# Clean build
cargo clean
cargo build

# Check for feature flag issues
cargo build --all-features
```