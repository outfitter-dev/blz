# Development Scripts

This directory contains scripts to help with development, optimized for both human developers and AI coding agents.

## Main Setup Script

### `../setup.sh`
The primary setup script for getting a development environment ready.
- Installs Rust toolchain if needed
- Installs required cargo tools
- Builds the project
- Runs tests and quality checks
- Sets up shell completions
- Creates development environment files

**Usage:**
```bash
./setup.sh
```

## Individual Scripts

### `check-environment.sh`
Verifies that the development environment meets all requirements.
- Checks OS compatibility
- Verifies Rust installation and version
- Checks for required build tools
- Tests network connectivity
- Validates file permissions

**Usage:**
```bash
./scripts/check-environment.sh
```

### `dev.sh`
Quick development commands for common tasks.

**Commands:**
- `build [debug|release]` - Build the project
- `test [all|unit|doc]` - Run tests
- `check` - Run quality checks (fmt, clippy, deny)
- `fix` - Fix common issues automatically
- `bench` - Run benchmarks
- `doc [--open]` - Build documentation
- `clean` - Clean build artifacts
- `coverage` - Generate code coverage report
- `watch [test|build]` - Watch files and run commands
- `ci` - Run full CI pipeline locally
- `quick` - Quick compile and test check

**Usage:**
```bash
./scripts/dev.sh build release
./scripts/dev.sh test unit
./scripts/dev.sh check
./scripts/dev.sh fix
./scripts/dev.sh ci
```

### `agent-bootstrap.sh`
Bootstrap script specifically designed for AI coding agents (Devin.ai, Factory.ai, Codex, etc.).
- Creates AGENT_CONTEXT.md with project overview
- Initializes agent state tracking
- Creates notes.txt for persistent notes
- Sets up knowledge base entries
- Provides clear guidance and next steps

**Usage:**
```bash
./scripts/agent-bootstrap.sh
```

### `install-completions.sh`
Installs shell completions for bash, zsh, and fish.

**Usage:**
```bash
./scripts/install-completions.sh blz
```

## For AI Agents

If you're an AI coding agent, start with:

1. **First time setup:**
   ```bash
   ./scripts/agent-bootstrap.sh
   ```
   This creates context files and guides you through the project.

2. **Daily development:**
   ```bash
   ./scripts/dev.sh quick  # Fast feedback during development
   ./scripts/dev.sh fix    # Auto-fix common issues
   ./scripts/dev.sh ci     # Full check before committing
   ```

3. **Keep notes:**
   - Write to `notes.txt` to track your progress
   - Check `AGENT_CONTEXT.md` for project overview
   - Review `.agent/rules/` for coding standards

## Common Workflows

### Starting fresh
```bash
./setup.sh                    # Full environment setup
./scripts/dev.sh build        # Build the project
./scripts/dev.sh test         # Run tests
```

### Before committing
```bash
./scripts/dev.sh ci           # Run all checks
# or manually:
./scripts/dev.sh fix          # Auto-fix issues
./scripts/dev.sh check        # Verify quality
```

### Continuous development
```bash
./scripts/dev.sh watch test   # Auto-run tests on file changes
```

### Debugging issues
```bash
./scripts/check-environment.sh  # Verify environment
./scripts/dev.sh clean          # Clean and rebuild
./scripts/dev.sh build debug    # Debug build for better errors
```

## Script Principles

All scripts follow these principles:

1. **Idempotent** - Can be run multiple times safely
2. **Clear output** - Color-coded status messages
3. **Graceful failures** - Handle errors and provide solutions
4. **AI-friendly** - Clear progress indicators and context
5. **DRY** - Reusable functions across scripts

## Adding New Scripts

When adding new scripts:

1. Follow the existing patterns for output and error handling
2. Make scripts idempotent
3. Add clear help text
4. Document in this README
5. Make executable: `chmod +x script-name.sh`