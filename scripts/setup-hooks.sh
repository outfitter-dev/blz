#!/usr/bin/env bash

# Setup script for git hooks in Rust project
# This installs lefthook and commitlint-rs for a Rust-first workflow

# Set script directory before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common configuration and utilities
source "${SCRIPT_DIR}/common.sh"

echo "🚀 Setting up git hooks for Rust project..."

# Check if lefthook is installed
if ! command_exists lefthook; then
    log_warning "📦 Lefthook not found. Installing..."
    
    # Prefer Homebrew on macOS
    if is_macos && command_exists brew; then
        echo "Installing lefthook via Homebrew..."
        brew install lefthook
    # Use cargo as fallback
    elif command_exists cargo; then
        echo "Installing lefthook via Cargo..."
        cargo install lefthook
    # Use install script as last resort
    else
        echo "Installing lefthook via install script..."
        curl -sSfL https://raw.githubusercontent.com/evilmartians/lefthook/master/install.sh | sh
    fi
else
    log_success "Lefthook already installed"
fi

# Check if commitlint-rs is installed
if ! command_exists commitlint; then
    log_warning "📦 commitlint-rs not found. Installing..."
    
    if command_exists cargo; then
        echo "Installing commitlint-rs via Cargo..."
        cargo install commitlint-rs
    else
        log_error "Cargo not found. Please install Rust first."
        echo "Visit: https://www.rust-lang.org/tools/install"
        exit 1
    fi
else
    log_success "commitlint-rs already installed"
fi

# Install git hooks via lefthook
log_info "🔗 Installing git hooks..."
lefthook install

# Verify installation
echo ""
log_success "Git hooks setup complete!"
echo ""
echo "Installed tools:"
echo -e "  ${GREEN}•${NC}" lefthook $(lefthook version 2>/dev/null || echo "(version unknown)")"
echo -e "  ${GREEN}•${NC}" commitlint-rs $(commitlint --version 2>/dev/null || echo "(version unknown)")"
echo ""
echo "Git hooks configured:"
echo -e "  ${GREEN}•${NC}" pre-commit: Rust formatting and linting"
echo -e "  ${GREEN}•${NC}" commit-msg: Conventional commit validation"
echo -e "  ${GREEN}•${NC}" pre-push: Test and build verification"
echo ""
echo "Available commands:"
echo -e "  ${BLUE}"lefthook run ci${NC}   - Run full CI checks locally"
echo -e "  ${BLUE}"lefthook run fix${NC}  - Auto-fix formatting and linting issues"
echo ""
echo "To skip hooks temporarily, use:"
echo -e "  ${YELLOW}"git commit --no-verify${NC}"
echo -e "  ${YELLOW}"git push --no-verify${NC}"