# Compiler-in-the-Loop Development for Agents

## Purpose

This guide helps AI agents integrate with Rust compiler tools for faster, more accurate development cycles using machine-readable diagnostics and automated fixes.

## Core Workflow

### 1. JSON Diagnostics Collection

Use `cargo check --message-format=json` to get structured compiler output:

```bash
# Get all compiler messages in JSON format
cargo check --message-format=json 2>/dev/null > diagnostics.json

# Filter to just errors
cargo check --message-format=json 2>/dev/null | jq '.message | select(.level=="error")'

# Filter to warnings
cargo check --message-format=json 2>/dev/null | jq '.message | select(.level=="warning")'

# Get specific error types
cargo check --message-format=json 2>/dev/null | jq '.message | select(.code.code=="E0308")'
```

### 2. Automated Fixes

Apply automated fixes where the compiler can help:

```bash
# Fix trivial issues (imports, unused vars, etc.)
cargo fix --allow-dirty --allow-staged

# Fix clippy suggestions
cargo clippy --fix --allow-dirty --allow-staged

# Allow experimental fixes
cargo fix --allow-dirty --allow-staged --edition-idioms
```

### 3. Macro Expansion for Debugging

When compiler errors involve macros, use `cargo expand`:

```bash
# Install cargo-expand
cargo install cargo-expand

# Expand specific item
cargo expand function_name

# Expand entire module
cargo expand module::submodule

# Expand with features
cargo expand --features "feature1,feature2"

# Expand and save to file for analysis
cargo expand > expanded.rs
```

## Agent Integration Script

Create `scripts/agent-check.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "🔍 Running agent-friendly Rust checks..."

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# 1. Initial check with JSON diagnostics
echo "📋 Collecting diagnostics..."
cargo check --message-format=json 2>/dev/null > "$TEMP_DIR/diagnostics.json" || true

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
        if cargo fix --allow-dirty --allow-staged 2>/dev/null; then
            echo "✅ cargo fix succeeded"
        else
            echo "❌ cargo fix failed or had no suggestions"
        fi
        
        # Apply clippy fixes
        if cargo clippy --fix --allow-dirty --allow-staged 2>/dev/null; then
            echo "✅ clippy --fix succeeded"  
        else
            echo "❌ clippy --fix failed or had no suggestions"
        fi
        
        # Re-check after fixes
        echo "📋 Re-checking after fixes..."
        cargo check --message-format=json 2>/dev/null > "$TEMP_DIR/post_fix_diagnostics.json" || true
        
        new_error_count=$(jq -sr '[ .[] | select(.reason=="compiler-message") | .message.level | select(.=="error") ] | length' "$TEMP_DIR/post_fix_diagnostics.json" 2>/dev/null || echo "0")
        new_warning_count=$(jq -sr '[ .[] | select(.reason=="compiler-message") | .message.level | select(.=="warning") ] | length' "$TEMP_DIR/post_fix_diagnostics.json" 2>/dev/null || echo "0")
        
        echo "After fixes: $new_error_count errors, $new_warning_count warnings"
    fi
    
    # 5. If errors remain, try macro expansion for context
    if [[ "${new_error_count:-$error_count}" -gt 0 ]] && command -v cargo-expand >/dev/null; then
        echo "🔍 Checking for macro-related errors..."
        
        # Look for macro-related errors in latest diagnostics
        macro_errors=$(jq -sr '[ .[] | select(.reason=="compiler-message") | select(.message.level=="error" and (.message.message | contains("macro") or contains("derive"))) ] | length' "$TEMP_DIR/${new_error_count:+post_fix_}diagnostics.json" 2>/dev/null || echo "0")
        
        if [[ "$macro_errors" -gt 0 ]]; then
            echo "📦 Macro errors detected, generating expansions..."
            if cargo expand 2>/dev/null > "$TEMP_DIR/expanded.rs"; then
                echo "✅ Macro expansion saved to $TEMP_DIR/expanded.rs"
                echo "💡 Review expanded code to debug macro issues"
            else
                echo "❌ Macro expansion failed"
            fi
        fi
    fi
else
    echo "⚠️  jq not found - install for better diagnostics parsing"
    echo "Running basic checks..."
    cargo check
fi

echo "✅ Agent check complete"
```

## Parsing JSON Diagnostics

### Message Structure

Rust compiler JSON messages have this structure:

```json
{
  "message": {
    "message": "mismatched types",
    "code": {
      "code": "E0308",
      "explanation": "..."
    },
    "level": "error",
    "spans": [
      {
        "file_name": "src/lib.rs",
        "byte_start": 100,
        "byte_end": 110,
        "line_start": 5,
        "line_end": 5,
        "column_start": 10,
        "column_end": 20,
        "text": [
          {
            "text": "    let x: u32 = \"hello\";",
            "highlight_start": 19,
            "highlight_end": 26
          }
        ]
      }
    ],
    "children": [
      {
        "message": "expected `u32`, found `&str`",
        "level": "note"
      }
    ]
  },
  "target": {
    "kind": ["lib"],
    "crate_types": ["lib"],
    "name": "myproject",
    "src_path": "src/lib.rs"
  }
}
```

