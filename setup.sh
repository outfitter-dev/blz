#!/usr/bin/env bash
#
# setup.sh - Comprehensive setup script for blz
# Optimized for AI coding agents (Devin.ai, Factory.ai, Codex, etc.)
#
# This script provides idempotent environment setup with clear progress indicators
# and handles common failure modes gracefully.

# Set script directory before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common configuration and utilities
source "${SCRIPT_DIR}/scripts/common.sh"

# Progress tracking for AI agents
STEP_COUNT=0
TOTAL_STEPS=10

# Override log_step to include counter
log_step_with_counter() {
    STEP_COUNT=$((STEP_COUNT + 1))
    log_step_with_counter "${STEP_COUNT}" "${TOTAL_STEPS}" "$1"
}

# Main setup functions
setup_rust() {
    log_step_with_counter "Checking Rust installation..."
    
    if command_exists rustc; then
        RUST_VERSION=$(rustc --version | cut -d' ' -f2)
        if version_ge "$RUST_VERSION" "$REQUIRED_RUST_VERSION"; then
            log_success "Rust $RUST_VERSION is installed (required: $REQUIRED_RUST_VERSION)"
        else
            log_warning "Rust $RUST_VERSION is installed but version $REQUIRED_RUST_VERSION is recommended"
            echo "    Run: rustup update"
        fi
    else
        log_error "Rust is not installed"
        echo "    Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    # Ensure we have required components
    log_step_with_counter "Installing Rust components..."
    rustup component add clippy rustfmt rust-src 2>/dev/null || true
    log_success "Rust components installed"
}

setup_cargo_tools() {
    log_step_with_counter "Installing required cargo tools..."
    
    local tools=("cargo-deny" "cargo-shear")
    local installed=0
    local failed=0
    
    for tool in "${tools[@]}"; do
        if ! command_exists "$tool"; then
            echo "    Installing $tool..."
            if cargo install "$tool" >/dev/null 2>&1; then
                installed=$((installed + 1))
            else
                log_warning "Failed to install $tool (non-critical)"
                failed=$((failed + 1))
            fi
        fi
    done
    
    if [ $installed -gt 0 ]; then
        log_success "Installed $installed cargo tools"
    fi
    
    if [ $failed -gt 0 ]; then
        log_warning "$failed tools failed to install (build will still work)"
    fi
}

setup_dev_tools() {
    log_step_with_counter "Installing optional development tools..."
    
    local optional_tools=("cargo-watch" "flamegraph" "cargo-llvm-cov")
    local installed=0
    
    for tool in "${optional_tools[@]}"; do
        if ! command_exists "$tool"; then
            echo "    Optional: $tool (install with 'cargo install $tool')"
        else
            installed=$((installed + 1))
        fi
    done
    
    log_success "$installed optional tools already installed"
}

build_project() {
    log_step_with_counter "Building project..."
    
    cd "$PROJECT_ROOT"
    
    # Clean build to ensure fresh state
    if [ -d "target" ]; then
        echo "    Cleaning previous build..."
        cargo clean
    fi
    
    echo "    Building in release mode..."
    if cargo build --release; then
        log_success "Build completed successfully"
        
        # Check binary exists
        if [ -f "target/release/$BINARY_NAME" ]; then
            log_success "Binary created: target/release/$BINARY_NAME"
        fi
    else
        log_error "Build failed"
        return 1
    fi
}

run_tests() {
    log_step_with_counter "Running tests..."
    
    cd "$PROJECT_ROOT"
    
    if cargo test --workspace; then
        log_success "All tests passed"
    else
        log_warning "Some tests failed (non-critical for setup)"
    fi
}

run_quality_checks() {
    log_step_with_counter "Running quality checks..."
    
    cd "$PROJECT_ROOT"
    
    local checks_passed=true
    
    # Format check
    echo "    Checking code formatting..."
    if ! cargo fmt --check >/dev/null 2>&1; then
        log_warning "Code needs formatting (run: cargo fmt)"
        checks_passed=false
    fi
    
    # Clippy check
    echo "    Running clippy..."
    if ! cargo clippy --workspace -- -D warnings >/dev/null 2>&1; then
        log_warning "Clippy found issues (run: cargo clippy)"
        checks_passed=false
    fi
    
    if [ "$checks_passed" = true ]; then
        log_success "All quality checks passed"
    else
        log_warning "Some quality checks need attention"
    fi
}

