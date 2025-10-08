# BLZ Quick Reference

One-page guide to common BLZ operations.

## Installation

```bash
# Quick install
curl -fsSL https://blz.run/install.sh | sh

# Verify
blz --version
```

## Common Commands

### Adding Sources

```bash
# Add single source
blz add <alias> <url>
blz add bun https://bun.sh/llms.txt

# Non-interactive (scripts)
blz add bun https://bun.sh/llms.txt -y

# Discover sources
blz lookup react

# Batch add from manifest
blz add --manifest sources.toml
```

### Searching

```bash
# Basic search (all sources)
blz "query"

# Filter by source
blz "query" -s bun

# JSON output
blz "query" --json

# Limit results
blz "query" -n5

# Paginate
blz "query" --page 2
```

### Getting Content

```bash
# Colon syntax (preferred)
blz get bun:120-142

# With context lines
blz get bun:120-142 -c3

# Multiple ranges
blz get bun:36:43,320:350

# JSON output
blz get bun:120-142 --json
```

### Managing Sources

```bash
# List all sources
blz list

# With metadata
blz list --details

# Update single source
blz update bun

# Update all sources
blz update --all

# Remove source
blz remove bun
```

### History

```bash
# View recent searches
blz history -n10

# JSON output
blz history --json
```

## Query Syntax

| Syntax | Meaning | Example |
|--------|---------|---------|
| `term1 term2` | OR (match any) | `blz "test runner"` |
| `+term` | AND (require) | `blz "+api +key"` |
| `"exact phrase"` | Exact match | `blz '"use strict"'` |

## Output Formats

| Flag | Format | Use Case |
|------|--------|----------|
| *(default)* | Human-readable | Terminal viewing |
| `--json` | JSON object | Scripting, agents |
| `--jsonl` | JSON Lines | Streaming |

## Common Patterns

### Search → Get Workflow

```bash
# 1. Find relevant content
blz "test runner" --json | jq -r '.results[0] | "\(.alias):\(.lines)"'
# Output: bun:304-324

# 2. Get full context
blz get bun:304-324 -c5
```

### Update All Sources Daily

```bash
# Add to cron/launchd
blz update --all
```

### Integration with Scripts

```bash
#!/bin/bash
# Search and open in editor
result=$(blz "$1" -n1 --json)
alias=$(echo "$result" | jq -r '.results[0].alias')
lines=$(echo "$result" | jq -r '.results[0].lines')
blz get "$alias:$lines"
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `command not found` | Add `~/.local/bin` to PATH |
| Empty search results | Check source exists: `blz list` |
| Slow search | First search caches to disk (subsequent faster) |
| Source fetch fails | Check URL is valid llms.txt |

## Environment Variables

```bash
export BLZ_OUTPUT_FORMAT=json        # Default output format
export BLZ_DATA_DIR=/custom/path     # Override data directory
export BLZ_CONFIG_DIR=/custom/path   # Override config directory
```

## Getting Help

```bash
# General help
blz --help

# Command-specific help
blz search --help
blz get --help

# Agent integration guidance
blz --prompt
blz --prompt search

# Generate CLI docs
blz docs
blz docs --json
```

## Shell Completions

```bash
# Fish
blz completions fish > ~/.config/fish/completions/blz.fish

# Bash
blz completions bash > ~/.local/share/bash-completion/completions/blz

# Zsh
blz completions zsh > ~/.zsh/completions/_blz
```

## Command Aliases

| Command | Alias | Description |
|---------|-------|-------------|
| `blz list` | `blz sources` | List sources |
| `blz remove` | `blz rm`, `blz delete` | Remove source |

## Global Options

```
  -h, --help      Print help
  -V, --version   Print version
      --verbose   Enable verbose output
      --debug     Show detailed performance metrics
      --profile   Show resource usage (memory, CPU)
```

## Links

- [Full Documentation](../README.md)
- [Quick Start](../QUICKSTART.md)
- [How-To Guide](howto.md)
- [Commands Reference](commands.md)
- [Search Guide](search.md)
- [Troubleshooting](troubleshooting.md)
- [GitHub Issues](https://github.com/outfitter-dev/blz/issues)
