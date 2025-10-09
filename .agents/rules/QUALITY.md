# Quality Assurance

## Code Quality Gates

### Pre-commit Checks

Install the pre-commit hook:

```bash
#!/bin/bash
# Save as .git/hooks/pre-commit

echo "Running pre-commit checks..."

# Check formatting
cargo fmt --check || {
    echo "❌ Code not formatted. Run 'cargo fmt'"
    exit 1
}

# Run clippy
cargo clippy --workspace --all-targets -- -D warnings || {
    echo "❌ Clippy found issues"
    exit 1
}

# Run tests
cargo test --workspace || {
    echo "❌ Tests failed"
    exit 1
}

echo "✅ Pre-commit checks passed!"
```

### Required Checks

Before any commit:

```bash
make ci  # or: just ci
```

This runs:

1. `cargo fmt` - Code formatting
2. `cargo clippy` - Linting
3. `cargo test` - Unit tests
4. `cargo deny check` - Security & licenses
5. `cargo shear` - Unused dependencies

## Testing Standards

### Test Coverage

- **Minimum**: 70% coverage for new code
- **Target**: 80% overall coverage
- **Critical paths**: 90% coverage required

Check coverage:

```bash
cargo llvm-cov --workspace --html
# Open target/llvm-cov/html/index.html
```

### Test Categories

#### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_specific_behavior() {
        // Arrange
        let input = "test";
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

#### Integration Tests

```rust
// tests/integration_test.rs
use blz_core::SearchIndex;
use tempfile::TempDir;

#[test]
fn test_end_to_end_search() {
    let temp_dir = TempDir::new().unwrap();
    let index = SearchIndex::new(temp_dir.path()).unwrap();
    
    // Test full workflow
    index.add_document("test content").unwrap();
    let results = index.search("test").unwrap();
    assert_eq!(results.len(), 1);
}
```

#### Property-based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_search_never_panics(query in "\\PC*") {
        let index = create_test_index();
        // Should not panic for any string input
        let _ = index.search(&query);
    }
}
```

### Test Organization

- Keep tests close to code
- Use descriptive test names
- One assertion per test when possible
- Use test fixtures for complex setup

## Performance Standards

### Benchmarking

Define benchmarks in `benches/`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn search_benchmark(c: &mut Criterion) {
    let index = create_test_index();
    
    c.bench_function("search_simple", |b| {
        b.iter(|| index.search(black_box("test")))
    });
}

criterion_group!(benches, search_benchmark);
criterion_main!(benches);
```

Run benchmarks:

```bash
cargo bench

# Compare with baseline
cargo bench -- --baseline
```

### Performance Requirements

- Search latency: P50 < 10ms, P99 < 50ms
- Index build: < 150ms per MB
- Memory usage: < 2x document size
- Zero allocations in hot paths

### Profiling

```bash
# CPU profiling
cargo flamegraph --bin blz -- search "test"

# Memory profiling
heaptrack target/release/blz
```

## Security Standards

### Dependency Security

```bash
# Check for vulnerabilities
cargo deny check advisories

# Check licenses
cargo deny check licenses

# Full check
cargo deny check
```

### Code Security

- No `unsafe` code without review
- No `unwrap()` or `expect()` in production
- Validate all user input
- Sanitize paths and filenames
- Use `SecStr` for sensitive data

## Documentation Standards

### Required Documentation

- All public APIs must be documented
- Include examples for complex functions
- Document error conditions
- Explain performance characteristics

### Documentation Coverage

Check documentation:

```bash
cargo doc --no-deps --document-private-items
cargo doc --open  # View in browser
```

Warning for missing docs:

```toml
[workspace.lints.rust]
missing_docs = "warn"
```

## Code Review Checklist

### Automated Checks

- [ ] CI passes (all green)
- [ ] No decrease in test coverage
- [ ] No new Clippy warnings
- [ ] No security advisories

### Manual Review

- [ ] Logic is correct and clear
- [ ] Error handling is comprehensive
- [ ] Tests cover edge cases
- [ ] Documentation is updated
- [ ] Performance impact considered
- [ ] Security implications reviewed

## Continuous Monitoring

### Metrics to Track

1. **Code Quality**
   - Clippy warning count
   - Cyclomatic complexity
   - Code duplication

2. **Test Quality**
   - Coverage percentage
   - Test execution time
   - Flaky test count

3. **Performance**
   - Benchmark trends
   - Memory usage
   - Binary size

4. **Dependencies**
   - Outdated count
   - Security advisories
   - License compliance

### Regular Audits

Weekly:

```bash
cargo outdated
cargo audit
```

Monthly:

```bash
cargo deny check
cargo bloat --release
tokei  # Lines of code metrics
```

## Anti-patterns to Avoid

### Code Smells

```rust
// ❌ BAD: Using unwrap
let value = option.unwrap();

// ✅ GOOD: Proper error handling
let value = option.ok_or(Error::MissingValue)?;

// ❌ BAD: Large functions
fn do_everything() {
    // 200 lines of code
}

// ✅ GOOD: Small, focused functions
fn parse_input() { }
fn validate_data() { }
fn process_result() { }

// ❌ BAD: Magic numbers
if count > 42 {

// ✅ GOOD: Named constants
const MAX_RETRIES: usize = 42;
if count > MAX_RETRIES {
```

### Performance Anti-patterns

```rust
// ❌ BAD: Cloning unnecessarily
let data = expensive_data.clone();

// ✅ GOOD: Borrowing when possible
let data = &expensive_data;

// ❌ BAD: Allocating in loops
for item in items {
    let mut vec = Vec::new(); // Allocates each iteration
}

// ✅ GOOD: Reuse allocations
let mut vec = Vec::new();
for item in items {
    vec.clear();
    // reuse vec
}
```

## Quality Metrics

Track these metrics over time:

| Metric | Target | Current |
|--------|--------|---------|
| Test Coverage | >80% | Check with `cargo llvm-cov` |
| Clippy Warnings | 0 | Check with `cargo clippy` |
| Security Issues | 0 | Check with `cargo deny` |
| P50 Search Latency | <10ms | Check with benchmarks |
| Documentation Coverage | 100% public APIs | Check with `cargo doc` |
