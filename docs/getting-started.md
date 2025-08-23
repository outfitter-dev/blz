# Getting Started with @outfitter/cache

This guide will walk you through installing and using @outfitter/cache for the first time.

## Installation

### Prerequisites
- Rust 1.75+ and Cargo (install from [rustup.rs](https://rustup.rs))
- Git

### Install from Source

```bash
# Clone the repository
git clone https://github.com/outfitter-dev/cache
cd cache

# Install the binary
cargo install --path crates/cache-cli

# Verify installation
cache --help
```

### Install from GitHub

```bash
# Direct install from GitHub
cargo install --git https://github.com/outfitter-dev/cache cache-cli
```

## First Steps

### 1. Add Your First Source

Let's start by caching Bun's documentation:

```bash
cache add bun https://bun.sh/llms.txt
```

This command:
- Fetches the llms.txt file from Bun's website
- Parses it into structured heading blocks
- Builds a search index
- Stores everything locally

Expected output:
```
✓ Added bun (26 headings, 364 lines)
```

### 2. Search Your Cached Docs

Now search for something:

```bash
cache search "test" --alias bun
```

You'll see results in ~6ms:
```
Search results for 'test':

1. bun (score: 4.09)
   Path: Bun Documentation > Guides > Test runner
   Lines: L304-324
   Snippet: ### Guides: Test runner...
```

### 3. Get Exact Content

Retrieve specific line ranges:

```bash
cache get bun --lines 304-324
```

This shows the exact content from those lines with line numbers.

### 4. List Your Sources

See all cached documentation:

```bash
cache sources
```

Output:
```
Cached sources:

  bun https://bun.sh/llms.txt
    Fetched: 2025-08-23 00:55:33
    Lines: 364
```

## Common Use Cases

### Caching Multiple Sources

```bash
# Add Node.js docs (if available)
cache add node https://nodejs.org/llms.txt

# Add Deno docs
cache add deno https://deno.land/llms.txt

# Search across all sources
cache search "http server"
```

### Searching with Filters

```bash
# Search only in Bun docs
cache search "test" --alias bun --limit 5

# Get more results
cache search "performance" --limit 20

# JSON output for scripts
cache search "bundler" --format json
```

### Integration with Scripts

```bash
#!/bin/bash
# Find and display TypeScript information

result=$(cache search "typescript" --format json | jq -r '.hits[0]')
alias=$(echo "$result" | jq -r '.alias')
lines=$(echo "$result" | jq -r '.lines')

echo "Found in $alias at lines $lines"
cache get "$alias" --lines "$lines"
```

## Shell Completion

Enable tab completion for your shell:

### Fish
```fish
cache completions fish > ~/.config/fish/completions/cache.fish
```

### Bash
```bash
cache completions bash > ~/.local/share/bash-completion/completions/cache
```

### Zsh
```zsh
cache completions zsh > ~/.zsh/completions/_cache
```

After installation, you can use TAB to complete commands and options:
```bash
cache sea<TAB>        # Completes to: cache search
cache search --al<TAB> # Completes to: cache search --alias
cache get <TAB>        # Shows available aliases
```

## Performance Tips

1. **Use aliases** - Searching within a specific source is faster
2. **Limit results** - Use `--limit` to get results quicker
3. **Cache locally** - Sources are stored in `~/.local/share/outfitter.cache/`

## Troubleshooting

### Command not found
Add `~/.cargo/bin` to your PATH:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### No sources found
Make sure you've added at least one source:
```bash
cache add bun https://bun.sh/llms.txt
```

### Slow first search
The first search after adding a source may take longer as the OS caches the index. Subsequent searches will be much faster (6ms).

## Next Steps

- Read about [Managing Sources](sources.md) to learn about updates and organization
- Explore [Search Syntax](search.md) for advanced queries
- Set up [Shell Integration](shell-integration.md) for better productivity
- Understand the [Architecture](architecture.md) for deeper knowledge

## Getting Help

- Run `cache --help` for command reference
- Run `cache <command> --help` for specific command help
- File issues at [GitHub](https://github.com/outfitter-dev/cache/issues)