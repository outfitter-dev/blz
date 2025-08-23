#!/usr/bin/env bash
# Run Clippy linting on the blz project

# Set script directory before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common configuration and utilities
source "${SCRIPT_DIR}/common.sh"

echo "🔍 Running Clippy linting checks..."
echo ""

# Run Clippy with all targets and features
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | {
    # Filter out known acceptable warnings
    grep -v "missing \`package" | 
    grep -v "multiple versions" |
    grep -v "lint group" |
    grep -v "missing documentation" || true
} | {
    # Store output
    OUTPUT=$(cat)
    # Count warnings - ensure we get a valid number
    if echo "$OUTPUT" | grep -q "warning:"; then
        WARNING_COUNT=$(echo "$OUTPUT" | grep -c "warning:")
    else
        WARNING_COUNT=0
    fi
    
    if [ $WARNING_COUNT -gt 0 ]; then
        echo "⚠️  Found $WARNING_COUNT warnings"
        echo ""
        # Show the warnings
        echo "$OUTPUT"
        echo ""
        echo "Run 'cargo clippy --fix' to auto-fix some issues"
        exit 1
    else
        echo "✅ No critical warnings found!"
        echo ""
        echo "Note: Documentation warnings are suppressed. Run 'cargo doc' to check docs."
        exit 0
    fi
}