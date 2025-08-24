.PHONY: help install-tools check-deps security audit deny unused clean test build release lint ci

# Default target
help:
	@echo "Available targets:"
	@echo "  install-tools  - Install cargo-deny and cargo-shear"
	@echo "  check-deps     - Run all dependency checks"
	@echo "  security       - Run security advisory checks"
	@echo "  audit          - Alias for security"
	@echo "  deny           - Run full cargo-deny validation"
	@echo "  unused         - Check for unused dependencies"
	@echo "  clean          - Clean build artifacts"
	@echo "  test           - Run all tests"
	@echo "  build          - Build release binaries"
	@echo "  release        - Build optimized release"
	@echo "  ci             - Run full CI validation locally"

# Install required tools
install-tools:
	@echo "Installing dependency management tools..."
	@command -v cargo-deny >/dev/null 2>&1 || cargo install cargo-deny
	@command -v cargo-shear >/dev/null 2>&1 || cargo install cargo-shear
	@echo "Tools installed successfully"

# Check all dependencies
check-deps: unused deny
	@echo "All dependency checks passed"

# Security advisory checks (non-blocking)
security:
	@echo "Checking for security advisories..."
	@cargo deny check advisories || echo "⚠️  Security advisories found (see above)"

# Alias for security
audit: security

# Full cargo-deny validation
deny:
	@echo "Running cargo-deny checks..."
	cargo deny check

# Check for unused dependencies
unused:
	@echo "Checking for unused dependencies..."
	cargo shear

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/

# Run all tests
test:
	cargo test --all-features --workspace

# Build release binaries
build:
	cargo build --release --all-features

# Build optimized release
release: clean
	RUSTFLAGS="-C target-cpu=native" cargo build --release --all-features
	@echo "Release binaries built in target/release/"
	@ls -lh target/release/blz target/release/blz-mcp

# Run full CI validation locally
ci: check-deps test
        @$(MAKE) lint
        @echo "Checking formatting..."
        cargo fmt -- --check
        @echo "Building documentation..."
        cargo doc --no-deps --all-features
        @echo "CI validation complete"

lint:
        cargo clippy --all-targets --all-features -- -D warnings

# Quick security check
.PHONY: quick-security
quick-security:
	@cargo deny check advisories --hide-inclusion-graph

# Update Cargo.lock with latest compatible versions
.PHONY: update
update:
	cargo update
	@echo "Dependencies updated. Run 'make check-deps' to validate."

# Check for outdated dependencies
.PHONY: outdated
outdated:
	@command -v cargo-outdated >/dev/null 2>&1 || cargo install cargo-outdated
	cargo outdated

# Generate dependency tree
.PHONY: tree
tree:
	cargo tree --all-features

# Check specific deny categories
.PHONY: check-licenses check-bans check-sources
check-licenses:
	cargo deny check licenses

check-bans:
	cargo deny check bans

check-sources:
	cargo deny check sources