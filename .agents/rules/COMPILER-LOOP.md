# Compiler-in-the-Loop for AI Agents

## Overview

This guide provides AI agents with tools and patterns for integrating Rust compiler feedback into their development workflow. The compiler-in-the-loop approach helps agents understand errors, apply fixes, and debug complex compilation issues.

## Core Workflow Script

The `scripts/agent-check.sh` script provides JSON diagnostics, automated fixes, and debugging support:

```bash
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
    local error_count
    error_count=$(jq -r 'select(.reason=="compiler-message" and .message.level=="error") | .message.message' /tmp/diagnostics.json | wc -l)
    
    local warning_count
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
    local macro_errors
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
```

## JSON Diagnostic Parsing

### Understanding Compiler Messages

The Rust compiler outputs structured JSON when using `--message-format=json`. Here are the key patterns agents should know:

```bash
# Get all errors
jq -r 'select(.reason=="compiler-message" and .message.level=="error")' diagnostics.json

# Get all warnings  
jq -r 'select(.reason=="compiler-message" and .message.level=="warning")' diagnostics.json

# Extract error messages with locations
jq -r 'select(.reason=="compiler-message" and .message.level=="error") | 
       "\(.target.src_path):\(.message.spans[0].line_start): \(.message.message)"' diagnostics.json

# Find macro-related errors
jq -r 'select(.reason=="compiler-message" and .message.level=="error" and 
       (.message.message | contains("macro")))' diagnostics.json
```

### Common Error Patterns

**Borrow Checker Issues**:
```json
{
  "reason": "compiler-message",
  "message": {
    "level": "error",
    "message": "cannot borrow `x` as mutable because it is also borrowed as immutable",
    "spans": [...],
    "children": [...]
  }
}
```

**Trait Bound Issues**:
```json
{
  "message": {
    "message": "the trait bound `T: Send` is not satisfied",
    "code": {
      "code": "E0277",
      "explanation": "..."
    }
  }
}
```

**Async Issues**:
```json
{
  "message": {
    "message": "future cannot be sent between threads safely",
    "code": {
      "code": "E0277"
    }
  }
}
```

## Automated Fix Strategies

### 1. Rustfix Integration

```bash
# Apply all rustfix suggestions automatically
cargo fix --workspace --allow-dirty --allow-staged

# Apply fixes for specific errors only
cargo fix --workspace --allow-dirty --allow-staged --broken-code
```

### 2. Clippy Fixes

```bash
# Apply safe clippy suggestions
cargo clippy --workspace --fix --allow-dirty --allow-staged

# Apply all clippy suggestions (potentially breaking)
cargo clippy --workspace --fix --allow-dirty --allow-staged --allow-no-vcs
```

### 3. Common Fix Patterns

**Adding Missing Traits**:
```rust
// Error: "the trait bound `T: Send` is not satisfied"
// Fix: Add the bound
fn spawn_task<T: Send + 'static>(data: T) -> JoinHandle<()> {
    tokio::spawn(async move {
        process(data).await;
    })
}
```

**Fixing Lifetime Issues**:
```rust
// Error: "borrowed value does not live long enough"
// Fix: Use owned data or extend lifetime
fn bad_function(data: &str) -> String {
    data.to_string() // Fix: convert to owned
}
```

**Resolving Import Issues**:
```rust
// Error: "use of undeclared type `HashMap`"
// Fix: Add import
use std::collections::HashMap;
```

## Macro Expansion Debugging

### When to Expand Macros

Expand macros when errors mention:
- "macro"
- "procedural macro"
- "derive"
- Error locations point to macro invocations

### Using cargo-expand

```bash
# Expand all macros in the current crate
cargo expand

# Expand specific module
cargo expand --lib module_name

# Expand with specific features
cargo expand --features feature_name

# Expand for specific target
cargo expand --bin binary_name
```

### Interpreting Expanded Code

```rust
// Original code with derive macro
#[derive(Debug, Clone)]
struct User {
    name: String,
}

// Expanded code (simplified)
impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Generated Debug implementation
    }
}

impl Clone for User {
    fn clone(&self) -> Self {
        // Generated Clone implementation
    }
}
```

## Error Classification & Solutions

### 1. Borrow Checker Errors

**Pattern Recognition**:
```bash
# Detect borrow checker errors
jq -r 'select(.message.message | test("cannot borrow.*as mutable.*borrowed as immutable"))' diagnostics.json
```

**Common Solutions**:
- Clone the data: `let owned = data.clone();`
- Use `Arc<Mutex<T>>` for shared mutable state
- Restructure code to avoid overlapping borrows
- Use interior mutability (`RefCell`, `Cell`)

