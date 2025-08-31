#!/usr/bin/env bash
set -euo pipefail

echo "🔍 Running agent-friendly Rust checks..."

TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

# 1. Initial check with JSON diagnostics
echo "📋 Collecting diagnostics..."
cargo check --workspace --all-targets --message-format=json 2>/dev/null > "$TEMP_DIR/diagnostics.json" || true

# 2. Count errors and warnings
if command -v jq >/dev/null; then
    error_count=$(jq -sr '[ .[] | select(.reason=="compiler-message") | .message.level | select(.=="error") ] | length' "$TEMP_DIR/diagnostics.json" 2>/dev/null || echo "0")
    warning_count=$(jq -sr '[ .[] | select(.reason=="compiler-message") | .message.level | select(.=="warning") ] | length' "$TEMP_DIR/diagnostics.json" 2>/dev/null || echo "0")
    
    echo "Found $error_count errors and $warning_count warnings"
    
    # 3. Show first few errors for context
    if [[ "$error_count" -gt 0 ]]; then
        echo "📝 First errors:"
        jq -sr '[
            .[] | select(.reason=="compiler-message") | select(.message.level=="error")
            | {msg:.message.message, span:(.message.spans[0]? // {})}
            | "\((.span.file_name // "<unknown>")):\((.span.line_start // 0)): \(.msg)"
        ] | .[]' "$TEMP_DIR/diagnostics.json" 2>/dev/null | head -5 || true
    fi
    
    # 4. Attempt automated fixes
    if [[ "$error_count" -gt 0 || "$warning_count" -gt 0 ]]; then
        echo "🔧 Attempting automated fixes..."
        
        # Apply cargo fix
        if cargo fix --workspace --all-targets --allow-dirty --allow-staged 2>/dev/null; then
            echo "✅ cargo fix succeeded"
        else
            echo "❌ cargo fix failed or had no suggestions"
        fi
        
        # Apply clippy fixes
        if cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged 2>/dev/null; then
            echo "✅ clippy --fix succeeded"  
        else
            echo "❌ clippy --fix failed or had no suggestions"
        fi
        
        # Re-check after fixes
        echo "📋 Re-checking after fixes..."
        cargo check --workspace --all-targets --message-format=json 2>/dev/null > "$TEMP_DIR/post_fix_diagnostics.json" || true
        
        new_error_count=$(jq -sr '[ .[] | select(.reason=="compiler-message") | .message.level | select(.=="error") ] | length' "$TEMP_DIR/post_fix_diagnostics.json" 2>/dev/null || echo "0")
        new_warning_count=$(jq -sr '[ .[] | select(.reason=="compiler-message") | .message.level | select(.=="warning") ] | length' "$TEMP_DIR/post_fix_diagnostics.json" 2>/dev/null || echo "0")
        
        echo "After fixes: $new_error_count errors, $new_warning_count warnings"
        
        # Show remaining errors
        if [[ "$new_error_count" -gt 0 ]]; then
            echo "📝 Remaining errors:"
            jq -sr '[
                .[] | select(.reason=="compiler-message") | select(.message.level=="error")
                | {msg:.message.message, span:(.message.spans[0]? // {})}
                | "\((.span.file_name // "<unknown>")):\((.span.line_start // 0)): \(.msg)"
            ] | .[]' "$TEMP_DIR/post_fix_diagnostics.json" 2>/dev/null | head -5 || true
        fi
    fi
    
    # 5. If errors remain, try macro expansion for context
    if [[ "${new_error_count:-$error_count}" -gt 0 ]] && command -v cargo-expand >/dev/null; then
        echo "🔍 Checking for macro-related errors..."
        
        # Look for macro-related errors using latest diagnostics
        macro_file="$TEMP_DIR/${new_error_count:+post_fix_}diagnostics.json"
        macro_errors=$(jq -sr '[ .[] | select(.reason=="compiler-message" and .message.level=="error" and (.message.message | contains("macro") or contains("derive") or contains("procedural"))) ] | length' "$macro_file" 2>/dev/null || echo "0")
        
        if [[ "$macro_errors" -gt 0 ]]; then
            echo "📦 Macro errors detected, generating expansions..."
            if timeout 30s cargo expand 2>/dev/null > "$TEMP_DIR/expanded.rs"; then
                echo "✅ Macro expansion saved to $TEMP_DIR/expanded.rs ($(wc -l < "$TEMP_DIR/expanded.rs") lines)"
                echo "💡 Review expanded code to debug macro issues"
                echo "💡 Use: cargo expand <specific_item> for targeted expansion"
            else
                echo "❌ Macro expansion failed or timed out"
            fi
        fi
    fi
    
    # 6. Summary
    echo ""
    echo "📊 Summary:"
    echo "  Original: $error_count errors, $warning_count warnings"
    if [[ -n "${new_error_count:-}" ]]; then
        echo "  After fixes: $new_error_count errors, $new_warning_count warnings"
        
        if [[ "$new_error_count" -lt "$error_count" ]]; then
            echo "  ✅ Reduced errors by $((error_count - new_error_count))"
        fi
        
        if [[ "$new_warning_count" -lt "$warning_count" ]]; then
            echo "  ✅ Reduced warnings by $((warning_count - new_warning_count))"
        fi
    fi
    
else
    echo "⚠️  jq not found - install for better diagnostics parsing"
    echo "Running basic checks..."
    cargo check --workspace --all-targets
fi

echo "✅ Agent check complete"