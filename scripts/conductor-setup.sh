#!/usr/bin/env bash
set -euo pipefail

# Conductor workspace setup for blz
# This script assumes the base repository tooling (Rust, etc.) is already installed
# It validates environment, installs workspace-specific dependencies, and runs initial checks

echo "🔧 Setting up blz workspace in Conductor..."
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_step() {
    echo -e "${BLUE}▶${NC} $1"
}

log_success() {
    echo -e "  ${GREEN}✓${NC} $1"
}

log_warning() {
    echo -e "  ${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "  ${RED}✗${NC} $1"
}

has_cmd() {
    command -v "$1" >/dev/null 2>&1
}

# Track failures
CRITICAL_FAILURES=0
WARNINGS=0

# ============================================================================
# STEP 1: Verify we're in the right place
# ============================================================================
log_step "1/9 Verifying workspace structure..."
if [ ! -f "Cargo.toml" ]; then
    log_error "Cargo.toml not found. Are we in the workspace root?"
    exit 1
fi
if [ ! -f "lefthook.yml" ]; then
    log_error "lefthook.yml not found. Are we in the blz repository?"
    exit 1
fi
log_success "Workspace structure verified"

# ============================================================================
# STEP 2: Check Rust toolchain (CRITICAL - fail fast)
# ============================================================================
log_step "2/9 Checking Rust toolchain..."
if ! has_cmd cargo; then
    log_error "Rust/cargo not found in PATH"
    echo ""
    echo "The base repository should have Rust installed."
    echo "Install from: https://rustup.rs"
    echo ""
    exit 1
fi

RUST_VERSION=$(rustc --version | cut -d' ' -f2)
REQUIRED_RUST="1.85.0"
log_success "Found Rust $RUST_VERSION (required: $REQUIRED_RUST)"

# Check for rustfmt and clippy
if has_cmd rustfmt; then
    log_success "rustfmt available"
else
    log_warning "rustfmt not found - installing via rustup"
    rustup component add rustfmt || ((WARNINGS++))
fi

if has_cmd cargo-clippy || cargo clippy --version >/dev/null 2>&1; then
    log_success "clippy available"
else
    log_warning "clippy not found - installing via rustup"
    rustup component add clippy || ((WARNINGS++))
fi

# ============================================================================
# STEP 3: Check Go tools (yamlfmt, actionlint) - OPTIONAL but used by hooks
# ============================================================================
log_step "3/9 Checking Go-based linting tools..."
if has_cmd yamlfmt; then
    log_success "yamlfmt found: $(which yamlfmt)"
else
    log_warning "yamlfmt not found (used by pre-commit hooks)"
    echo "         Install: go install github.com/google/yamlfmt/cmd/yamlfmt@v0.10.0"
    ((WARNINGS++))
fi

if has_cmd actionlint; then
    log_success "actionlint found: $(which actionlint)"
else
    log_warning "actionlint not found (used by pre-commit hooks)"
    echo "         Install: go install github.com/rhysd/actionlint/cmd/actionlint@latest"
    ((WARNINGS++))
fi

if has_cmd shellcheck; then
    log_success "shellcheck found (enhances actionlint)"
else
    log_warning "shellcheck not found (optional, enhances actionlint diagnostics)"
    ((WARNINGS++))
fi

# ============================================================================
# STEP 4: Check markdown/doc formatting tools - OPTIONAL
# ============================================================================
log_step "4/9 Checking markdown and document formatting tools..."

# Check markdownlint-cli2
if has_cmd markdownlint-cli2; then
    log_success "markdownlint-cli2 found: $(which markdownlint-cli2)"
elif has_cmd npx; then
    log_success "npx available (will use as markdownlint-cli2 fallback)"
elif has_cmd bunx; then
    log_success "bunx available (will use as markdownlint-cli2 fallback)"
else
    log_warning "No markdown linter found (markdownlint-cli2, npx, or bunx)"
    echo "         Install: npm install -g markdownlint-cli2 markdownlint-cli2-formatter-default"
    echo "         (This is optional - markdown files may not be auto-fixed)"
    ((WARNINGS++))
fi

# Check Prettier (for document formatting)
if has_cmd prettier; then
    log_success "prettier found: $(which prettier)"
elif has_cmd npx; then
    log_success "npx available (will use as prettier fallback)"
elif has_cmd bunx; then
    log_success "bunx available (will use as prettier fallback)"
else
    log_warning "No prettier found (prettier, npx, or bunx)"
    echo "         Install: npm install -g prettier"
    echo "         (This is optional - used for doc formatting)"
    ((WARNINGS++))
fi

# Check Bun (used by this project for various things)
if has_cmd bun; then
    log_success "bun found: $(which bun)"
else
    log_warning "bun not found (used by some project scripts)"
    echo "         Install: curl -fsSL https://bun.sh/install | bash"
    echo "         (This is optional but recommended for this project)"
    ((WARNINGS++))
fi

# ============================================================================
# STEP 5: Check/Install lefthook (CRITICAL for git hooks)
# ============================================================================
log_step "5/9 Setting up git hooks with lefthook..."
if ! has_cmd lefthook; then
    log_warning "lefthook not found - installing via cargo"
    if cargo install lefthook --quiet; then
        log_success "lefthook installed"
    else
        log_error "Failed to install lefthook"
        ((CRITICAL_FAILURES++))
    fi
else
    log_success "lefthook found: $(which lefthook)"
fi

# Install hooks
if has_cmd lefthook; then
    if lefthook install; then
        log_success "Git hooks installed"
    else
        log_warning "Failed to install git hooks (non-fatal)"
        ((WARNINGS++))
    fi
fi