### Useful jq Filters

```bash
# Get error locations
jq -r '.message | select(.level=="error") | "\(.spans[0].file_name):\(.spans[0].line_start):\(.spans[0].column_start)"'

# Get error codes
jq -r '.message | select(.level=="error") | .code.code'

# Get suggestions/help
jq -r '.message | select(.level=="error") | .children[] | select(.level=="help") | .message'

# Group by file
jq -r '.message | select(.level=="error") | group_by(.spans[0].file_name)'
```

## Common Error Patterns and Fixes

### Pattern 1: Lifetime Errors

```bash
# Detect lifetime errors
jq -r '.message | select(.level=="error" and (.code.code=="E0106" or .code.code=="E0621"))'

# Common fixes:
# - Add explicit lifetime parameters
# - Use owned types instead of references  
# - Restructure code to avoid complex lifetimes
```

### Pattern 2: Borrow Checker Errors

```bash
# Detect borrowing issues
jq -r '.message | select(.level=="error" and (.code.code | startswith("E05")))'

# Common fixes:
# - Clone data instead of borrowing
# - Use Arc/Rc for shared ownership
# - Restructure to avoid conflicting borrows
```

### Pattern 3: Type Mismatches

```bash
# Detect type errors
jq -r '.message | select(.level=="error" and .code.code=="E0308")'

# Common fixes:
# - Add type annotations
# - Use .into() or .try_into() conversions
# - Check generic type parameters
```

### Pattern 4: Missing Trait Implementations

```bash
# Detect trait errors
jq -r '.message | select(.level=="error" and .code.code=="E0277")'

# Common fixes:
# - Add derive macros: #[derive(Debug, Clone)]
# - Implement traits manually
# - Add trait bounds to generics
```

## Macro Debugging Strategies

### When to Use cargo expand

1. **Derive macro errors**: When `#[derive(...)]` fails
2. **Procedural macro issues**: Complex custom macros
3. **Compiler pointing to generated code**: Error spans in generated code
4. **Unfamiliar macro behavior**: Understanding what code is generated

### Expansion Examples

```bash
# Debug a derive macro
cargo expand --bin mybin MyStruct

# See what serde generates
cargo expand serde_example

# Debug async code (tokio macros)
cargo expand async_function

# Compare before/after macro changes
cargo expand > before.rs
# Make macro changes
cargo expand > after.rs
diff before.rs after.rs
```

### Reading Expanded Code

- Look for generated `impl` blocks
- Check trait implementations
- Verify generic bounds
- Understand lifetime parameters

## Integration with IDEs

### rust-analyzer Integration

Configure rust-analyzer to use JSON diagnostics:

```json
{
  "rust-analyzer.check.command": "check",
  "rust-analyzer.check.extraArgs": ["--message-format=json"]
}
```

### Custom LSP Integration

For custom tools that process Rust code:

```rust
// Example: Parse JSON diagnostics in Rust
use serde_json::Value;

fn parse_diagnostics(json_output: &str) -> Vec<Diagnostic> {
    json_output
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|value| {
            if let Some(message) = value.get("message") {
                Some(parse_message(message))
            } else {
                None
            }
        })
        .collect()
}
```

## Performance Considerations

### Caching Checks

```bash
# Use cargo's built-in caching
export CARGO_TARGET_DIR=target-shared

# Skip dependencies if unchanged
cargo check --workspace --lib

# Check specific packages
cargo check -p blz-core -p blz-cli
```

### Incremental Compilation

```bash
# Enable incremental compilation
export CARGO_INCREMENTAL=1

# Use faster linker (on Linux)
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
```

## Agent Best Practices

1. **Always parse JSON output** - Don't rely on text parsing
2. **Apply fixes incrementally** - Run checks between fix attempts  
3. **Use macro expansion sparingly** - Only when errors are unclear
4. **Cache results** - Don't re-run checks on unchanged code
5. **Understand error codes** - Learn common E-codes for faster fixes
6. **Test fixes** - Always verify fixes don't break existing functionality

## Troubleshooting

### Common Issues

**jq not available:**

```bash
# Install jq
# Ubuntu/Debian: apt install jq
# macOS: brew install jq
# Windows: choco install jq
```

**cargo-expand not available:**

```bash
cargo install cargo-expand
```

**Large macro expansions:**

```bash
# Expand specific items only
cargo expand specific_function
cargo expand 'MyStruct::*'

# Limit output size
cargo expand | head -1000
```

**Slow check times:**

```bash
# Use faster check mode
cargo check --profile dev

# Skip expensive checks
SKIP_EXPENSIVE_TESTS=1 cargo check
```

This compiler-loop approach transforms Rust development from a reactive debugging process into a proactive, tool-assisted workflow that agents can leverage effectively.
