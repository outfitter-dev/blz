#!/usr/bin/env bash

# Setup script for git hooks in Rust project
# This installs lefthook and commitlint-rs for a Rust-first workflow

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "🚀 Setting up git hooks for Rust project..."

# Check if lefthook is installed
if ! command -v lefthook &> /dev/null; then
    echo -e "${YELLOW}📦 Lefthook not found. Installing...${NC}"
    
    # Prefer Homebrew on macOS
    if [[ "$OSTYPE" == "darwin"* ]] && command -v brew &> /dev/null; then
        echo "Installing lefthook via Homebrew..."
        brew install lefthook
    # Use cargo as fallback
    elif command -v cargo &> /dev/null; then
        echo "Installing lefthook via Cargo..."
        cargo install lefthook
    # Use install script as last resort
    else
        echo "Installing lefthook via install script..."
        curl -sSfL https://raw.githubusercontent.com/evilmartians/lefthook/master/install.sh | sh
    fi
else
    echo -e "${GREEN}✓ Lefthook already installed${NC}"
fi

# Check if commitlint-rs is installed
if ! command -v commitlint &> /dev/null; then
    echo -e "${YELLOW}📦 commitlint-rs not found. Installing...${NC}"
    
    if command -v cargo &> /dev/null; then
        echo "Installing commitlint-rs via Cargo..."
        cargo install commitlint-rs
    else
        echo -e "${RED}❌ Cargo not found. Please install Rust first.${NC}"
        echo "Visit: https://www.rust-lang.org/tools/install"
        exit 1
    fi
else
    echo -e "${GREEN}✓ commitlint-rs already installed${NC}"
fi

# Install git hooks via lefthook
echo -e "${BLUE}🔗 Installing git hooks...${NC}"
lefthook install

# Verify installation
echo ""
echo -e "${GREEN}✅ Git hooks setup complete!${NC}"
echo ""
echo "Installed tools:"
echo -e "  ${GREEN}•${NC} lefthook $(lefthook version 2>/dev/null || echo "(version unknown)")"
echo -e "  ${GREEN}•${NC} commitlint-rs $(commitlint --version 2>/dev/null || echo "(version unknown)")"
echo ""
echo "Git hooks configured:"
echo -e "  ${GREEN}•${NC} pre-commit: Rust formatting and linting"
echo -e "  ${GREEN}•${NC} commit-msg: Conventional commit validation"
echo -e "  ${GREEN}•${NC} pre-push: Test and build verification"
echo ""
echo "Available commands:"
echo -e "  ${BLUE}lefthook run ci${NC}   - Run full CI checks locally"
echo -e "  ${BLUE}lefthook run fix${NC}  - Auto-fix formatting and linting issues"
echo ""
echo "To skip hooks temporarily, use:"
echo -e "  ${YELLOW}git commit --no-verify${NC}"
echo -e "  ${YELLOW}git push --no-verify${NC}"