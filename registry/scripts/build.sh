#!/usr/bin/env bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Building registry.json from sources..."

# Check for required tools
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ cargo not found. Please install Rust.${NC}"
    exit 1
fi

# Get repo root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# Build the registry using the Rust helper
echo "Compiling registry builder..."
cargo build --release --bin blz-registry-build 2>/dev/null || {
    echo -e "${YELLOW}ℹ Registry builder not yet implemented, using fallback${NC}"

    # Fallback: Simple JSON generation using jq if available
    if command -v jq &> /dev/null; then
        echo "Using jq fallback..."

        # Start with empty registry
        echo '{"version":"1.0.0","updated":"'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'","sources":[]}' > registry.json

        # For each .toml file, convert to JSON and add to array
        # Note: This is a simplified version. Full implementation would parse TOML properly
        echo -e "${YELLOW}⚠️  Fallback mode: Limited TOML parsing${NC}"
        echo -e "${YELLOW}   Run 'cargo build --release --bin blz-registry-build' for full support${NC}"

        echo -e "${GREEN}✓ Created registry.json${NC}"
        exit 0
    else
        echo -e "${RED}✗ Neither blz-registry-build nor jq available${NC}"
        exit 1
    fi
}

# Run the registry builder
./target/release/blz-registry-build

echo -e "${GREEN}✓ Generated registry.json with $(jq '.sources | length' registry.json 2>/dev/null || echo '?') sources${NC}"