setup_shell_completions() {
    log_step_with_counter "Setting up shell completions..."
    
    if [ -f "$PROJECT_ROOT/scripts/install-completions.sh" ]; then
        if bash "$PROJECT_ROOT/scripts/install-completions.sh" "$BINARY_NAME" >/dev/null 2>&1; then
            log_success "Shell completions installed"
        else
            log_warning "Shell completions installation failed (non-critical)"
        fi
    else
        log_warning "Completions script not found (non-critical)"
    fi
}

create_development_env() {
    log_step_with_counter "Creating development environment files..."
    
    # Create .env.example if it doesn't exist
    if [ ! -f "$PROJECT_ROOT/.env.example" ]; then
        cat > "$PROJECT_ROOT/.env.example" << 'EOF'
# Development environment variables
RUST_LOG=debug
RUST_BACKTRACE=1
RUST_TEST_THREADS=4

# Optional: Performance profiling
# CARGO_PROFILE_RELEASE_DEBUG=true
EOF
        log_success "Created .env.example"
    fi
    
    # Create notes.txt for AI agents
    if [ ! -f "$PROJECT_ROOT/notes.txt" ]; then
        cat > "$PROJECT_ROOT/notes.txt" << 'EOF'
# Development Notes

## Quick Commands
- Build: `cargo build --release`
- Test: `cargo test`
- Run: `./target/release/blz search "query"`
- Quality: `make ci` or `just ci`

## Project Structure
- blz-core: Core search functionality
- blz-cli: Command-line interface
- blz-mcp: MCP server (in development)

## Common Tasks
- Format code: `cargo fmt`
- Lint: `cargo clippy -- -D warnings`
- Security check: `cargo deny check`
- Benchmarks: `cargo bench`

## Documentation
- View docs: `cargo doc --open`
- Agent rules: `.agents/rules/`
EOF
        log_success "Created notes.txt for agent context"
    fi
}

print_summary() {
    echo
    echo "════════════════════════════════════════════════════════════════"
    echo -e "${GREEN}Setup Complete!${NC}"
    echo "════════════════════════════════════════════════════════════════"
    echo
    echo "Project: blz (local search for llms.txt documentation)"
    echo "Location: $PROJECT_ROOT"
    echo
    echo "Next steps:"
    echo "  1. Run the binary: ./target/release/$BINARY_NAME --help"
    echo "  2. Run quality checks: make ci"
    echo "  3. View documentation: cargo doc --open"
    echo
    echo "For AI agents:"
    echo "  - Configuration: See .agents/rules/ for development guidelines"
    echo "  - Quick reference: See notes.txt for common commands"
    echo "  - Environment: Copy .env.example to .env for local settings"
    echo
    echo "Binary location: $PROJECT_ROOT/target/release/$BINARY_NAME"
    echo "════════════════════════════════════════════════════════════════"
}

print_error_summary() {
    echo
    echo "════════════════════════════════════════════════════════════════"
    echo -e "${RED}Setup Failed${NC}"
    echo "════════════════════════════════════════════════════════════════"
    echo
    echo "The setup process encountered errors. Please check:"
    echo "  1. Rust is installed: rustup.rs"
    echo "  2. You have internet connection for downloading dependencies"
    echo "  3. You have write permissions in the current directory"
    echo
    echo "For debugging, run with verbose output:"
    echo "  RUST_BACKTRACE=1 cargo build --verbose"
    echo
    echo "For help, see:"
    echo "  - README.md"
    echo "  - .agents/rules/DEVELOPMENT.md"
    echo "════════════════════════════════════════════════════════════════"
}

# Main execution
main() {
    echo "════════════════════════════════════════════════════════════════"
    echo "blz Setup Script"
    echo "════════════════════════════════════════════════════════════════"
    echo
    
    # Change to project root
    cd "$PROJECT_ROOT"
    
    # Track if any critical step fails
    local setup_failed=false
    
    # Execute setup steps
    setup_rust || setup_failed=true
    
    if [ "$setup_failed" = false ]; then
        setup_cargo_tools
        setup_dev_tools
        build_project || setup_failed=true
        run_tests
        run_quality_checks
        setup_shell_completions
        create_development_env
    fi
    
    # Print summary
    if [ "$setup_failed" = false ]; then
        print_summary
        exit 0
    else
        print_error_summary
        exit 1
    fi
}

# Run main function
main "$@"