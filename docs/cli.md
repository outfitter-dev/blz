# blz CLI Reference

Complete command-line interface reference for `blz`.

For enhanced productivity with tab completion and shell integration, see the [Shell Integration Guide](shell-integration/README.md).

## Global Options

```
  -h, --help      Print help
  -V, --version   Print version
      --verbose   Enable verbose output
      --debug     Show detailed performance metrics
      --profile   Show resource usage (memory, CPU)
      --config <FILE>  Path to configuration file (overrides autodiscovery)
      --config-dir <DIR>  Directory containing config.toml (overrides autodiscovery)
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
| `diff` | | View changes in sources (hidden/experimental) |
| `completions` | | Generate shell completions |
| `docs` | | Generate CLI docs (Markdown/JSON) |
| `alias` | | Manage aliases for a source |
| `instruct` | | Print instructions for agent use of blz |

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
blz lookup <QUERY> [--output text|json|ndjson]
```

**Arguments:**

- `<QUERY>` - Search term (tool name, partial name, etc.)

**Options:**

- `-o, --output <FORMAT>` - Output format (defaults to `text`; use `BLZ_OUTPUT_FORMAT=json` for agents)

**Examples:**

```bash
# Find TypeScript-related documentation
blz lookup typescript

# Search for web frameworks (JSON for scripting)
blz lookup react -o json | jq '.[0]'
```

### `blz search`

Search across all indexed documentation sources.

```bash
blz search <QUERY> [OPTIONS]
```

**Arguments:**

- `<QUERY>` - Search terms

**Options:**

- `--source <SOURCE>` - Filter results to specific source (also supports `-s`)
- `-n, --limit <N>` - Maximum results to show (default: 50)
- `--all` - Show all results (no limit)
- `--page <N>` - Page number for pagination (default: 1)
- `--top <N>` - Show only top N percentile of results (1-100)
- `-o, --output <FORMAT>` - Output format: `text` (default), `json`, or `ndjson`
  - Environment default: set `BLZ_OUTPUT_FORMAT=json|text|ndjson` to avoid passing `-o` each time

**Examples:**

```bash
# Basic search
blz search "test runner"

# Search only in Bun docs
blz search "bundler" --source bun

# Get more results
blz search "performance" --limit 100

# JSON output for scripting
blz search "async" --output json

# Top 10% of results only
blz search "database" --top 10
```

Aliases and resolution

- Use `--source <SOURCE>` (or `-s`) with either the canonical source or a metadata alias added via `blz alias add`.
- When running `blz QUERY SOURCE` or `blz SOURCE QUERY` without a subcommand, SOURCE may be a canonical name or a metadata alias; the CLI resolves it to the canonical source.

### `blz get`

Retrieve exact line ranges from an indexed source.

```bash
blz get <SOURCE> --lines <RANGE> [OPTIONS]
```

**Arguments:**

- `<SOURCE>` - Canonical source or metadata alias to read from

**Options:**

- `-l, --lines <RANGE>` - Line range(s) to retrieve
- `-c, --context <N>` - Include N context lines around each range
- `-o, --output <FORMAT>` - Output format: `text` (default), `json`, or `ndjson`

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

# JSON output for agents
blz get bun --lines 42-55 -o json | jq '.content'
```

### `blz list` / `blz sources`

List all indexed documentation sources.

```bash
blz list [OPTIONS]
```

**Options:**

- `-o, --output <FORMAT>` - Output format: `text` (default) or `json`
  - Environment default: set `BLZ_OUTPUT_FORMAT=json|text|ndjson`

JSON keys

- Each entry includes: `alias`, `source` (canonical handle), `url`, `fetchedAt`, `lines`, `sha256`
- When available: `etag`, `lastModified`, and `aliases` (array of metadata aliases)

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

- `[SOURCE]` - Specific source to update (canonical or metadata alias; optional)

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

- `<SOURCE>` - Source to remove (canonical or metadata alias)

**Examples:**

```bash
# Remove Bun documentation
blz remove bun

# Alternative commands (same effect)
blz rm bun
blz delete bun
```

### `blz diff` (Hidden/Experimental)

View changes in indexed sources.

**Note**: This command is experimental and hidden from help output. Its output format may change in future releases.

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

- `<SHELL>` - Target shell: `bash`, `zsh`, `fish`, `elvish`, or `powershell`

**Examples:**

```bash
# Generate Fish completions
blz completions fish > ~/.config/fish/completions/blz.fish

# Generate Bash completions
blz completions bash > ~/.local/share/bash-completion/completions/blz

