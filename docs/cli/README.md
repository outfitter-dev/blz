# BLZ CLI

Command-line interface for searching and managing local `llms.txt` documentation caches.

## Installation

### Quick Install (macOS/Linux)

**No prerequisites needed** - install script handles everything:

```bash
curl -fsSL https://blz.run/install.sh | sh
```

The script installs the latest release to `~/.local/bin`. Override the location with `BLZ_INSTALL_DIR=/path`, or pin a version using `BLZ_VERSION=v0.4.1`.

### Install from Source

If you prefer building from source, you'll need:
- Rust 1.75+ and Cargo (install from [rustup.rs](https://rustup.rs))
- Git

```bash
# Clone the repository
git clone https://github.com/outfitter-dev/blz
cd blz

# Install the binary
cargo install --path crates/blz-cli

# Verify installation
blz --help
```

### Install from GitHub

```bash
# Direct install from GitHub
cargo install --git https://github.com/outfitter-dev/blz blz-cli
```

## Quick Start

```bash
# Add documentation source
blz add bun https://bun.sh/llms.txt

# Search
blz "test runner" -s bun

# Get exact lines
blz get bun:304-324

# List all sources
blz list
```

## CLI Documentation

### Guides

- [**How-To Guide**](howto.md) - Task-oriented "I want to..." solutions for common tasks
- [**Search Guide**](search.md) - Search syntax, performance tips, and advanced queries
- [**Sources**](sources.md) - Managing documentation sources

### Reference

- [**Commands**](commands.md) - Complete command reference
- [**Configuration**](configuration.md) - Global config, per-source settings, env vars
- [**Shell Integration**](shell_integration.md) - Setup for Bash, Zsh, Fish, PowerShell, Elvish

## Common Tasks

### Add documentation sources

```bash
# Add single source
blz add react https://react.dev/llms.txt

# Discover sources in registry
blz lookup typescript

# Add from manifest file
blz add --manifest sources.toml
```

### Search documentation

```bash
# Search all sources
blz "async await"

# Search specific source
blz "hooks" -s react

# Limit results
blz "testing" -n10

# JSON output
blz "api" --json
```

### Manage sources

```bash
# List all sources
blz list

# Update sources
blz update --all
blz update react

# Remove source
blz remove react

# Upgrade to llms-full.txt
blz upgrade --all
```

### Get exact content

```bash
# Get line range
blz get react:120-145

# With context lines
blz get react:120-145 -c5

# Copy to clipboard
blz get react:120-145 --copy
```

## Global Options

```
  -h, --help      Print help
  -V, --version   Print version
      --verbose   Enable verbose output
      --debug     Show detailed performance metrics
      --profile   Show resource usage (memory, CPU)
      --config <FILE>      Path to configuration file
      --config-dir <DIR>   Directory containing config.toml
```

## Output Formats

BLZ supports multiple output formats for different use cases:

- **text** (default) - Human-readable output with colors
- **json** - Machine-readable JSON array (use `--json` shortcut)
- **jsonl** - JSON Lines format for streaming (use `--jsonl` shortcut)

```bash
# JSON output
blz "query" --json

# JSON Lines for streaming
blz "query" --jsonl
```

## Environment Variables

- `BLZ_DATA_DIR` - Override data directory
- `BLZ_GLOBAL_CONFIG_DIR` - Override config directory
- `BLZ_OUTPUT_FORMAT` - Default output format (text, json, jsonl)

See [Configuration](configuration.md) for more details.

## See Also

- [Quick Start](../QUICKSTART.md) - First-time setup walkthrough
- [Architecture](../architecture/README.md) - How BLZ works
- [Contributing](../../CONTRIBUTING.md) - Development guidelines
