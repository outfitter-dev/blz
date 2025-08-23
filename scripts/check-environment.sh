#!/usr/bin/env bash
#
# check-environment.sh - Verify development environment for blz
# Can be run independently or sourced by other scripts

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Configuration
REQUIRED_RUST_VERSION="1.75.0"
REQUIRED_NODE_VERSION="18.0.0"  # For development tools
MIN_DISK_SPACE_MB=500
MIN_MEMORY_MB=1024

# Status tracking
CHECKS_PASSED=0
CHECKS_FAILED=0
CHECKS_WARNING=0

# Helper functions
check_pass() {
    echo -e "${GREEN}✓${NC} $1"
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
    CHECKS_FAILED=$((CHECKS_FAILED + 1))
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
    CHECKS_WARNING=$((CHECKS_WARNING + 1))
}

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

version_ge() {
    [ "$(printf '%s\n' "$2" "$1" | sort -V | head -n1)" = "$2" ]
}

# Check functions
check_os() {
    echo "Checking operating system..."
    
    case "$(uname -s)" in
        Linux*)
            check_pass "Linux detected"
            ;;
        Darwin*)
            check_pass "macOS detected"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            check_warn "Windows detected - some features may require WSL"
            ;;
        *)
            check_fail "Unknown operating system: $(uname -s)"
            ;;
    esac
}

check_rust() {
    echo "Checking Rust toolchain..."
    
    if command_exists rustc; then
        RUST_VERSION=$(rustc --version | cut -d' ' -f2)
        if version_ge "$RUST_VERSION" "$REQUIRED_RUST_VERSION"; then
            check_pass "Rust $RUST_VERSION (required: $REQUIRED_RUST_VERSION)"
        else
            check_fail "Rust $RUST_VERSION is too old (required: $REQUIRED_RUST_VERSION)"
        fi
    else
        check_fail "Rust is not installed"
    fi
    
    # Check cargo
    if command_exists cargo; then
        check_pass "Cargo is available"
    else
        check_fail "Cargo is not available"
    fi
    
    # Check important components
    if rustup component list --installed 2>/dev/null | grep -q clippy; then
        check_pass "Clippy is installed"
    else
        check_warn "Clippy is not installed (recommended)"
    fi
    
    if rustup component list --installed 2>/dev/null | grep -q rustfmt; then
        check_pass "Rustfmt is installed"
    else
        check_warn "Rustfmt is not installed (recommended)"
    fi
}

check_build_tools() {
    echo "Checking build tools..."
    
    # Check for C compiler (needed for some dependencies)
    if command_exists cc || command_exists gcc || command_exists clang; then
        check_pass "C compiler is available"
    else
        check_fail "No C compiler found (required for some dependencies)"
    fi
    
    # Check for pkg-config (needed on Linux)
    if [[ "$(uname -s)" == "Linux" ]]; then
        if command_exists pkg-config; then
            check_pass "pkg-config is available"
        else
            check_warn "pkg-config not found (may be needed for some dependencies)"
        fi
    fi
    
    # Check for make
    if command_exists make; then
        check_pass "Make is available"
    else
        check_warn "Make not found (optional for convenience commands)"
    fi
    
    # Check for just
    if command_exists just; then
        check_pass "Just is available"
    else
        check_warn "Just not found (optional alternative to make)"
    fi
}

check_cargo_tools() {
    echo "Checking cargo tools..."
    
    local tools=("cargo-deny" "cargo-shear" "cargo-watch" "cargo-llvm-cov")
    local required=("cargo-deny" "cargo-shear")
    
    for tool in "${tools[@]}"; do
        if command_exists "$tool"; then
            check_pass "$tool is installed"
        elif [[ " ${required[@]} " =~ " ${tool} " ]]; then
            check_fail "$tool is not installed (required)"
        else
            check_warn "$tool is not installed (optional)"
        fi
    done
}

