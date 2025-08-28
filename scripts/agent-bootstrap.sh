#!/usr/bin/env bash
#
# agent-bootstrap.sh - Bootstrap script optimized for AI coding agents
# Provides context, handles common failure modes, and guides agents through setup

set -euo pipefail

# This script is designed for AI agents like Devin.ai, Factory.ai, Codex, etc.
# It provides clear context, handles errors gracefully, and maintains state.

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Project information
PROJECT_NAME="blz"
PROJECT_DESC="Local-first search cache for llms.txt documentation"
REPO_URL="https://github.com/outfitter-dev/blz"

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
AGENT_DIR="$PROJECT_ROOT/.agents"
STATE_FILE="$PROJECT_ROOT/.agent-state.json"
NOTES_FILE="$PROJECT_ROOT/notes.txt"

# Create agent context file
create_agent_context() {
    cat > "$PROJECT_ROOT/AGENT_CONTEXT.md" << 'EOF'
# Agent Context for blz

## Project Overview
- **Name**: blz (pronounced "blaze")
- **Purpose**: Local-first search cache for llms.txt documentation
- **Language**: Rust
- **Build System**: Cargo
- **Key Dependencies**: Tantivy (search engine), Tokio (async runtime)

## Quick Start Commands
```bash
# Setup environment
./setup.sh

# Common development tasks
./scripts/dev.sh build     # Build the project
./scripts/dev.sh test      # Run tests
./scripts/dev.sh check     # Run quality checks
./scripts/dev.sh fix       # Auto-fix issues
./scripts/dev.sh ci        # Run full CI locally
```

## Project Structure
```
cache/                      # Root directory (will be renamed to blz/)
├── crates/
│   ├── blz-core/          # Core search functionality
│   ├── blz-cli/           # Command-line interface
│   └── blz-mcp/           # MCP server (in development)
├── scripts/               # Development scripts
├── .agents/                # Agent-specific configuration
│   └── rules/            # Development guidelines
└── target/               # Build artifacts (git-ignored)
```

## Key Files for Agents
1. **Cargo.toml** - Workspace configuration
2. **.agents/rules/** - Development standards and patterns
3. **CLAUDE.md** - AI agent instructions
4. **AGENTS.md** - Source of truth for agent behavior
5. **notes.txt** - Your persistent notes (created by this script)

## Common Tasks

### Building the Project
```bash
cargo build --release
# Output: target/release/blz
```

### Running Tests
```bash
cargo test --workspace
```

### Code Quality
```bash
cargo fmt           # Format code
cargo clippy        # Lint code
cargo deny check    # Security audit
```

### Making Changes
1. Create feature branch
2. Make changes
3. Run `./scripts/dev.sh ci` to verify
4. Commit with conventional message: `feat:`, `fix:`, `docs:`, etc.

## Error Handling

### Common Issues and Solutions

**Build Fails**
- Check Rust version: `rustc --version` (need 1.75+)
- Clean and rebuild: `cargo clean && cargo build`

**Test Fails**
- Run specific test: `cargo test test_name`
- See test output: `cargo test -- --nocapture`

**Clippy Warnings**
- Auto-fix: `cargo clippy --fix`
- Manual review: `cargo clippy`

## Performance Targets
- Search latency: <10ms
- Index size: <10% overhead
- Memory usage: <100MB baseline

## Security Requirements
- No `unsafe` code without review
- All inputs validated
- Dependencies audited with `cargo deny`

## Testing Requirements
- 80% code coverage minimum
- All public APIs documented
- Tests must be deterministic

## Your Notes
Keep track of your work in `notes.txt`. This file persists across sessions.

## Getting Help
- Development guide: `.agents/rules/DEVELOPMENT.md`
- Architecture: `.agents/rules/ARCHITECTURE.md`
- Performance: `.agents/rules/PERFORMANCE.md`
- Security: `.agents/rules/SECURITY.md`
EOF

    echo -e "${GREEN}✓${NC} Created AGENT_CONTEXT.md"
}

# Initialize agent state
init_agent_state() {
    if [ ! -f "$STATE_FILE" ]; then
        cat > "$STATE_FILE" << EOF
{
  "initialized": true,
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "rust_version": "$(rustc --version 2>/dev/null || echo "not installed")",
  "last_build": null,
  "last_test": null,
  "tasks_completed": [],
  "current_task": null
}
EOF
        echo -e "${GREEN}✓${NC} Initialized agent state"
    else
        echo -e "${BLUE}ℹ${NC} Agent state already exists"
    fi
}

# Create agent notes file
create_notes_file() {
    if [ ! -f "$NOTES_FILE" ]; then
        cat > "$NOTES_FILE" << EOF
# Agent Notes for $PROJECT_NAME
# This file is for you to keep track of your work

## Session started: $(date)

### Quick Reference
- Build: cargo build --release
- Test: cargo test
- Run: ./target/release/blz
- Check: ./scripts/dev.sh check

### Current Focus
[Your current task goes here]

### Progress Log
[Track your progress here]

### Issues Encountered
[Document any problems and solutions]

### Next Steps
[What needs to be done next]

---
EOF
        echo -e "${GREEN}✓${NC} Created notes.txt for your use"
    else
        echo -e "${BLUE}ℹ${NC} Notes file already exists"
        echo "    Appending session marker..."
        echo -e "\n## New session: $(date)\n" >> "$NOTES_FILE"
    fi
}

# Create knowledge base entries
create_knowledge_entries() {
    local kb_dir="$PROJECT_ROOT/.agents/knowledge"
    mkdir -p "$kb_dir"
    
    # Create common patterns file
    cat > "$kb_dir/common-patterns.md" << 'EOF'
# Common Patterns in blz

## Result Pattern for Error Handling
```rust
use anyhow::Result;

pub fn operation() -> Result<Value> {
    // Returns Ok(value) or Err(error)
}
```

## Workspace Structure
- Each crate in `crates/` is independent
- Shared dependencies in workspace root Cargo.toml
- Project references for incremental builds

## Testing Patterns
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_function() {
        // Test implementation
    }
}
```

## Common Commands
- `cargo build --release` - Production build
- `cargo test --workspace` - All tests
- `cargo clippy -- -D warnings` - Strict linting
- `cargo fmt` - Auto-format code
EOF
    
    echo -e "${GREEN}✓${NC} Created knowledge base entries"
}

# Setup development environment
setup_environment() {
    echo -e "${BLUE}Setting up development environment...${NC}"
    
    # Check if Rust is installed
    if ! command -v rustc >/dev/null 2>&1; then
        echo -e "${YELLOW}⚠${NC} Rust not installed. Installing..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    # Install required components
    rustup component add clippy rustfmt rust-src 2>/dev/null || true
    
    # Install required tools
    local tools=("cargo-deny" "cargo-shear")
    for tool in "${tools[@]}"; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            echo "    Installing $tool..."
            cargo install "$tool" || true
        fi
    done
    
    echo -e "${GREEN}✓${NC} Environment ready"
}

# Verify project can build
verify_build() {
    echo -e "${BLUE}Verifying project builds...${NC}"
    
    if cargo build --release; then
        echo -e "${GREEN}✓${NC} Project builds successfully"
        
        # Update state
        if [ -f "$STATE_FILE" ]; then
            local temp_file=$(mktemp)
            jq '.last_build = now | .last_build_success = true' "$STATE_FILE" > "$temp_file"
            mv "$temp_file" "$STATE_FILE"
        fi
    else
        echo -e "${YELLOW}⚠${NC} Build failed - this is normal for a work-in-progress"
        echo "    You can start fixing build issues as your first task"
    fi
}

# Run basic tests
run_basic_tests() {
    echo -e "${BLUE}Running basic tests...${NC}"
    
    if cargo test --workspace; then
        echo -e "${GREEN}✓${NC} Tests pass"
        
        # Update state
        if [ -f "$STATE_FILE" ]; then
            local temp_file=$(mktemp)
            jq '.last_test = now | .last_test_success = true' "$STATE_FILE" > "$temp_file"
            mv "$temp_file" "$STATE_FILE"
        fi
    else
        echo -e "${YELLOW}⚠${NC} Some tests fail - you may need to fix these"
    fi
}

# Print guidance for agent
print_agent_guidance() {
    echo
    echo "════════════════════════════════════════════════════════════════"
    echo -e "${GREEN}Agent Bootstrap Complete${NC}"
    echo "════════════════════════════════════════════════════════════════"
    echo
    echo "Welcome to the $PROJECT_NAME project!"
    echo
    echo "📁 Your workspace is ready at: $PROJECT_ROOT"
    echo "📝 Your notes file is at: $NOTES_FILE"
    echo "📖 Context reference is at: $PROJECT_ROOT/AGENT_CONTEXT.md"
    echo
    echo "🎯 Suggested first tasks:"
    echo "  1. Review AGENT_CONTEXT.md for project overview"
    echo "  2. Run './scripts/dev.sh quick' for a quick check"
    echo "  3. Review .agents/rules/ for coding standards"
    echo "  4. Check current issues with './scripts/dev.sh check'"
    echo
    echo "🔧 Useful commands:"
    echo "  ./scripts/dev.sh build    # Build project"
    echo "  ./scripts/dev.sh test     # Run tests"
    echo "  ./scripts/dev.sh fix      # Auto-fix issues"
    echo "  ./scripts/dev.sh ci       # Full CI check"
    echo
    echo "💡 Tips:"
    echo "  - Keep notes in notes.txt as you work"
    echo "  - Run 'dev.sh quick' frequently for fast feedback"
    echo "  - Use 'dev.sh fix' to auto-resolve common issues"
    echo "  - Check .agents/rules/ for detailed guidelines"
    echo
    echo "Ready to start development! What would you like to work on?"
    echo "════════════════════════════════════════════════════════════════"
}

# Main execution
main() {
    echo "════════════════════════════════════════════════════════════════"
    echo -e "${BLUE}AI Agent Bootstrap for $PROJECT_NAME${NC}"
    echo "════════════════════════════════════════════════════════════════"
    echo
    
    cd "$PROJECT_ROOT"
    
    # Create necessary directories
    mkdir -p "$AGENT_DIR"
    
    # Setup steps
    create_agent_context
    init_agent_state
    create_notes_file
    create_knowledge_entries
    setup_environment
    verify_build
    run_basic_tests
    
    # Print guidance
    print_agent_guidance
}

# Check for jq (used for JSON manipulation)
if ! command -v jq >/dev/null 2>&1; then
    echo -e "${YELLOW}Note: jq not installed (optional for state tracking)${NC}"
fi

# Run main
main "$@"
