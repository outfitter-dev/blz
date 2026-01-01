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
# Colon syntax (preferred, matches search output)
blz find bun:41994-42009

# Multiple ranges (comma-separated)
blz find bun:41994-42009,42010-42020 --json

# Multiple sources in one call
blz find bun:41994-42009,42010-42020 turbo:2656-2729 --json

# Heading-aware retrieval (entire section, capped at 80 lines)
blz find bun:41994-42009 --context all --max-lines 80 --json

# Add context lines without blocks
blz find bun:25760-25780 -C 3
```

### Managing Sources

```bash
# List all sources
blz list

# With metadata
blz list --details

# Refresh single source
blz refresh bun  # deprecated alias: blz update bun

# Refresh all sources
blz refresh --all  # deprecated alias: blz update --all

# Remove source
blz remove bun
```

### Exploring Headings

```bash
# Inspect heading hierarchy
blz toc bun --limit 20

# Boolean filter (API but not deprecated)
blz toc react --filter "API AND NOT deprecated" --format json
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
blz find bun:304-324 -C 5
```

### Refresh All Sources Daily

```bash
# Add to cron/launchd
blz refresh --all  # deprecated alias: blz update --all
```

### Integration with Scripts

```bash
#!/bin/bash
# Search and open in editor
result=$(blz "$1" -n1 --json)
alias=$(echo "$result" | jq -r '.results[0].alias')
lines=$(echo "$result" | jq -r '.results[0].lines')
blz find "$alias:$lines"
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
blz find --help

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
