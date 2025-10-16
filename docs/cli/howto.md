# CLI How-To Guide

Task-oriented guide for common BLZ CLI tasks. Each section answers "I want to..." questions with quick, actionable steps.

## Quick Navigation

- [Get Started](#get-started)
- [Search & Find](#search--find)
- [Manage Sources](#manage-sources)
- [Integrate with Tools](#integrate-with-tools)
- [Troubleshoot Issues](#troubleshoot-issues)

---

## Get Started

### I want to install BLZ

**Fastest way**:

```bash
curl -fsSL https://blz.run/install.sh | sh
```

**From source**:

```bash
cargo install --git https://github.com/outfitter-dev/blz blz-cli
```

**Verify installation**:

```bash
blz --version
```

### I want to add my first documentation source

1. Find the llms.txt URL (usually `https://example.com/llms.txt`)
2. Add it with a memorable alias:

```bash
blz add <alias> <url>
# Example:
blz add bun https://bun.sh/llms.txt
```

### I want to discover available documentation sources

```bash
# Search curated registry
blz lookup react

# Browse all registry sources
blz lookup ""
```

---

## Search & Find

### I want to search across all my documentation

```bash
blz "your query"
```

**Tips**:

- Results show source, heading path, and lines
- Default shows top 50 results
- Use `-n N` to show more/fewer

### I want to search in a specific documentation set

```bash
blz "query" -s bun
```

### I want to search with AND logic (require all terms)

```bash
blz "+api +authentication"
```

This ensures both "api" AND "authentication" appear in results.

### I want to search for an exact phrase

```bash
blz '"exact phrase here"'
```

Note: Use single quotes around double quotes for shell escaping.

### I want JSON output for scripting

```bash
blz "query" --json

# Or set default format
export BLZ_OUTPUT_FORMAT=json
blz "query"
```

### I want to get specific lines from a search result

After searching:

```bash
# Copy the alias:lines from the result (e.g., bun:41994-42009)
blz get bun:41994-42009

# Include ±5 lines of context without changing the span
blz get bun:41994-42009 -C 5

# Merge multiple spans for the same source
blz get bun:41994-42009,42010-42020 --json

blz get bun:41994-42009,42010-42020 turbo:2656-2729 --json

# Pull the entire heading block (great for sections with tables or prose)
blz get bun:41994-42009 --context all --max-lines 80 --json
```

### I want to limit search results

```bash
# Get top 10 results
blz "query" -n10

# Get just the best match
blz "query" -n1
```

### I want to paginate through results

```bash
# First page (default)
blz "query"

# Second page
blz "query" --page 2

# Jump to last page
blz "query" --last
```

---

## Manage Sources

### I want to see all my indexed sources

```bash
blz list
```

### I want details about my sources

```bash
# Include fetch metadata
blz list --status

# Include descriptor metadata
blz list --details

# JSON output for scripting
blz list --json
```

### I want to update all my sources

```bash
blz update --all
```

### I want to update one specific source

```bash
blz update bun
```

### I want to remove a source

```bash
blz remove bun

# Skip confirmation prompt
blz remove bun --yes
```

### I want to upgrade from llms.txt to llms-full.txt

```bash
# Upgrade specific source
blz upgrade bun

# Check all sources for upgrades
blz upgrade --all
```

### I want to add multiple sources at once

Create a `sources.toml` manifest file:

```toml
version = "1"

[[source]]
alias = "bun"
url = "https://bun.sh/llms-full.txt"

[[source]]
alias = "node"
url = "https://nodejs.org/llms.txt"

[[source]]
alias = "deno"
url = "https://deno.land/llms.txt"
```

Then:

```bash
blz add --manifest sources.toml
```

### I want to add a source with metadata

```bash
blz add react https://react.dev/llms.txt \
  --name "React" \
  --category framework \
  --tags javascript,ui,library
```

### I want to manage aliases for a source

```bash
# Add an alias
blz alias add react @facebook/react

# Remove an alias
blz alias rm react @facebook/react
```

---

## Integrate with Tools

### I want to use BLZ in my shell scripts

```bash
#!/bin/bash
# Search and get top result

query="$1"
result=$(blz "$query" -n1 --json)

# Check if we got results
if [ "$(echo "$result" | jq '.totalResults')" -gt 0 ]; then
  # Extract first result
  first=$(echo "$result" | jq -r '.results[0]')
  alias=$(echo "$first" | jq -r '.alias')
  lines=$(echo "$first" | jq -r '.lines')

  echo "Best match: $alias at lines $lines"
  blz get "$alias:$lines"
else
  echo "No results found for: $query"
fi
```

### I want to pipe search results to other tools

```bash
# Get just the snippets
blz "query" --json | jq -r '.results[].snippet'

# Get source:lines for each result
blz "query" --json | jq -r '.results[] | "\(.alias):\(.lines)"'

# Count results by source
blz "query" --json | jq '.results | group_by(.alias) | map({alias: .[0].alias, count: length})'
```

### I want to use BLZ with AI agents

See detailed agent integration patterns in:

- [Commands Reference](commands.md#blz---prompt) (`--prompt` flag)
- `docs/agents/use-blz.md` (agent-specific patterns)

Quick example:

```bash
# Get agent-focused guidance
blz --prompt
blz --prompt search

# Output JSON for agents
export BLZ_OUTPUT_FORMAT=json
blz "query"
```

### I want shell completions

```bash
# Bash
blz completions bash > ~/.local/share/bash-completion/completions/blz

# Zsh
blz completions zsh > ~/.zsh/completions/_blz

# Fish
blz completions fish > ~/.config/fish/completions/blz.fish
```

See [Shell Integration](shell_integration.md) for detailed setup.

### I want to view my search history

```bash
# Recent searches
blz history

# Last 5 searches
blz history -n5

# JSON output
blz history --json
```

### I want to customize output preferences

```bash
# Set global defaults interactively
blz config

# Set prefer_full globally
blz config set add.prefer_full true

# Override for current directory only
blz config set add.prefer_full false --scope local
```

---

## Troubleshoot Issues

### I get "command not found: blz"

Make sure install directory is in PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"

# Add to shell config to persist
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc  # or ~/.zshrc
```

### Search returns no results

1. Check you have sources: `blz list`
2. Verify source is indexed: `blz list --json | jq '.[].alias'`
3. Try simpler search terms

### Search is too slow

First search may be slower (OS caching). Subsequent searches should be <10ms.

To check performance:

```bash
blz "query" --debug
```

### Source update fails

```bash
# Check network connectivity
curl -I https://bun.sh/llms.txt

# Force re-fetch
blz remove bun && blz add bun https://bun.sh/llms.txt
```

### Index seems corrupted

```bash
# Remove and re-add source (rebuilds index)
blz remove bun
blz add bun https://bun.sh/llms.txt
```

### I want to debug command execution

```bash
# Show detailed timing metrics
blz "query" --debug

# Show memory and CPU usage
blz "query" --profile

# Enable verbose logging
blz "query" --verbose
```

### I want to check cache location

```bash
# macOS
ls ~/Library/Application\ Support/dev.outfitter.blz/

# Linux
ls ~/.local/share/blz/

# Or use custom location
export BLZ_DATA_DIR=~/my-blz-cache
blz list
```

---

## See Also

- [Quick Start](../QUICKSTART.md) - First-time setup
- [CLI Overview](README.md) - CLI installation and index
- [Commands Reference](commands.md) - Complete command documentation
- [Search Guide](search.md) - Advanced search techniques
- [Configuration](configuration.md) - Customization options
- [Sources Guide](sources.md) - Managing documentation sources
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