### 2. Trait Bound Errors

**Pattern Recognition**:
```bash
# Detect missing trait bounds
jq -r 'select(.message.message | test("trait bound.*is not satisfied"))' diagnostics.json
```

**Common Solutions**:
- Add missing trait bounds: `T: Send + Sync`
- Use trait objects: `Box<dyn Trait>`
- Add `where` clauses for complex bounds

### 3. Async Errors

**Pattern Recognition**:
```bash
# Detect async errors
jq -r 'select(.message.message | test("future cannot be sent|borrowed value.*await"))' diagnostics.json
```

**Common Solutions**:
- Use owned data in async blocks
- Clone `Arc` before moving into async
- Avoid holding references across `.await` points

### 4. Macro Errors

**Pattern Recognition**:
```bash
# Detect macro errors
jq -r 'select(.message.message | test("macro|procedural macro"))' diagnostics.json
```

**Debugging Steps**:
1. Use `cargo expand` to see generated code
2. Check macro syntax and arguments
3. Verify required dependencies are available
4. Look at macro documentation for correct usage

## Integration with Development Workflow

### 1. Pre-commit Workflow

```bash
#!/bin/bash
# Pre-commit script using compiler loop

./scripts/agent-check.sh --fix

# Check if fixes resolved all issues
if ./scripts/agent-check.sh | grep -q "Errors: 0"; then
    echo "✅ All issues resolved"
    exit 0
else
    echo "❌ Remaining issues need manual attention"
    ./scripts/agent-check.sh --verbose
    exit 1
fi
```

### 2. Continuous Integration

```yaml
# .github/workflows/rust.yml
- name: Run compiler loop check
  run: |
    ./scripts/agent-check.sh --verbose
    # Fail if there are any errors
    if ./scripts/agent-check.sh | grep -q "Errors: [1-9]"; then
      exit 1
    fi
```

### 3. Development Iteration

```bash
# Development loop
while true; do
    echo "Making changes..."
    # Edit code
    
    echo "Checking compilation..."
    ./scripts/agent-check.sh --fix
    
    echo "Running tests..."
    cargo test
    
    if [[ $? -eq 0 ]]; then
        echo "✅ Iteration complete"
        break
    else
        echo "🔄 Issues found, continuing..."
    fi
done
```

## Advanced Debugging Techniques

### 1. Compiler Flag Analysis

```bash
# Show all available lints
rustc -W help

# Explain specific error codes
rustc --explain E0277

# Show expanded code with specific flags
cargo rustc -- -Z unpretty=expanded
```

### 2. Dependency Analysis

```bash
# Check for conflicting dependencies
cargo tree --duplicates

# Analyze feature dependencies
cargo tree --features serde

# Show why a dependency is included
cargo tree --invert dep_name
```

### 3. Target-Specific Issues

```bash
# Check compilation for specific targets
cargo check --target x86_64-unknown-linux-gnu
cargo check --target wasm32-unknown-unknown

# Show target-specific diagnostic differences
cargo check --target x86_64-pc-windows-msvc --message-format=json > windows.json
cargo check --target x86_64-unknown-linux-gnu --message-format=json > linux.json
```

## Quick Reference Commands

| Task | Command |
|------|---------|
| Get all errors | `jq 'select(.reason=="compiler-message" and .message.level=="error")' diagnostics.json` |
| Apply automatic fixes | `cargo fix --allow-dirty --allow-staged` |
| Show macro expansions | `cargo expand` |
| Explain error code | `rustc --explain E0277` |
| Find duplicate deps | `cargo tree --duplicates` |
| Check specific target | `cargo check --target target-name` |
| Get clippy fixes | `cargo clippy --fix --allow-dirty` |

## When to Use Manual vs Automated Fixes

### Use Automated Fixes For:
- Import statements
- Unused variable warnings
- Simple clippy suggestions
- Formatting issues
- Obvious syntax corrections

### Use Manual Analysis For:
- Complex borrow checker errors
- Architecture/design issues
- Performance optimizations
- Trait bound complications
- Macro debugging

## Error Recovery Strategies

1. **Start Simple**: Fix compilation errors before addressing warnings
2. **One Error at a Time**: Focus on the first error, as later errors may be cascading
3. **Use Expansion**: When stuck on macro errors, always check expanded code
4. **Check Dependencies**: Version conflicts often manifest as mysterious errors
5. **Read Error Messages**: Rust errors are usually very helpful - read them carefully

The compiler is your best debugging tool. Learn to work with it, not against it!