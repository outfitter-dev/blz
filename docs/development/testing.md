# Testing Guide

Complete guide to testing strategies, tools, and local validation for BLZ development.

## Table of Contents

- [Overview](#overview)
- [Test Suite](#test-suite)
- [Running Tests](#running-tests)
- [Local Validation with Act](#local-validation-with-act)
- [Pre-commit Hooks](#pre-commit-hooks)
- [Test Coverage](#test-coverage)
- [Documentation Link Checking](#documentation-link-checking)
- [Best Practices](#best-practices)

## Overview

BLZ uses a comprehensive testing approach combining:

- **Unit tests** - In-file tests alongside implementation
- **Integration tests** - End-to-end CLI testing in `tests/`
- **Benchmarks** - Performance regression detection via Criterion
- **Local CI** - GitHub Actions validation with Act before pushing

## Test Suite

### Unit Tests

Located alongside source code:

```rust
// crates/blz-core/src/parser.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_headings() {
        // Test implementation
    }
}
```

Run with:

```bash
cargo test --lib
```

### Integration Tests

Located in `tests/`:

```rust
// tests/cli_integration.rs
#[test]
fn test_search_command() {
    // CLI integration test
}
```

Run with:

```bash
cargo test --test '*'
```

### Benchmarks

Located in `benches/`:

```rust
// benches/search_performance.rs
fn search_benchmark(c: &mut Criterion) {
    // Benchmark implementation
}
```

Run with:

```bash
cargo bench
```

## Running Tests

### Quick Commands

```bash
# All tests (workspace)
cargo test --workspace

# Fast tests only
cargo test --lib --bins

# Specific package
cargo test -p blz-core
cargo test -p blz-cli

# Specific test
cargo test test_search_command

# With output
cargo test -- --nocapture

# Single-threaded (for debugging)
cargo test -- --test-threads=1
```

### Using Nextest

Nextest provides faster parallel test execution:

```bash
# Install
cargo install cargo-nextest

# Run with nextest
cargo nextest run

# Watch mode
cargo watch -x "nextest run"
```

### Test Organization

| Location | Purpose | Run with |
|----------|---------|----------|
| `src/**/*.rs` (mod tests) | Unit tests | `cargo test --lib` |
| `tests/*.rs` | Integration tests | `cargo test --test '*'` |
| `benches/*.rs` | Benchmarks | `cargo bench` |
| `examples/*.rs` | Example validation | `cargo test --examples` |

## Local Validation with Act

`act` allows running GitHub Actions workflows locally for immediate CI feedback.

### Installation

```bash
# macOS
brew install act

# Linux/Windows
# See: https://github.com/nektos/act#installation
```

### Quick Start

```bash
# Run fast validation (format + clippy)
./scripts/act-validate.sh fast

# Run full CI locally
./scripts/act-validate.sh full

# Run tests only
./scripts/act-validate.sh test

# Verbose output for debugging
VERBOSE=1 ./scripts/act-validate.sh fast
```

### Validation Modes

#### Fast Mode (<30s)

Ideal for pre-commit hooks and quick iteration:

- Rust formatting check
- Basic Clippy validation (workspace bins only)
- No test execution

```bash
./scripts/act-validate.sh fast
```

#### Full Mode (2-5 minutes)

Complete CI validation before pushing:

- Complete rust-ci workflow
- All Clippy checks (bins, examples, tests)
- Full test suite
- Build validation

```bash
./scripts/act-validate.sh full
```

#### Specialized Modes

```bash
# Format only
./scripts/act-validate.sh format

# Clippy only
./scripts/act-validate.sh clippy

# Tests only
./scripts/act-validate.sh test
```

### Performance Optimization

#### Container Reuse

Containers are reused by default for faster subsequent runs:

```bash
# First run: ~60s (downloads image, builds cache)
./scripts/act-validate.sh fast

# Subsequent runs: ~15-30s (reuses container)
./scripts/act-validate.sh fast
```

#### Resource Limits

Configured in `.actrc`:

```text
--container-options "--memory=4g --cpus=2"
```

Adjust based on your system capabilities.

#### Workflow Selection

- **rust-ci-local.yml**: Optimized for local execution
- **rust-ci.yml**: Full CI workflow (slower locally)
- **miri.yml**: Not recommended locally (very slow)

### Troubleshooting Act

**"act: command not found"**

Install act: `brew install act`

**Docker not running**

Start Docker Desktop or Docker daemon

**Out of memory errors**

Reduce memory limit in `.actrc` or close other applications

**Slow first run**

Normal - downloading Docker images. Subsequent runs are faster.

**Container cleanup**

```bash
# Remove act containers
docker container prune -f

# Remove act images (forces re-download)
docker image rm catthehacker/ubuntu:act-latest
```

## Pre-commit Hooks

BLZ uses Lefthook for git hooks with optional act integration.

### Bootstrap Development Environment

Quick setup with nextest and hooks:

```bash
# Install nextest and set up hooks
just bootstrap-fast
```

### Pre-push Hook (Enabled by Default)

Runs fast validation automatically before pushing:

```yaml
# lefthook.yml
pre-push:
  commands:
    act-validation:
      run: ./scripts/act-validate.sh fast rust-ci-local
```

Skip for a single push:

```bash
git push --no-verify
```

### Pre-commit Hook (Optional)

For even earlier feedback, enable act in pre-commit by uncommenting in `lefthook.yml`:

```yaml
pre-commit:
  commands:
    act-fast:
      run: ./scripts/act-validate.sh fast rust-ci-local
```

### Manual Validation

Run before creating PRs:

```bash
# Quick check
./scripts/act-validate.sh fast

# Thorough validation
./scripts/act-validate.sh full
```

## Test Coverage

### Generating Coverage Reports

```bash
# Install coverage tool
cargo install cargo-llvm-cov

# Generate HTML coverage report
cargo llvm-cov --html

# Open report
open target/llvm-cov/html/index.html

# Coverage summary
cargo llvm-cov
```

### Coverage Targets

- Minimum coverage: 80% overall
- Critical paths: 90% coverage
- New code: 100% coverage

### CI Coverage

Coverage is automatically generated for pull requests via GitHub Actions:

```yaml
# .github/workflows/coverage.yml
- name: Generate coverage
  run: cargo llvm-cov --lcov --output-path lcov.info
```

## Documentation Link Checking

Run [Lychee](https://github.com/lycheeverse/lychee) to catch broken links before they land in `main`:

```bash
# Via Just (installs lychee if needed)
just link-check

# Direct invocation
lychee --no-progress README.md docs .agents/rules .agents/instructions AGENTS.md
```

`just link-check` is included in the local `just ci` recipe and the `install-tools` bootstrap installs `lychee` automatically.

## Best Practices

### For Development

1. **Use fast mode during development** - Quick feedback loop
2. **Run tests frequently** - Catch issues early
3. **Write tests alongside code** - Test-driven development
4. **Keep tests isolated** - No shared state between tests

### Before Pushing

1. **Run full validation** - `./scripts/act-validate.sh full`
2. **Fix all issues** - Don't push with known failures
3. **Update tests** - Ensure new code is tested
4. **Check coverage** - Maintain or improve coverage

### For PRs

1. **All tests pass** - Green CI required for merge
2. **Coverage maintained** - No coverage regression
3. **Benchmarks checked** - No performance regression
4. **Act validation passed** - Document in PR if relevant

### Don't Skip on Critical Branches

Always validate on `main` and release branches:

```bash
# Never use --no-verify on main
git push origin main  # Will run hooks
```

## CI/CD Workflow Comparison

| Check | Local (act) | GitHub Actions | Time |
|-------|------------|----------------|------|
| Format | ✅ Fast mode | ✅ Always | <5s |
| Clippy (basic) | ✅ Fast mode | ✅ Always | 10-15s |
| Clippy (full) | ✅ Full mode | ✅ Always | 30-45s |
| Build | ✅ Full mode | ✅ Always | 45-60s |
| Tests | ✅ Full/test mode | ✅ Always | 30-45s |
| Miri | ❌ Too slow | ✅ Nightly | 30-60min |
| Coverage | ❌ Not local | ✅ PR only | 2-3min |

## Agent Instructions

For AI agents working with this repository:

### Before Making Changes

1. Run format check: `cargo fmt --check`
2. Run fast validation: `./scripts/act-validate.sh fast`

### Before Creating PRs

1. Run full validation: `./scripts/act-validate.sh full`
2. Fix any issues found
3. Document in PR if act validation passed

### Debugging CI Failures

1. Reproduce locally: `./scripts/act-validate.sh full`
2. Use verbose mode: `VERBOSE=1 ./scripts/act-validate.sh full`
3. Check specific job: `act -j rust -W .github/workflows/rust-ci.yml`

### Performance Expectations

- Fast mode: Should complete in <30 seconds
- Full mode: Should complete in <5 minutes
- If slower, check Docker resources and system load

## Configuration Files

- **`.actrc`**: Act configuration (platform, resources, defaults)
- **`.github/workflows/act-event.json`**: Default event for act
- **`.github/workflows/rust-ci-local.yml`**: Optimized workflow for act
- **`scripts/act-validate.sh`**: Validation script with modes
- **`lefthook.yml`**: Git hooks configuration with act integration

## See Also

- [CI/CD Pipeline](ci_cd.md) - Continuous integration and deployment
- [Contributing](./contributing.md) - How to contribute
- [Development Workflow](./workflow.md) - Development process
- [Act Documentation](https://github.com/nektos/act)
- [Nextest Documentation](https://nexte.st/)