check_disk_space() {
    echo "Checking disk space..."
    
    # Get available space in MB
    if [[ "$(uname -s)" == "Darwin" ]]; then
        AVAILABLE_MB=$(df -m . | awk 'NR==2 {print $4}')
    else
        AVAILABLE_MB=$(df -BM . | awk 'NR==2 {print $4}' | sed 's/M//')
    fi
    
    if [ "$AVAILABLE_MB" -ge "$MIN_DISK_SPACE_MB" ]; then
        check_pass "Disk space: ${AVAILABLE_MB}MB available (minimum: ${MIN_DISK_SPACE_MB}MB)"
    else
        check_fail "Insufficient disk space: ${AVAILABLE_MB}MB (minimum: ${MIN_DISK_SPACE_MB}MB)"
    fi
}

check_memory() {
    echo "Checking system memory..."
    
    # Get total memory in MB
    if [[ "$(uname -s)" == "Darwin" ]]; then
        TOTAL_MB=$(($(sysctl -n hw.memsize) / 1024 / 1024))
    elif [[ "$(uname -s)" == "Linux" ]]; then
        TOTAL_MB=$(free -m | awk 'NR==2 {print $2}')
    else
        # Skip on Windows/other
        check_warn "Cannot determine system memory"
        return
    fi
    
    if [ "$TOTAL_MB" -ge "$MIN_MEMORY_MB" ]; then
        check_pass "Memory: ${TOTAL_MB}MB total (minimum: ${MIN_MEMORY_MB}MB)"
    else
        check_warn "Low memory: ${TOTAL_MB}MB (recommended: ${MIN_MEMORY_MB}MB)"
    fi
}

check_network() {
    echo "Checking network connectivity..."
    
    # Check if we can reach crates.io
    if curl -s --head --connect-timeout 5 https://crates.io > /dev/null; then
        check_pass "Can reach crates.io"
    else
        check_fail "Cannot reach crates.io (needed for dependencies)"
    fi
    
    # Check if we can reach github.com
    if curl -s --head --connect-timeout 5 https://github.com > /dev/null; then
        check_pass "Can reach github.com"
    else
        check_warn "Cannot reach github.com (may affect some operations)"
    fi
}

check_permissions() {
    echo "Checking file permissions..."
    
    # Check if we can write to current directory
    if touch .test_write_permission 2>/dev/null; then
        rm .test_write_permission
        check_pass "Write permission in current directory"
    else
        check_fail "No write permission in current directory"
    fi
    
    # Check if cargo home is writable
    CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
    if [ -w "$CARGO_HOME" ]; then
        check_pass "Cargo home is writable"
    else
        check_fail "Cargo home is not writable: $CARGO_HOME"
    fi
}

print_summary() {
    echo
    echo "════════════════════════════════════════════════════════════════"
    echo "Environment Check Summary"
    echo "════════════════════════════════════════════════════════════════"
    echo -e "Passed:   ${GREEN}$CHECKS_PASSED${NC}"
    echo -e "Failed:   ${RED}$CHECKS_FAILED${NC}"
    echo -e "Warnings: ${YELLOW}$CHECKS_WARNING${NC}"
    echo
    
    if [ "$CHECKS_FAILED" -eq 0 ]; then
        echo -e "${GREEN}Environment is ready for development!${NC}"
        return 0
    else
        echo -e "${RED}Environment has issues that need to be resolved.${NC}"
        echo
        echo "To fix:"
        echo "  - Rust: Install from https://rustup.rs"
        echo "  - Build tools: Install development packages for your OS"
        echo "  - Cargo tools: cargo install cargo-deny cargo-shear"
        return 1
    fi
}

# Main execution
main() {
    echo "════════════════════════════════════════════════════════════════"
    echo "blz Environment Check"
    echo "════════════════════════════════════════════════════════════════"
    echo
    
    check_os
    check_rust
    check_build_tools
    check_cargo_tools
    check_disk_space
    check_memory
    check_network
    check_permissions
    
    print_summary
}

# Only run main if executed directly (not sourced)
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    main "$@"
fi