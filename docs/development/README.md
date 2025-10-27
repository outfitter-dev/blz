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

## ⚡ Build Performance Optimization

### Compilation Speed

BLZ uses several techniques to optimize build and test performance:

#### Profile Optimizations

The workspace is configured with optimized build profiles in `Cargo.toml`:

- **Dev profile**: Incremental compilation enabled, dependencies optimized at level 2
- **Test profile**: Incremental compilation with opt-level 1 for faster test execution
- **Release profile**: Full LTO and single codegen unit for maximum runtime performance

These settings significantly reduce compilation time while maintaining good runtime performance during development. Host/OS-specific tweaks (e.g., `target-cpu=native` on macOS) remain in `.cargo/config.toml`.

#### Shared Compilation Cache (sccache)

[sccache](https://github.com/mozilla/sccache) provides shared compilation caching to dramatically reduce rebuild times. **Optional but recommended:** builds work without sccache, but enabling it accelerates rebuilds significantly—especially on repeat clippy/test runs.

**Installation (once):**

```bash
# macOS
brew install sccache

# Arch Linux
pacman -S sccache

# Cargo (universal)
cargo install sccache
```

**Setup (per shell):**

Add to your shell configuration file (`~/.bashrc`, `~/.zshrc`, or `~/.config/fish/config.fish`):

```bash
# Bash/Zsh
export RUSTC_WRAPPER=sccache

# Fish
set -gx RUSTC_WRAPPER sccache
```

**Verify it's working:**

```bash
sccache --show-stats
```

You should see cache hits increase as you rebuild the project.

**Performance Impact:**

- Cold compile of the entire workspace: ~8½ minutes (measured 2025-10-19 without cache)
- With warm sccache: ~2-3 minutes (60-70% faster once cache is primed)
- Incremental builds: typically <30 seconds

#### Build Timings Analysis

To identify slow dependencies and compilation bottlenecks:

```bash
# Generate HTML report of build times
cargo build --timings

# Open the generated report
open target/cargo-timings/cargo-timing.html
```

This creates a detailed timeline showing:

- Which crates take longest to compile
- Dependency graph and parallel compilation opportunities
- CPU utilization during the build

Use this to identify optimization opportunities or problematic dependencies.

### Parallel Test Execution

Use [cargo-nextest](https://nexte.st/) for faster test runs:

```bash
# Install
cargo install cargo-nextest

# Run tests (automatically uses all CPU cores)
cargo nextest run --workspace

# Run with coverage
cargo llvm-cov nextest --workspace
```

Nextest runs tests in parallel by default and provides better output formatting.

### Reducing Target Directory Bloat

The `target/` directory can grow quickly—on 2025-10-19 a long-lived checkout measured ~86 GB with the heaviest paths at `target/debug` (~43 GB), `target/llvm-cov-target` (~2.9 GB), and `target/tests` (~1.1 GB). Clean it periodically:

Use the helper script to inspect and prune bloat:

```bash
# Summarize sizes and warn if they exceed 8 GB
scripts/prune-target.sh --check

# Drop incremental/debug caches (fast rebuild once sccache warms)
scripts/prune-target.sh --prune-debug

# Remove coverage + test artefacts (prompts before deleting)
scripts/prune-target.sh --prune

# Full reset (equivalent to cargo clean, but safer prompts)
scripts/prune-target.sh --prune-all

# Optional: pair any prune with cargo-sweep cleanup (requires cargo-sweep installed)
scripts/prune-target.sh --prune-debug --sweep
```

Most integration tests already run in temporary directories via `tempfile::tempdir()`, so they clean up cleanly and keep `~/.blz` untouched. Large leftovers usually come from cached build artefacts rather than the tests themselves. The pruning script also powers automated warnings in local git hooks when the cache grows past the configured threshold.

Consider using [cargo-sweep](https://github.com/holmgr/cargo-sweep) to automatically remove old artifacts:

```bash
cargo install cargo-sweep

# Mark current files as used
cargo sweep -s

# Remove unused artifacts older than 30 days
cargo sweep -f -t 30
```

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

## 🌳 Git Worktrees Support

BLZ automatically optimizes build performance when using git worktrees by sharing compilation artifacts across all worktrees.

### What Are Git Worktrees?

Git worktrees let you check out multiple branches simultaneously in separate directories:

```bash
# Create a worktree for a new feature
git worktree add ../blz-feature-x feature-branch

# Work in both directories without switching branches
cd ../blz-feature-x
cargo build  # Uses shared target directory
```

### Automatic Shared Target Configuration

When the setup scripts detect multiple worktrees, they automatically:

1. **Set `CARGO_TARGET_DIR`** to `<repo-root>/target-shared/`
2. **Share compilation artifacts** across all worktrees
3. **Reduce disk usage** by 40-50% (from ~6.6GB to ~3-4GB)
4. **Speed up builds** with shared incremental compilation

**Example disk usage:**

```
Without shared target:
  main/target/       4.2GB
  feature-x/target/  2.4GB
  Total:             6.6GB

With shared target:
  target-shared/     3.1GB  (shared across all worktrees)
  main/target/       (empty, can be removed)
  feature-x/target/  (empty, can be removed)
  Total:             3.1GB (53% reduction)
```

### Manual Configuration

The setup is automatic when using `scripts/setup-agent-conductor.sh` or `scripts/setup-agent-universal.sh`, but you can configure manually:

```bash
# Enable shared target for current shell
export CARGO_TARGET_DIR="$(git rev-parse --show-toplevel)/target-shared"

# Or add to your shell rc file (~/.bashrc, ~/.zshrc, etc.)
echo 'export CARGO_TARGET_DIR="$HOME/Developer/outfitter/blz/target-shared"' >> ~/.zshrc
```

### Managing Shared Target Space

Use the dedicated pruning script for shared targets:

```bash
# Check shared target size
scripts/prune-shared-target.sh --check

# Remove debug caches (safe, fast to rebuild with sccache)
scripts/prune-shared-target.sh --prune-debug

# Full clean (requires full rebuild)
scripts/prune-shared-target.sh --prune-all
```

The regular `scripts/prune-target.sh` automatically detects shared targets and provides guidance:

```bash
scripts/prune-target.sh --check
# Output:
# ℹ️  Shared target detected for git worktrees
#    Shared: /Users/you/blz/target-shared
#    Local:  /Users/you/blz/target
#
#    Shared target size: 3.1GB
#    Manage with: scripts/prune-shared-target.sh
```

### Cleanup After Worktree Removal

When you remove worktrees and no longer need the shared target:

```bash
# Remove all worktrees
git worktree remove feature-x

# Clean up shared target
rm -rf target-shared/

# Unset environment variable
unset CARGO_TARGET_DIR
# (or remove from shell rc file)
```

### Best Practices with Worktrees

1. **Use conductor agent**: `scripts/setup-agent-conductor.sh` handles everything automatically
2. **Enable sccache**: Shared target + sccache = maximum speed with minimum disk usage
3. **Monitor size**: Pre-push hooks check both local and shared targets for bloat
4. **Clean old locals**: After switching to shared target, remove old per-worktree `target/` directories:

   ```bash
   cd path/to/worktree
   scripts/prune-target.sh --prune-all
   ```

### Compatibility Notes

- **Remote agents** (Factory, Codex, Devin): Automatically use per-workspace targets (no worktrees in containers)
- **CI environments**: Use standard `target/` directory (no worktrees in CI)
- **Manual override**: Set `CARGO_TARGET_DIR` to any path to override auto-detection
- **Disable sharing**: `unset CARGO_TARGET_DIR` to revert to per-worktree targets

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
```

The script wraps `cargo install --features dev-profile --bin blz-dev --path crates/blz-cli` and passes through any extra flags you supply (`--root`, `--force`, `--locked`, etc).

After installation, add the target `bin` directory to your PATH *ahead* of other blz binaries:

```bash
export PATH="$HOME/.local/share/blz-dev/bin:$PATH"
```

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
```

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
```

Be careful to double-check paths before running the commands above.

## 🤝 Getting Help

- **Issues**: [GitHub Issues](https://github.com/outfitter-dev/blz/issues)
- **Documentation**: Check `.agents/rules/` for detailed development rules

## 📜 License

This project is licensed under MIT OR Apache-2.0. See [LICENSE](../../LICENSE) for details.
