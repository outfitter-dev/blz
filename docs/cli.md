# blz CLI Reference

Complete command-line interface reference for `blz`.

For enhanced productivity with tab completion and shell integration, see the [Shell Integration Guide](shell-integration.md).

## Global Options

```
  -h, --help      Print help
  -V, --version   Print version
      --verbose   Enable verbose output
      --debug     Show detailed performance metrics
      --profile   Show resource usage (memory, CPU)
      --flamegraph Generate CPU flamegraph (requires flamegraph feature)
```

## Commands Overview

| Command | Alias | Description |
|---------|-------|-------------|
| `add` | | Add a new llms.txt source |
| `lookup` | | Search registries for documentation to add |
| `search` | | Search across indexed documentation |
| `get` | | Get exact lines from a source |
| `list` | `sources` | List all indexed sources |
| `update` | | Update indexed sources |
| `remove` | `rm`, `delete` | Remove an indexed source |
| `diff` | | View changes in sources |
| `completions` | | Generate shell completions |

## Command Reference

### `blz add`

Add a new llms.txt source to your local cache.

```bash
blz add <ALIAS> <URL> [OPTIONS]
```

**Arguments:**

- `<ALIAS>` - Short name to reference this source
- `<URL>` - URL to the llms.txt file

**Options:**

- `-y, --yes` - Auto-select the best flavor without prompts

**Examples:**

```bash
# Add Bun documentation
blz add bun https://bun.sh/llms.txt

# Add with auto-confirmation
blz add node https://nodejs.org/llms.txt --yes
```

### `blz lookup`

Search registries for available documentation sources.

```bash
blz lookup <QUERY>
```

**Arguments:**

- `<QUERY>` - Search term (tool name, partial name, etc.)

**Examples:**

```bash
# Find TypeScript-related documentation
blz lookup typescript

# Search for web frameworks
blz lookup react
```

### `blz search`

Search across all indexed documentation sources.

```bash
blz search <QUERY> [OPTIONS]
```

**Arguments:**

- `<QUERY>` - Search terms

**Options:**

- `--alias <ALIAS>` - Filter results to specific source
- `-n, --limit <N>` - Maximum results to show (default: 50)
- `--all` - Show all results (no limit)
- `--page <N>` - Page number for pagination (default: 1)
- `--top <N>` - Show only top N percentile of results (1-100)
- `-o, --output <FORMAT>` - Output format: `text` (default) or `json`

**Examples:**

```bash
# Basic search
blz search "test runner"

# Search only in Bun docs
blz search "bundler" --alias bun

# Get more results
blz search "performance" --limit 100

# JSON output for scripting
blz search "async" --output json

# Top 10% of results only
blz search "database" --top 10
```

### `blz get`

Retrieve exact line ranges from an indexed source.

```bash
blz get <ALIAS> --lines <RANGE> [OPTIONS]
```

**Arguments:**

- `<ALIAS>` - Source alias to read from

**Options:**

- `-l, --lines <RANGE>` - Line range(s) to retrieve
- `-c, --context <N>` - Include N context lines around each range

**Line Range Formats:**

- Single line: `42`
- Range: `120-142`
- Multiple ranges: `36:43,320:350`
- Relative: `36+20` (36 plus next 20 lines)

**Examples:**

```bash
# Get lines 120-142 from Bun docs
blz get bun --lines 120-142

# Get multiple ranges
blz get node --lines "10:20,50:60"

# Include 3 lines of context
blz get deno --lines 100-110 --context 3
```

### `blz list` / `blz sources`

List all indexed documentation sources.

```bash
blz list [OPTIONS]
```

**Options:**

- `-o, --output <FORMAT>` - Output format: `text` (default) or `json`

**Examples:**

```bash
# List all sources
blz list

# JSON output for scripting
blz list --output json
```

### `blz update`

Update indexed sources with latest content.

```bash
blz update [ALIAS] [OPTIONS]
```

**Arguments:**

- `[ALIAS]` - Specific source to update (optional)

**Options:**

- `--all` - Update all sources

**Examples:**

```bash
# Update specific source
blz update bun

# Update all sources
blz update --all
```

### `blz remove` / `blz rm` / `blz delete`

Remove an indexed source.

```bash
blz remove <ALIAS>
```

**Arguments:**

- `<ALIAS>` - Source alias to remove

**Examples:**

```bash
# Remove Bun documentation
blz remove bun

# Alternative commands (same effect)
blz rm bun
blz delete bun
```

### `blz diff`

View changes in indexed sources.

```bash
blz diff <ALIAS> [OPTIONS]
```

**Arguments:**

- `<ALIAS>` - Source alias to check

**Options:**

- `--since <TIMESTAMP>` - Show changes since specific time

**Examples:**

```bash
# View changes in Bun docs
blz diff bun

# Changes since specific date
blz diff node --since "2025-08-20"
```

### `blz completions`

Generate shell completion scripts.

```bash
blz completions <SHELL>
```

**Arguments:**

- `<SHELL>` - Target shell: `bash`, `zsh`, `fish`, or `powershell`

**Examples:**

```bash
# Generate Fish completions
blz completions fish > ~/.config/fish/completions/blz.fish

# Generate Bash completions
blz completions bash > ~/.local/share/bash-completion/completions/blz

# Generate Zsh completions
blz completions zsh > ~/.zsh/completions/_blz
```

## Default Behavior

When you run `blz` without a subcommand, it acts as a search:

```bash
# These are equivalent
blz "test runner"
blz search "test runner"
```

## Output Formats

### Text Format (Default)

Human-readable output with colors and formatting:

```
Search results for 'test runner':

1. bun (score: 4.09)
   Path: Bun Documentation > Guides > Test runner
   Lines: L304-324
   Snippet: ### Guides: Test runner...
```

### JSON Format

Machine-readable JSON for scripting and integration:

```json
{
  "hits": [
    {
      "alias": "bun",
      "lines": "304-324",
      "score": 4.09,
      "heading_path": ["Bun Documentation", "Guides", "Test runner"],
      "content": "### Guides: Test runner..."
    }
  ],
  "total": 1,
  "query": "test runner"
}
```

## Performance Profiling

Use global flags to analyze performance:

```bash
# Show detailed timing metrics
blz search "performance" --debug

# Show memory and CPU usage
blz search "bundler" --profile

# Generate CPU flamegraph (requires flamegraph feature)
blz search "complex query" --flamegraph
```

## Exit Codes

- `0` - Success
- `1` - General error
- `2` - Invalid arguments
- `3` - Network/fetch error
- `4` - File system error

## Configuration

`blz` stores data in `~/.outfitter/blz/`:

```
~/.outfitter/blz/
├── sources/          # Cached documentation
│   ├── bun.json
│   └── node.json
├── indices/          # Search indices
│   ├── bun.idx
│   └── node.idx
└── config.json      # Configuration
```

## Tips

1. **Use aliases** - They make commands shorter and searches faster
2. **Combine with shell tools** - `blz search "test" | grep -i jest`
3. **JSON output for scripts** - Easy to parse with `jq` or similar tools
4. **Set up completions** - Tab completion makes the CLI much more productive
5. **Regular updates** - Run `blz update --all` periodically for fresh docs
