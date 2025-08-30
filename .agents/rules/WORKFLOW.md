# Development Workflow

## Daily Development

### Start of Day

```bash
# Sync with trunk and prune merged branches
gt sync

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

# Check your current stack state
gt log short
```

## Feature Development Process

### 1. Stack Creation & Management

```bash
# Always start by syncing
gt sync

# Create new branch in the stack
gt create feat/your-feature-name -am "Initial implementation"

# Or track existing branch
gt track --parent main
```

Follow branch naming conventions:

- `feature/` or `feat/` - New features
- `fix/` - Bug fixes
- `refactor/` - Code refactoring
- `docs/` - Documentation updates
- `perf/` - Performance improvements
- `chore/` - Miscellaneous tasks
- `test/` - Testing changes
- `build/` - Build system changes
- `ci/` - Continuous integration changes
- `ops/` - Operations changes

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

### 4. Update & Refactor

```bash
# Amend current branch commit
gt modify -am "Updated implementation"

# Or create new commit on current branch
gt modify -cam "Additional changes"

# Fix the right commit in the stack
gt absorb -a
```

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

### 6. Stack Operations

```bash
# Restack after changes
gt restack --upstack

# Navigate the stack
gt up        # Move up one branch
gt down      # Move down one branch
gt top       # Jump to stack tip
gt bottom    # Jump to stack base

# Submit/update PRs for entire stack
gt submit --stack
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

Follow conventional commits (see @conventions/commits.md) with Graphite:

```bash
# Create commits with Graphite (preferred)
gt create feat/cache -am "feat: add search result caching"
gt create fix/index -am "fix: resolve index corruption on concurrent writes"
gt create perf/parser -am "perf: optimize search query parsing"
gt create docs/api -am "docs: update API documentation for search module"

# Modify existing commits
gt modify -am "fix: address review feedback"
gt modify -cam "feat: add additional functionality"

# Never use raw git commands on tracked branches
# ❌ git commit -m "message"
# ✅ gt create -am "message" or gt modify -am "message"
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

### Working with PRs

```bash
# Submit entire stack as PRs
gt submit --stack

# Merge PRs in correct order (trunk → tip)
gt merge

# Preview merge order
gt merge --dry-run

# After PRs are merged
gt sync  # Prune merged branches and update
```

## Release Process

1. Update version in Cargo.toml files
2. Update CHANGELOG.md
3. Run full test suite
4. Create release stack:

```bash
# Create release preparation branch
gt create release/v0.1.0 -am "chore: prepare release v0.1.0"

# Submit for review
gt submit --stack

# After merge via gt merge
gt sync
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

### Graphite-Specific Issues

**Stack out of sync:**

```bash
# Fix everything
gt sync && gt restack --upstack && gt submit --stack
```

**Untracked branch created with git:**

```bash
# Track existing branch
gt track --parent main
gt restack --only
gt submit --stack
```

**Merge conflicts during restack:**

```bash
# Resolve conflicts in files
# Stage resolved files
git add <resolved-files>

# Continue Graphite operation
gt continue

# Or abort if needed
gt abort
```

**PRs out of sync:**

```bash
# Normal update
gt submit --stack

# Force sync if severely desynced
gt submit --always --stack
```

**Need to check Graphite state:**

```bash
# View current stack
gt log --stack

# Interactive branch switch
gt checkout

# Show diagnostic info
gt log short && git status
```
