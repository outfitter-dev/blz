# justfile - Modern command runner for blz development
# https://github.com/casey/just

# Default recipe shows help
default:
    @just --list

# Install required development tools
install-tools:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Installing dependency management tools..."
    command -v cargo-deny >/dev/null 2>&1 || cargo install cargo-deny
    command -v cargo-shear >/dev/null 2>&1 || cargo install cargo-shear
    command -v cargo-nextest >/dev/null 2>&1 || cargo install cargo-nextest
    command -v sccache >/dev/null 2>&1 || cargo install sccache
    command -v commitlint >/dev/null 2>&1 || cargo install commitlint-rs
    echo "✅ Tools installed successfully"

# Run all dependency checks
check-deps: unused deny
    @echo "✅ All dependency checks passed"

# Check for security advisories (non-blocking)
security:
    @echo "🔍 Checking for security advisories..."
    @cargo deny check advisories || echo "⚠️  Security advisories found (see above)"

# Alias for security check
audit: security

# Run full cargo-deny validation
deny:
    @echo "🔍 Running cargo-deny checks..."
    cargo deny check

# Check for unused dependencies
unused:
    @echo "🔍 Checking for unused dependencies..."
    cargo shear

# Run unused deps check with auto-fix
fix-unused:
    @echo "🔧 Removing unused dependencies..."
    cargo shear --fix

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/

# Run all tests
test:
    cargo test --all-features --workspace

# Run tests with coverage (requires cargo-llvm-cov)
test-coverage:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
        echo "Installing cargo-llvm-cov..."
        cargo install cargo-llvm-cov
    fi
    cargo llvm-cov --all-features --workspace --html
    echo "📊 Coverage report generated in target/llvm-cov/html/index.html"

# Build debug binaries
build:
    cargo build --all-features

# Build release binaries
release: clean
    RUSTFLAGS="-C target-cpu=native" cargo build --release --all-features
    @echo "📦 Release binaries built:"
    @ls -lh target/release/blz target/release/blz-mcp

# Run clippy lints
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Run full CI validation locally
ci: check-deps lint fmt-check test
    @echo "📖 Building documentation..."
    cargo doc --no-deps --all-features
    @echo "✅ CI validation complete"

# Quick security check without graphs
quick-security:
    @cargo deny check advisories --hide-inclusion-graph

# Update dependencies to latest compatible versions
update:
    cargo update
    @echo "📦 Dependencies updated. Run 'just check-deps' to validate."

# Check for outdated dependencies
outdated:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v cargo-outdated >/dev/null 2>&1; then
        echo "Installing cargo-outdated..."
        cargo install cargo-outdated
    fi
    cargo outdated

# Show dependency tree
tree:
    cargo tree --all-features

# Show dependency tree for a specific package
tree-pkg package:
    cargo tree -p {{package}}

# Check licenses only
check-licenses:
    cargo deny check licenses

# Check for banned dependencies
check-bans:
    cargo deny check bans

# Check dependency sources
check-sources:
    cargo deny check sources

# Run benchmarks
bench:
    cargo bench --all-features

# Run a specific benchmark
bench-specific name:
    cargo bench --bench {{name}}

# Watch for changes and run tests
watch:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v cargo-watch >/dev/null 2>&1; then
        echo "Installing cargo-watch..."
        cargo install cargo-watch
    fi
    cargo watch -x test

# Generate shell completions for development
completions:
    cargo run --bin blz -- completions bash > completions/blz.bash
    cargo run --bin blz -- completions fish > completions/blz.fish
    cargo run --bin blz -- completions zsh > completions/_blz

# Install blz locally for testing
install-local:
    cargo install --path crates/blz-cli --force
    @echo "✅ blz installed to ~/.cargo/bin/blz"

# Run the MCP server for testing
run-mcp:
    cargo run --bin blz-mcp

# Run with debug logging
debug *args:
    RUST_LOG=debug cargo run --bin blz -- {{args}}

# Run with trace logging (very verbose)
trace *args:
    RUST_LOG=trace cargo run --bin blz -- {{args}}

# Agent scripts: generate timestamps
get-date *args:
    # Call the UTC/local timestamp helper (YYYYMMDDHHmm)
    ./.agents/scripts/get-date.sh {{args}}

# Agent scripts: create a new log from template
new-log *args:
    # Forward arguments to the log generator
    ./.agents/scripts/new-log.sh {{args}}

# Branchwork helper
branchwork *args:
    ./.agents/scripts/branchwork.sh {{args}}

# Check the codebase for common issues
check:
    @echo "🔍 Checking for common issues..."
    @echo "Checking for TODO comments..."
    @rg "TODO|FIXME|HACK|XXX" --type rust || echo "No TODOs found"
    @echo "Checking for unwrap() calls..."
    @rg "\.unwrap\(\)" --type rust || echo "No unwrap() calls found"
    @echo "Checking for println! in library code..."
    @rg "println!" crates/blz-core/src || echo "No println! in library"

# Create a new release (requires cargo-release)
release-prep version:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v cargo-release >/dev/null 2>&1; then
        echo "Please install cargo-release: cargo install cargo-release"
        exit 1
    fi
    cargo release version {{version}}
    cargo release changes

# Fast bootstrap: install tools, enable sccache, install hooks
bootstrap-fast:
    scripts/bootstrap-fast.sh

# Control strict push bypass
strict-bypass action="status":
    scripts/hooks-bypass.sh {{action}}
