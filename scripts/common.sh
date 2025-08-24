#!/usr/bin/env bash
#
# common.sh - Shared configuration and utilities for all blz scripts
#
# This file should be sourced by other scripts:
#   source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
#
# It provides:
# - Color definitions for consistent output
# - Project configuration variables
# - Common helper functions
#

# Strict mode (inherited by sourcing scripts)
set -euo pipefail

# Color definitions for terminal output
export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export CYAN='\033[0;36m'
export MAGENTA='\033[0;35m'
export NC='\033[0m' # No Color

# Project configuration
export BINARY_NAME="blz"
export REQUIRED_RUST_VERSION="1.75.0"

# Derive paths from script location
# Note: SCRIPT_DIR should be set by the calling script before sourcing
if [ -z "${SCRIPT_DIR:-}" ]; then
    echo "Error: SCRIPT_DIR must be set before sourcing common.sh" >&2
    exit 1
fi

# Determine project root based on script location
if [ "$(basename "$SCRIPT_DIR")" = "scripts" ]; then
    export PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
else
    # Script is in project root
    export PROJECT_ROOT="$SCRIPT_DIR"
fi

# Common paths
export TARGET_DIR="${PROJECT_ROOT}/target"
export RELEASE_BINARY="${TARGET_DIR}/release/${BINARY_NAME}"
export DEBUG_BINARY="${TARGET_DIR}/debug/${BINARY_NAME}"
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"

# Tool requirements
export MIN_DISK_SPACE_MB=500
export MIN_MEMORY_MB=1024
export REQUIRED_NODE_VERSION="18.0.0"  # For optional dev tools

# Logging functions with consistent format
log_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

log_step() {
    local step="${1:-}"
    local total="${2:-}"
    if [ -n "$total" ]; then
        echo -e "${BLUE}[${step}/${total}]${NC} ${3:-}"
    else
        echo -e "${BLUE}▶${NC} $step"
    fi
}

log_section() {
    echo
    echo -e "${CYAN}▶ $1${NC}"
    echo "────────────────────────────────────────────"
}

# Check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Compare version strings (returns 0 if $1 >= $2)
version_ge() {
    [ "$(printf '%s\n' "$2" "$1" | sort -V | head -n1)" = "$2" ]
}

# Check if running on macOS
is_macos() {
    [[ "$OSTYPE" == "darwin"* ]]
}

# Check if running on Linux
is_linux() {
    [[ "$OSTYPE" == "linux-gnu"* ]]
}

# Check if running on Windows (Git Bash, WSL, etc.)
is_windows() {
    [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "win32" ]]
}

# Ensure we're in the project root
ensure_project_root() {
    if [ ! -f "${PROJECT_ROOT}/Cargo.toml" ]; then
        log_error "Not in project root. Expected to find Cargo.toml at: ${PROJECT_ROOT}"
        exit 1
    fi
    cd "$PROJECT_ROOT"
}

# Check if Rust toolchain is available
check_rust_available() {
    if ! command_exists rustc; then
        log_error "Rust is not installed. Install from: https://rustup.rs"
        return 1
    fi
    
    local rust_version=$(rustc --version | cut -d' ' -f2)
    if ! version_ge "$rust_version" "$REQUIRED_RUST_VERSION"; then
        log_warning "Rust $rust_version is installed but $REQUIRED_RUST_VERSION is required"
        return 1
    fi
    
    return 0
}

# Run cargo command with proper error handling
run_cargo() {
    local cmd="$1"
    shift
    
    if ! check_rust_available; then
        return 1
    fi
    
    log_info "Running: cargo $cmd $*"
    if cargo "$cmd" "$@"; then
        return 0
    else
        log_error "cargo $cmd failed"
        return 1
    fi
}

# Clean temporary files
clean_temp_files() {
    log_info "Cleaning temporary files..."
    find "${PROJECT_ROOT}" -name "*.orig" -type f -delete 2>/dev/null || true
    find "${PROJECT_ROOT}" -name "*.rej" -type f -delete 2>/dev/null || true
    find "${PROJECT_ROOT}" -name "*~" -type f -delete 2>/dev/null || true
    find "${PROJECT_ROOT}" -name ".DS_Store" -type f -delete 2>/dev/null || true
}

# Print a separator line
print_separator() {
    echo "════════════════════════════════════════════════════════════════"
}

# Export a flag to indicate common.sh has been sourced
export BLZ_COMMON_SOURCED=1