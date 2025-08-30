# Development Environment

## Required Tools

### Core Toolchain

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup component add clippy rustfmt rust-src
```

### Security & Dependency Tools

```bash
cargo install cargo-deny      # License and vulnerability checking
cargo install cargo-shear     # Unused dependency detection
cargo install cargo-audit     # Security auditing (optional - deny covers this)
cargo install cargo-outdated  # Check for outdated dependencies
```

### Development Tools

```bash
cargo install cargo-watch     # File watching for auto-rebuild
cargo install flamegraph      # Performance profiling
cargo install cargo-llvm-cov  # Code coverage with LLVM
```

### Editor Setup

**Required:**

- **rust-analyzer**: Language server for IDE support
- **Rust syntax highlighting**: For your preferred editor

**Recommended:**

- **CodeLLDB** or **GDB**: Debugging support
- **Even Better TOML**: TOML file support
- **Markdown linting**: For documentation

## Workspace Configuration

### Root Cargo.toml Structure

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/outfitter-dev/blz"
rust-version = "1.75.0"

[workspace.dependencies]
# Core dependencies shared across crates
tantivy = "0.22"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"

# Development dependencies
criterion = "0.5"
tempfile = "3.0"
pretty_assertions = "1.4"
```

### Workspace Lints

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
all = "deny"
pedantic = "deny"
nursery = "deny"
cargo = "deny"
# Exceptions
module_name_repetitions = "allow"
missing_errors_doc = "allow"
```

## Environment Variables

### Development

```bash
# Enable debug logging
export RUST_LOG=debug

# Enable backtrace for errors
export RUST_BACKTRACE=1

# Set test parallelism
export RUST_TEST_THREADS=4
```

### Performance Profiling

```bash
# Enable flamegraph symbols
export CARGO_PROFILE_RELEASE_DEBUG=true

# Set perf permissions (Linux)
echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid
```

## Platform-Specific Setup

### macOS

```bash
# Install Xcode command line tools
xcode-select --install

# For profiling
brew install flamegraph
```

### Linux

```bash
# Install build essentials
sudo apt-get install build-essential pkg-config libssl-dev

# For profiling
sudo apt-get install linux-tools-common linux-tools-generic
```

### Windows

```powershell
# Install Visual Studio Build Tools
# Download from: https://visualstudio.microsoft.com/downloads/

# Install LLVM for better debugging
winget install LLVM.LLVM
```

## VS Code Configuration

See `.vscode/settings.json` for project-specific settings.

Key configurations:

- Rust analyzer with Clippy integration
- Format on save enabled
- Recommended extensions in `.vscode/extensions.json`

## Quick Verification

After setup, verify your environment:

```bash
# Check Rust installation
rustc --version
cargo --version

# Check required tools
cargo deny --version
cargo shear --version

# Run basic checks
cargo build
cargo test
cargo clippy
```