# ============================================================================
# STEP 6: Install critical cargo tools
# ============================================================================
log_step "6/9 Installing critical cargo tools..."

# cargo-deny (dependency validation)
if has_cmd cargo-deny; then
    log_success "cargo-deny already installed"
else
    log_warning "Installing cargo-deny..."
    if cargo install cargo-deny --quiet; then
        log_success "cargo-deny installed"
    else
        log_warning "Failed to install cargo-deny"
        ((WARNINGS++))
    fi
fi

# cargo-shear (unused dependency detection)
if has_cmd cargo-shear; then
    log_success "cargo-shear already installed"
else
    log_warning "Installing cargo-shear..."
    if cargo install cargo-shear --quiet; then
        log_success "cargo-shear installed"
    else
        log_warning "Failed to install cargo-shear"
        ((WARNINGS++))
    fi
fi

# cargo-nextest (fast test runner)
if has_cmd cargo-nextest; then
    log_success "cargo-nextest already installed"
else
    log_warning "Installing cargo-nextest..."
    if cargo install cargo-nextest --quiet; then
        log_success "cargo-nextest installed"
    else
        log_warning "Failed to install cargo-nextest (will use 'cargo test' instead)"
        ((WARNINGS++))
    fi
fi

# sccache (build caching)
if has_cmd sccache; then
    log_success "sccache already installed"
else
    log_warning "Installing sccache (speeds up builds)..."
    if cargo install sccache --quiet; then
        log_success "sccache installed"
        # Configure sccache in workspace .cargo/config.toml
        mkdir -p .cargo
        if ! grep -q 'rustc-wrapper = "sccache"' .cargo/config.toml 2>/dev/null; then
            echo "" >> .cargo/config.toml
            echo "[build]" >> .cargo/config.toml
            echo 'rustc-wrapper = "sccache"' >> .cargo/config.toml
            log_success "sccache configured in .cargo/config.toml"
        fi
    else
        log_warning "Failed to install sccache (builds will be slower)"
        ((WARNINGS++))
    fi
fi

# commitlint-rs (commit message validation)
if has_cmd commitlint; then
    log_success "commitlint already installed"
else
    log_warning "Installing commitlint-rs..."
    if cargo install commitlint-rs --quiet; then
        log_success "commitlint-rs installed"
    else
        log_warning "Failed to install commitlint-rs (commit-msg hook may fail)"
        ((WARNINGS++))
    fi
fi

# cargo-watch (needed for run script)
if has_cmd cargo-watch; then
    log_success "cargo-watch already installed"
else
    log_warning "Installing cargo-watch (needed for run script)..."
    if cargo install cargo-watch --quiet; then
        log_success "cargo-watch installed"
    else
        log_warning "Failed to install cargo-watch (run script will not work)"
        ((WARNINGS++))
    fi
fi

# lychee (link checker - optional)
if has_cmd lychee; then
    log_success "lychee already installed"
else
    log_warning "lychee not found (optional, used for link checking)"
    echo "         Install: cargo install lychee"
    ((WARNINGS++))
fi

# ============================================================================
# STEP 7: Fetch dependencies and build
# ============================================================================
log_step "7/9 Fetching Rust dependencies..."
if cargo fetch --quiet; then
    log_success "Dependencies fetched"
else
    log_error "Failed to fetch dependencies"
    ((CRITICAL_FAILURES++))
fi

log_step "8/9 Building workspace..."
if cargo build --workspace --all-targets --quiet 2>&1 | head -20; then
    log_success "Build successful"
else
    log_warning "Build failed (workspace still usable, but may need fixes)"
    echo "         Check errors above or run 'cargo build' for details"
    echo "         You can still work in this workspace and fix build issues"
    ((WARNINGS++))
fi

# ============================================================================
# STEP 9: Run quick validation
# ============================================================================
log_step "9/9 Running validation checks..."

# Run a quick test to ensure everything works
if cargo test --workspace --lib --quiet 2>&1 | grep -q "test result: ok"; then
    log_success "Library tests passed"
else
    log_warning "Some library tests failed (check with: cargo test)"
    ((WARNINGS++))
fi

# Check formatting
if cargo fmt --all --check >/dev/null 2>&1; then
    log_success "Code formatting is correct"
else
    log_warning "Code needs formatting (run: cargo fmt --all)"
    ((WARNINGS++))
fi

# ============================================================================
# Summary
# ============================================================================
echo ""
echo "════════════════════════════════════════════════════════════════"

if [ $CRITICAL_FAILURES -eq 0 ]; then
    echo -e "${GREEN}✅ Workspace setup complete!${NC}"
    echo ""
    echo "📊 Summary:"
    echo "   - Rust toolchain: ✓"
    echo "   - Git hooks: ✓"
    echo "   - Dependencies: ✓"
    echo "   - Build: ✓"

    if [ $WARNINGS -gt 0 ]; then
        echo ""
        echo -e "${YELLOW}⚠  $WARNINGS warnings${NC} (optional tools not installed)"
        echo "   Your workspace will work, but some features may be limited."
        echo "   Review warnings above for installation instructions."
    fi

    echo ""
    echo "💡 Next steps:"
    echo "   • Click 'Run' button to start auto-reload development"
    echo "   • Run './scripts/agent-check.sh' for Rust diagnostics"
    echo "   • Run 'cargo test' to run all tests"
    echo "   • Check 'just --list' or 'make help' for available commands"

    exit 0
else
    echo -e "${RED}❌ Setup failed with $CRITICAL_FAILURES critical errors${NC}"
    echo ""
    echo "Critical issues that must be resolved:"
    echo "   - Check error messages above"
    echo "   - Ensure Rust is properly installed"
    echo "   - Verify network connectivity for cargo dependencies"
    echo ""
    exit 1
fi
