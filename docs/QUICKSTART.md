# Getting Started with BLZ

This guide will walk you through installing and using `blz` for the first time.

## Installation

### Quick Install (Recommended)

**No prerequisites needed** - the install script handles everything:

```bash
curl -fsSL https://blz.run/install.sh | sh
```

The script installs the latest release to `~/.local/bin`. Override the location with `BLZ_INSTALL_DIR=/path`, or pin a version using `BLZ_VERSION=v0.4.1`.

### From Source (Optional)

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

### From GitHub (Optional)

```bash
# Direct install from GitHub
cargo install --git https://github.com/outfitter-dev/blz blz-cli
```

## First Steps

### 1. Add Your First Source

Choose a documentation set you use often:

```bash
# Option 1: Bun (if you use Bun)
blz add bun https://bun.sh/llms.txt

# Option 2: Discover sources from the registry
blz lookup react

# Option 3: Add directly from a known URL
blz add nextjs https://nextjs.org/llms-full.txt
```

💡 **Tip**: Start with docs you reference daily for maximum impact.

Let's use Bun as an example:

```bash
blz add bun https://bun.sh/llms.txt
```

**What happens:**
- Fetches the llms.txt file from Bun's website
- Parses it into structured heading blocks
- Builds a search index
- Stores everything locally

**You'll see** (~1s):

```
✓ Added bun (1,926 headings, 43,150 lines) in 890ms
```

### 2. Search Your Indexed Docs

Now search for something:

```bash
blz "test runner" -s bun
```

**You'll see** (~6ms):

```
Search results for 'test runner' (6ms):

1. bun:304-324 (score: 92%)
   📍 Bun Documentation > Guides > Test runner
   Lines: 304-324

   ### Test runner
   Bun includes a fast built-in test runner with built-in code
   coverage, snapshot testing, and mocking...
```

### 3. Get Exact Content

Retrieve specific line ranges:

```bash
blz get bun:304-324
```

**You'll see:**

```
Source: bun
Lines: 304-324
Path: Bun Documentation > Guides > Test runner

  304 │ ### Test runner
  305 │
  306 │ Bun includes a fast built-in test runner with built-in code
  307 │ coverage, snapshot testing, and mocking...
  ...
```

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
blz history -n5
```


## Common Use Cases

### Caching Multiple Sources

```bash
# Add Node.js docs (if available)
blz add node https://nodejs.org/llms.txt

# Add Deno docs
blz add deno https://deno.land/llms.txt

# Search across all sources
blz "http server"
```

### Searching with Filters

```bash
# Search only in Bun docs
blz "test" -s bun -n5

# Get more results
blz "performance" -n20

# JSON output for scripts
blz "bundler" --json
```

### Integration with Scripts

```bash
#!/bin/bash
# Find and display TypeScript information

result=$(blz "typescript" --json | jq -r '.results[0]')
alias=$(echo "$result" | jq -r '.alias')
lines=$(echo "$result" | jq -r '.lines')

echo "Found in $alias at lines $lines"
blz get "$alias:$lines"
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
2. **Limit results** - Use `-n` to get results quicker
3. **Cache locally** - Sources are stored in `~/.outfitter/blz/`

## Troubleshooting

### "Command not found: blz"

Make sure the install script completed successfully. Add the install location to your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Or if you installed with Cargo:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### "No sources found"

Make sure you've added at least one source:

```bash
blz list  # Check current sources
blz add bun https://bun.sh/llms.txt  # Add a source
```

### "No results found"

1. Check you have sources indexed: `blz list`
2. Verify source content: `blz info <alias>`
3. Try broader search terms
4. Search across all sources (omit `-s` flag)

### "Source already exists"

Use a different alias or update the existing source:

```bash
# Update existing source
blz update bun

# Use a different alias
blz add bun-docs https://bun.sh/llms.txt
```

### Slow first search

The first search after adding a source may take longer as the OS caches the index. Subsequent searches will be much faster (6ms).

For more task-oriented solutions, see the [CLI How-To Guide](cli/howto.md).

## Next Steps

- Read about [managing sources](cli/sources.md) to learn about updates and organization
- Explore [search syntax](cli/search.md) for advanced queries
- Set up [shell integration](cli/shell_integration.md) for better productivity
- Understand the [architecture](architecture/README.md) for deeper knowledge

## Getting Help

- Run `blz --help` for command reference
- Run `blz <command> --help` for specific command help
- File issues at [GitHub](https://github.com/outfitter-dev/blz/issues)