# Generate Zsh completions
blz completions zsh > ~/.zsh/completions/_blz
```

### `blz docs`

Generate CLI documentation directly from the clap definitions.

```bash
blz docs [--format markdown|json]
```

**Options:**

- `--format` - Output format. Defaults to `markdown`. Use `json` for agent/scripting scenarios.
  - Respects global `BLZ_OUTPUT_FORMAT=json` to default to JSON without passing `--format`.

**Examples:**

```bash
# Human-readable CLI docs
blz docs --format markdown

# Structured docs for agents / tooling
blz docs --format json | jq '.subcommands[] | {name, usage}'

# Pipe docs into a file for offline reference
blz docs --format markdown > BLZ-CLI.md

# Use global env var to default to JSON
BLZ_OUTPUT_FORMAT=json blz docs | jq '.name'
```

## Default Behavior

When you run `blz` without a subcommand, it acts as a search:

```bash
# These are equivalent
blz "test runner"
blz search "test runner"

# SOURCE may be canonical or a metadata alias
blz bun "install"
blz "install" @scope/package
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

Machine-readable JSON for scripting and integration. Top-level includes pagination and performance metadata, and results use camelCase keys:

```json
{
  "query": "test runner",
  "page": 1,
  "limit": 50,
  "totalResults": 1,
  "totalPages": 1,
  "totalLinesSearched": 50000,
  "searchTimeMs": 6,
  "sources": ["bun"],
  "results": [
    {
      "alias": "bun",
      "file": "llms.txt",
      "headingPath": ["Bun Documentation", "Guides", "Test runner"],
      "lines": "304-324",
      "snippet": "### Guides: Test runner...",
      "score": 4.09,
      "sourceUrl": "https://bun.sh/llms.txt",
      "checksum": "abc123...",
      "anchor": "bun-guides-test-runner"
    }
  ]
}
```

JSON + jq examples

```bash
# Set JSON as the default output for agents
export BLZ_OUTPUT_FORMAT=json

# List result summaries
blz search "hooks" | jq -r '.results[] | "\(.alias) \(.lines) \(.headingPath[-1])"'

# Top 10 results with score > 2.0
blz search "sqlite" | jq '.results | map(select(.score > 2.0)) | .[:10]'
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

`blz` stores data in platform-specific locations:

### Data Storage

- **macOS**: `~/Library/Application Support/dev.outfitter.blz/`
- **Linux**: `~/.local/share/outfitter/blz/`
- **Windows**: `%APPDATA%\outfitter\blz\data\`

### Configuration

Config discovery order:

- `$XDG_CONFIG_HOME/blz/config.toml` or `~/.config/blz/config.toml`
- Fallback: `~/.blz/config.toml`
- Explicit override: `--config <FILE>` or `--config-dir <DIR>` (uses `<DIR>/config.toml`)
- Optional overlay: `config.local.toml` in the same directory

### Storage Structure

```
<data_directory>/
├── <alias>/          # Per-source data
│   ├── llms.txt     # Original documentation
│   ├── llms.json    # Parsed structure
│   ├── .index/      # Tantivy search index
│   ├── .archive/    # Historical snapshots
│   └── settings.toml # Source-specific config
└── (global config is stored under XDG, not inside the data directory)
```

**Note**: If upgrading from an earlier version, `blz` will automatically migrate your data from the old cache directory location.

## Tips

1. **Use aliases** - They make commands shorter and searches faster
2. **Combine with shell tools** - `blz search "test" | grep -i jest`
3. **JSON output for scripts** - Easy to parse with `jq` or similar tools
4. **Set up completions** - Tab completion makes the CLI much more productive
5. **Regular updates** - Run `blz update --all` periodically for fresh docs
### `blz instruct`

Print instructions for agent use of blz, followed by the current `--help` content so onboarding takes a single command. Examples and flags are kept in sync with the CLI.

```bash
blz instruct
```

Use this to quickly onboard agents without external rules files. For a longer guide, see `.agents/instructions/use-blz.md`.
### Setting a Global Default

Set a single environment variable to control default output across commands that support `-o/--output`:

```bash
export BLZ_OUTPUT_FORMAT=json   # or text, ndjson

# Now these default to JSON unless overridden
blz search "async"
blz list --status
blz anchors react --mappings
blz anchor list react -o json | jq '.[0]'
blz anchor get react <ANCHOR> -o json | jq '.content'
```
# `blz alias`

Manage aliases for a source. Aliases are stored in source metadata and resolved across commands.

```bash
blz alias add <SOURCE> <ALIAS>
blz alias rm <SOURCE> <ALIAS>
```

Examples:

```bash
blz alias add react @facebook/react
blz alias rm react @facebook/react
```

Notes:
- Canonical "source" remains the primary handle; aliases are alternate names.
- Alias formats like `@scope/package` are allowed (not used for directories).
- Ambiguous aliases across multiple sources will produce an error; use the canonical name instead.
