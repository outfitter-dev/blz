#!/usr/bin/env bash
set -euo pipefail

# Compiler-in-the-Loop for AI Agents
# Usage: ./scripts/agent-check.sh [--fix] [--expand] [--verbose]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Parse arguments
APPLY_FIXES=false
SHOW_EXPANSIONS=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --fix) APPLY_FIXES=true; shift ;;
        --expand) SHOW_EXPANSIONS=true; shift ;;
        --verbose) VERBOSE=true; shift ;;
        --help) 
            echo "Usage: $0 [--fix] [--expand] [--verbose]"
            echo "  --fix      Apply automated fixes (cargo fix, clippy --fix)"
            echo "  --expand   Generate macro expansions for debugging"
            echo "  --verbose  Show full JSON diagnostic output"
            exit 0
            ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

cd "$ROOT_DIR"

echo "=== Compiler-in-the-Loop Check ==="

# Step 1: Collect machine-readable diagnostics
echo "Collecting diagnostics..."
cargo check --workspace --all-targets --message-format=json 2>/dev/null > /tmp/diagnostics.json || true

# Step 2: Parse and summarize errors
if command -v jq >/dev/null 2>&1; then
    echo ""
    echo "=== Error Summary ==="
    
    # Count errors by type
    error_count=$(jq -r 'select(.reason=="compiler-message" and .message.level=="error") | .message.message' /tmp/diagnostics.json | wc -l)
    warning_count=$(jq -r 'select(.reason=="compiler-message" and .message.level=="warning") | .message.message' /tmp/diagnostics.json | wc -l)
    
    echo "Errors: $error_count"
    echo "Warnings: $warning_count"
    
    if [[ $error_count -gt 0 ]]; then
        echo ""
        echo "=== Error Details ==="
        jq -r 'select(.reason=="compiler-message" and .message.level=="error") | 
               "\(.target.src_path):\(.message.spans[0].line_start): \(.message.message)"' \
               /tmp/diagnostics.json | head -10
    fi
else
    echo "jq not found; install with: apt-get install jq (or brew install jq)"
fi

# Step 3: Apply automated fixes if requested
if [[ "$APPLY_FIXES" == "true" ]]; then
    echo ""
    echo "=== Applying Automated Fixes ==="
    
    # Apply rustfix suggestions
    echo "Running cargo fix..."
    cargo fix --workspace --allow-dirty --allow-staged || true
    
    # Apply clippy fixes
    echo "Running clippy --fix..."
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged -- -D warnings || true
    
    echo "Fixes applied. Re-run check to see remaining issues."
fi

# Step 4: Generate macro expansions if needed
if [[ "$SHOW_EXPANSIONS" == "true" ]] && command -v jq >/dev/null 2>&1; then
    # Check if errors involve macros
    macro_errors=$(jq -r 'select(.reason=="compiler-message" and .message.level=="error" and (.message.message | contains("macro")))' /tmp/diagnostics.json | wc -l)
    
    if [[ $macro_errors -gt 0 ]]; then
        echo ""
        echo "=== Macro Expansions (for debugging) ==="
        echo "Detected macro-related errors. Generating expansions..."
        
        # Generate expansions for problematic modules
        if command -v cargo-expand >/dev/null 2>&1; then
            cargo expand --lib > /tmp/expanded.rs 2>/dev/null || {
                echo "Note: cargo expand failed. Install with: cargo install cargo-expand"
            }
            echo "Macro expansions saved to /tmp/expanded.rs"
        else
            echo "Note: cargo-expand not installed. Install with: cargo install cargo-expand"
        fi
    fi
fi

# Step 5: Verbose output if requested
if [[ "$VERBOSE" == "true" ]]; then
    echo ""
    echo "=== Full Diagnostic JSON ==="
    cat /tmp/diagnostics.json
fi

echo ""
echo "=== Check Complete ==="