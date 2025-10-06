# Getting Started with BLZ

This guide will walk you through installing and using `blz` for the first time.

## Installation

### Prerequisites

- Rust 1.75+ and Cargo (install from [rustup.rs](https://rustup.rs))
- Git

### Quick Install (macOS/Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/outfitter-dev/blz/main/install.sh | sh
```

The script installs the latest release to `~/.local/bin`. Override the location with `BLZ_INSTALL_DIR=/path`, or pin a version using `BLZ_VERSION=v0.4.1`.

### Install from Source

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

## First Steps

### 1. Add Your First Source

Let's start by adding Bun's documentation:

```bash
blz add bun https://bun.sh/llms.txt
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

### 2. Search Your Indexed Docs

Now search for something:

```bash
blz search "test" --source bun
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
blz get bun --lines 304-324
```

This shows the exact content from those lines with line numbers.

### 4. View Your Cache

See all indexed documentation:

```bash
blz list
```

Get stats about your cache:

```bash
blz stats
```

Get info about a source:

```bash
blz info bun
```

### 5. View Your Search History

Inspect recent searches and persisted defaults:

```bash
blz history --limit 5
```

### 6. Batch Operations

You can add multiple sources at once using a manifest file. Create a `manifest.toml` file like this:

```toml
[[source]]
alias = "react"
url = "https://react.dev/llms-full.txt"

[[source]]
alias = "typescript"
url = "https://www.typescriptlang.org/docs/handbook/llms-full.txt"
```

Then, add the sources using the `blz add --manifest` command:

```bash
blz add --manifest manifest.toml
```

## Common Use Cases

### Caching Multiple Sources

```bash
# Add Node.js docs (if available)
blz add node https://nodejs.org/llms.txt

# Add Deno docs
blz add deno https://deno.land/llms.txt

# Search across all sources
blz search "http server"
```

### Searching with Filters

```bash
# Search only in Bun docs
blz search "test" --source bun --limit 5

# Get more results
blz search "performance" --limit 20

# JSON output for scripts
blz search "bundler" --format json
```

### Integration with Scripts

```bash
#!/bin/bash
# Find and display TypeScript information

result=$(blz search "typescript" --format json | jq -r '.results[0]')
alias=$(echo "$result" | jq -r '.alias')
lines=$(echo "$result" | jq -r '.lines')

echo "Found in $alias at lines $lines"
blz get "$alias" --lines "$lines"
```

## Shell Completion

Enable tab completion for your shell:

### Fish

```fish
blz completions fish > ~/.config/fish/completions/blz.fish
```

### Bash

```bash
blz completions bash > ~/.local/share/bash-completion/completions/blz
```

### Zsh

```zsh
blz completions zsh > ~/.zsh/completions/_blz
```

After installation, you can use TAB to complete commands and options:

```bash
blz sea<TAB>        # Completes to: blz search
blz search --so<TAB> # Completes to: blz search --source
blz get <TAB>        # Shows available aliases
```

## Performance Tips

1. **Use aliases** - Searching within a specific source is faster
2. **Limit results** - Use `--limit` to get results quicker
3. **Cache locally** - Sources are stored in `~/.outfitter/blz/`

## Troubleshooting

### Command not found
Add `~/.cargo/bin` to your PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### No sources found
Make sure you've added at least one source:

```bash
blz add bun https://bun.sh/llms.txt
```

### Slow first search
The first search after adding a source may take longer as the OS caches the index. Subsequent searches will be much faster (6ms).

## Next Steps

- Read about [Managing Sources](sources.md) to learn about updates and organization
- Explore [Search Syntax](search.md) for advanced queries
- Set up [Shell Integration](shell-integration/README.md) for better productivity
- Understand the [Architecture](architecture.md) for deeper knowledge

## Getting Help

- Run `blz --help` for command reference
- Run `blz <command> --help` for specific command help
- File issues at [GitHub](https://github.com/outfitter-dev/blz/issues)
