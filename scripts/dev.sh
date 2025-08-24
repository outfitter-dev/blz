#!/usr/bin/env bash
#
# dev.sh - Quick development commands for blz
# Provides common development workflows for AI agents and developers

# Set script directory before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common configuration and utilities
source "${SCRIPT_DIR}/common.sh"

# Command functions
cmd_build() {
    log_section "Building project"
    
    local build_type="${1:-release}"
    
    case "$build_type" in
        debug|dev)
            log_info "Building in debug mode..."
            cargo build
            log_success "Debug build complete: target/debug/$BINARY_NAME"
            ;;
        release|prod)
            log_info "Building in release mode..."
            cargo build --release
            log_success "Release build complete: target/release/$BINARY_NAME"
            ;;
        *)
            log_error "Unknown build type: $build_type"
            echo "Usage: $0 build [debug|release]"
            exit 1
            ;;
    esac
}

cmd_test() {
    log_section "Running tests"
    
    local test_type="${1:-all}"
    
    case "$test_type" in
        all)
            log_info "Running all tests..."
            cargo test --workspace
            ;;
        unit)
            log_info "Running unit tests..."
            cargo test --lib
            ;;
        integration)
            log_info "Running integration tests..."
            cargo test --test '*'
            ;;
        doc)
            log_info "Running documentation tests..."
            cargo test --doc
            ;;
        *)
            log_info "Running tests matching: $test_type"
            cargo test "$test_type"
            ;;
    esac
    
    log_success "Tests completed"
}

cmd_check() {
    log_section "Running quality checks"
    
    local failed=false
    
    # Format check
    log_info "Checking formatting..."
    if ! cargo fmt --check; then
        log_error "Formatting issues found. Run: cargo fmt"
        failed=true
    else
        log_success "Formatting OK"
    fi
    
    # Clippy
    log_info "Running clippy..."
    if ! cargo clippy --workspace -- -D warnings; then
        log_error "Clippy warnings found"
        failed=true
    else
        log_success "Clippy OK"
    fi
    
    # Deny
    if command -v cargo-deny >/dev/null 2>&1; then
        log_info "Checking dependencies..."
        if ! cargo deny check; then
            log_error "Dependency issues found"
            failed=true
        else
            log_success "Dependencies OK"
        fi
    fi
    
    # Shear
    if command -v cargo-shear >/dev/null 2>&1; then
        log_info "Checking for unused dependencies..."
        if ! cargo shear; then
            log_error "Unused dependencies found"
            failed=true
        else
            log_success "No unused dependencies"
        fi
    fi
    
    if [ "$failed" = true ]; then
        log_error "Some checks failed"
        exit 1
    else
        log_success "All checks passed"
    fi
}

cmd_fix() {
    log_section "Fixing common issues"
    
    # Format code
    log_info "Formatting code..."
    cargo fmt
    log_success "Code formatted"
    
    # Fix clippy warnings
    log_info "Fixing clippy warnings..."
    cargo clippy --fix --allow-dirty --allow-staged || true
    log_success "Clippy fixes applied"
    
    # Update dependencies
    log_info "Updating dependencies..."
    cargo update
    log_success "Dependencies updated"
}

cmd_bench() {
    log_section "Running benchmarks"
    
    log_info "Running performance benchmarks..."
    cargo bench
    
    log_success "Benchmarks complete"
    echo "Results saved in: target/criterion/"
}

cmd_doc() {
    log_section "Building documentation"
    
    local open_docs="${1:-}"
    
    log_info "Building documentation..."
    cargo doc --no-deps --document-private-items
    
    if [ "$open_docs" = "--open" ] || [ "$open_docs" = "open" ]; then
        cargo doc --open
    else
        log_success "Documentation built"
        echo "View at: target/doc/blz_core/index.html"
    fi
}

cmd_clean() {
    log_section "Cleaning build artifacts"
    
    log_info "Removing target directory..."
    cargo clean
    
    log_info "Removing temporary files..."
    find . -name "*.orig" -type f -delete 2>/dev/null || true
    find . -name "*.rej" -type f -delete 2>/dev/null || true
    
    log_success "Clean complete"
}

cmd_coverage() {
    log_section "Generating code coverage"
    
    if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
        log_error "cargo-llvm-cov not installed"
        echo "Install with: cargo install cargo-llvm-cov"
        exit 1
    fi
    
    log_info "Running tests with coverage..."
    cargo llvm-cov --workspace --html
    
    log_success "Coverage report generated"
    echo "View at: target/llvm-cov/html/index.html"
}

cmd_watch() {
    log_section "Starting file watcher"
    
    if ! command -v cargo-watch >/dev/null 2>&1; then
        log_error "cargo-watch not installed"
        echo "Install with: cargo install cargo-watch"
        exit 1
    fi
    
    local watch_cmd="${1:-test}"
    
    case "$watch_cmd" in
        test)
            log_info "Watching for changes and running tests..."
            cargo watch -x test
            ;;
        build)
            log_info "Watching for changes and building..."
            cargo watch -x build
            ;;
        check)
            log_info "Watching for changes and checking..."
            cargo watch -x check
            ;;
        *)
            log_info "Watching for changes and running: $watch_cmd"
            cargo watch -x "$watch_cmd"
            ;;
    esac
}

cmd_ci() {
    log_section "Running full CI pipeline"
    
    log_info "This will run all checks that CI runs..."
    
    # Run all checks in sequence
    cmd_test all
    cmd_check
    cmd_doc
    
    log_success "CI pipeline complete"
}

cmd_quick() {
    log_section "Quick check (fast feedback)"
    
    log_info "Running quick checks..."
    
    # Just check if it compiles and basic tests pass
    cargo check --workspace
    cargo test --workspace --lib
    
    log_success "Quick check complete"
}

show_help() {
    cat << EOF
blz Development Helper

Usage: $0 <command> [options]

Commands:
    build [debug|release]   Build the project
    test [all|unit|doc]     Run tests
    check                   Run quality checks (fmt, clippy, deny)
    fix                     Fix common issues automatically
    bench                   Run benchmarks
    doc [--open]           Build documentation
    clean                   Clean build artifacts
    coverage               Generate code coverage report
    watch [test|build]     Watch files and run commands
    ci                     Run full CI pipeline locally
    quick                  Quick compile and test check
    help                   Show this help message

Examples:
    $0 build               # Build in release mode
    $0 test unit          # Run only unit tests
    $0 check              # Run all quality checks
    $0 watch test         # Watch and test on changes
    $0 ci                 # Run full CI locally

For AI Agents:
    - Use 'quick' for fast feedback during development
    - Use 'fix' to automatically resolve common issues
    - Use 'ci' before committing to ensure all checks pass

EOF
}

# Main execution
main() {
    cd "$PROJECT_ROOT"
    
    local cmd="${1:-help}"
    shift || true
    
    case "$cmd" in
        build)
            cmd_build "$@"
            ;;
        test)
            cmd_test "$@"
            ;;
        check)
            cmd_check "$@"
            ;;
        fix)
            cmd_fix "$@"
            ;;
        bench)
            cmd_bench "$@"
            ;;
        doc)
            cmd_doc "$@"
            ;;
        clean)
            cmd_clean "$@"
            ;;
        coverage)
            cmd_coverage "$@"
            ;;
        watch)
            cmd_watch "$@"
            ;;
        ci)
            cmd_ci "$@"
            ;;
        quick)
            cmd_quick "$@"
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            log_error "Unknown command: $cmd"
            show_help
            exit 1
            ;;
    esac
}

main "$@"