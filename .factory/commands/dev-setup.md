---
description: Set up development environment for blz
argument-hint: [quick|full]
---

# blz Development Environment Setup

Please help set up the development environment for the blz project. Mode: $ARGUMENTS

## Setup Tasks

### 1. **Environment Validation**
- Check Rust toolchain (verify rust-toolchain.toml requirements)
- Validate required system dependencies
- Confirm git hooks and pre-commit setup
- Check Node.js for documentation tooling

### 2. **Build & Dependencies**
- Run `cargo build --release` to ensure clean compilation
- Execute `cargo test` to verify all tests pass
- Check `cargo clippy` for linting issues
- Run formatting with `cargo fmt --check`

### 3. **Development Tools**
- Install local blz binary with `cargo install --path crates/blz-cli`
- Set up shell completions for enhanced development experience
- Configure IDE/editor settings if applicable
- Validate benchmark environment with `cargo bench --dry-run`

### 4. **Project Validation**
- Run the smoke test script: `./scripts/agent-check.sh`
- Verify documentation builds correctly
- Test CLI with sample sources from registry
- Check that all scripts in `./scripts/` are executable and functional

### 5. **Development Workflow**
- Explain the git workflow and branch naming conventions
- Show how to run specific test suites efficiently
- Demonstrate performance profiling setup
- Guide through the release process basics

## Setup Mode Instructions

**Quick Mode** ($ARGUMENTS contains "quick"):
- Focus on essential build and test validation
- Skip optional tooling and advanced setup
- Provide minimal viable development environment

**Full Mode** (default or $ARGUMENTS contains "full"):
- Complete development environment setup
- Include all optional tools and configurations
- Provide comprehensive workflow guidance
- Set up performance monitoring and profiling tools

## Expected Outcome

After completion, the developer should have:
1. A fully functional build environment
2. All tests passing locally
3. CLI tool installed and accessible
4. Understanding of the development workflow
5. Access to debugging and profiling tools

If any setup step fails, please:
- Provide specific error diagnosis
- Suggest concrete resolution steps
- Offer alternative approaches when possible
- Document any known issues or workarounds