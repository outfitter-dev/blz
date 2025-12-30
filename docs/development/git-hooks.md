# Git Hooks Performance Guide

This document explains the git hook configuration, performance optimizations, and bypass mechanisms in the blz repository.

## Overview

The repository uses [lefthook](https://github.com/evilmartians/lefthook) to manage git hooks that enforce code quality standards. The hooks are designed to:

- **Pre-commit**: Fast hygiene checks (<30s target)
- **Pre-push**: Comprehensive quality gates (<3 minutes with caching)

## Hook Configuration

### Pre-commit Hook (~30 seconds)

Runs automatically before each commit with parallel execution:

- **YAML formatting** (`yamlfmt`) - Formats GitHub Actions workflows
- **Action linting** (`actionlint`) - Validates GitHub Actions syntax
- **Whitespace checks** - Detects trailing whitespace
- **Rust formatting** (`cargo fmt`) - Auto-formats Rust code
- **Rust compile + basic clippy** (`cargo check` + `cargo clippy --workspace`) - Skips when no Rust changes are staged, or only Rust tests/benches/examples change; scopes to `blz-cli` when only `crates/blz-cli/**` changed; emits output to avoid “hang” perception

### Pre-push Hook (~3 minutes with caching)

Runs before pushing to remote with strict quality gates:

- **Strict clippy** (`cargo clippy --all-targets -- -D warnings`) - Zero-tolerance for warnings
- **Full test suite** - All tests except slow compile-time UI tests
  - Uses `cargo-nextest` for parallel execution when available
  - Skips `compile_fail_ui` test by default (120+ seconds, CI-only)

## Performance Optimizations

### 1. sccache (Compilation Caching)

The hooks automatically use [sccache](https://github.com/mozilla/sccache) when available for 2-3x faster compilation:

```bash
# Install sccache
cargo install sccache

# Or use bootstrap script
./scripts/bootstrap-fast.sh
```

**How it works:**

- Hooks check for `sccache` and set `RUSTC_WRAPPER=sccache` automatically
- For non-hook workflows (manual `cargo` runs, IDE integration), export `RUSTC_WRAPPER=sccache` in your shell rc file
- No `.cargo/config.toml` modification is required for cache usage, which keeps remote container and worktree setups happy
- sccache caches compilation artifacts locally (default: `~/.cache/sccache`)

**Benefits:**

- First run: Normal compilation time
- Subsequent runs: 2-3x faster (50-70% cache hit rate typical)
- Especially effective for clippy + tests since they share compilation artifacts
- Hooks also surface cache-bloat warnings via `scripts/prune-target.sh --check`, nudging you to prune oversized `target/` directories (8 GB default threshold). For large debug caches, run `scripts/prune-target.sh --prune-debug [--sweep]`.

### 2. cargo-nextest (Parallel Test Execution)

[cargo-nextest](https://nexte.st/) runs tests in parallel for 2-3x faster execution:

```bash
# Install cargo-nextest
cargo install cargo-nextest

# Or use bootstrap script
./scripts/bootstrap-fast.sh
```

**Benefits:**

- Parallel test execution across all CPU cores
- Better test output and failure reporting
- Automatically used by pre-push hook when available

### 3. Incremental Compilation

The workspace `Cargo.toml` enables incremental compilation (and dependency opt-level tuning) for dev and test profiles. Host-specific overrides, such as `target-cpu=native` for macOS, remain in `.cargo/config.toml`. Together they ensure fast rebuilds without sacrificing per-platform optimizations.

### 4. Excluded Slow Tests

The `compile_fail_ui` test is excluded from git hooks by default:

- **Why**: Takes 120+ seconds to compile test cases
- **Purpose**: Validates compile-time API constraints (not graphical UI)
- **When to run**: Explicitly with `cargo test --ignored compile_fail_ui`
- **CI**: Should run in CI pipeline with full test suite

## Bootstrap Setup

Run the bootstrap script to install all performance tools:

```bash
./scripts/bootstrap-fast.sh
```

This script:

1. Installs and configures lefthook
2. Installs cargo-nextest for fast tests
3. Installs sccache for build caching
4. Starts sccache server
5. Installs commitlint-rs for commit message validation
6. Ensures rustfmt and clippy are available
7. Runs pre-commit once to prime caches

## Bypass Mechanism

For emergency situations where you need to push without running full checks:

```bash
# Enable bypass (with confirmation)
./scripts/hooks-bypass.sh enable

# Enable bypass without confirmation (for scripts)
./scripts/hooks-bypass.sh enable --force

# Check bypass status
./scripts/hooks-bypass.sh status

# Disable bypass (restore normal behavior)
./scripts/hooks-bypass.sh disable
```

**Important:**

- The bypass file (`.hooks/allow-strict-bypass`) is git-ignored
- Remember to disable bypass after your emergency push
- Both clippy and tests are skipped when bypass is enabled

## Performance Targets

With all optimizations enabled:

| Stage | Target | First Run | Cached Run |
|-------|--------|-----------|------------|
| Pre-commit | <30s | ~20-30s | ~15-20s |
| Pre-push | <3min | ~5-10min | ~2-3min |

**Factors affecting performance:**

- Number of files changed
- sccache cache state
- CPU cores available
- Disk I/O speed

## Troubleshooting

### Hooks are slow

1. **Install sccache:**

   ```bash
   cargo install sccache
   sccache --start-server
   ```

2. **Install cargo-nextest:**

   ```bash
   cargo install cargo-nextest
   ```

3. **Check sccache stats:**

   ```bash
   sccache --show-stats
   ```

   Look for high cache hit rates. Low hit rates suggest cache directory issues.

4. **Clear sccache if needed:**

   ```bash
   sccache --stop-server
   rm -rf ~/.cache/sccache
   sccache --start-server
   ```

### sccache not working in remote containers

This is expected. The hooks gracefully degrade:

- They check for sccache availability before using it
- They show helpful tip messages when sccache is not available
- Compilation still works, just slower

### Git worktree issues

The configuration avoids setting `rustc-wrapper` in `.cargo/config.toml` to prevent worktree issues. Instead, hooks set `RUSTC_WRAPPER` environment variable per-run.

### Bypass not working

Check if the bypass file exists:

```bash
ls -la .hooks/allow-strict-bypass
```

If it exists but hooks still run, ensure you're using the latest lefthook:

```bash
lefthook install
```

## CI/CD Recommendations

For CI/CD pipelines:

1. **Enable full test suite including UI tests:**

   ```bash
   cargo test --workspace  # Includes ignored tests in CI
   ```

2. **Use sccache with shared storage:**

   ```bash
   # S3 backend (AWS)
   export SCCACHE_BUCKET=my-cache-bucket
   export SCCACHE_REGION=us-east-1
   
   # Redis backend (self-hosted)
   export SCCACHE_REDIS=redis://cache.example.com
   
   cargo test --workspace
   ```

3. **Cache cargo and sccache directories:**

   ```yaml
   # GitHub Actions example
   - uses: actions/cache@v3
     with:
       path: |
         ~/.cargo/bin/
         ~/.cargo/registry/
         ~/.cargo/git/
         ~/.cache/sccache/
       key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
   ```

## Further Reading

- [lefthook documentation](https://github.com/evilmartians/lefthook/blob/master/docs/usage.md)
- [sccache documentation](https://github.com/mozilla/sccache)
- [cargo-nextest documentation](https://nexte.st/)
- [Rust incremental compilation](https://blog.rust-lang.org/2016/09/08/incremental.html)
