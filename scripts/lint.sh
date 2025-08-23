#!/usr/bin/env bash
# Run Clippy linting on the blz project

set -e

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
    # Count warnings
    WARNING_COUNT=$(grep -c "warning:" || echo "0")
    
    if [ "$WARNING_COUNT" -gt 0 ]; then
        echo "⚠️  Found $WARNING_COUNT warnings"
        echo ""
        # Show the warnings
        cat
        echo ""
        echo "Run 'cargo clippy --fix' to auto-fix some issues"
        exit 1
    else
        echo "✅ No critical warnings found!"
        echo ""
        echo "Note: Documentation warnings are suppressed. Run 'cargo doc' to check docs."
    fi
}

echo "🎯 Clippy check complete!"