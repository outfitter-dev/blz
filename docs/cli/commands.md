# BLZ Command Reference

Complete reference for all BLZ CLI commands.

For shell integration, see [Shell Integration](shell_integration.md). For task-oriented guides, see [How-To](howto.md).

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
| `upgrade` | | Upgrade sources from llms.txt to llms-full.txt |
| `remove` | `rm`, `delete` | Remove an indexed source |
| `diff` | | View changes in sources (hidden/experimental) |
| `completions` | | Generate shell completions |
| `docs` | | Generate CLI docs (Markdown/JSON) |
| `alias` | | Manage aliases for a source |
| `--prompt` | | Emit agent-focused JSON guidance for the CLI or specific commands |
| `history` | | Show recent searches and CLI defaults |
| `config` | | Manage configuration (global/local/project scopes) |

## Table of Contents

- [Global Options](#global-options)
- [Commands Overview](#commands-overview)
- [Core Commands](#core-commands)
  - [blz add](#blz-add)
  - [blz lookup](#blz-lookup)
  - [blz search](#blz-search)
  - [blz get](#blz-get)
- [Management Commands](#management-commands)
  - [blz list](#blz-list--blz-sources)
  - [blz update](#blz-update)
  - [blz remove](#blz-remove--blz-rm--blz-delete)
- [Utility Commands](#utility-commands)
  - [blz diff](#blz-diff)
  - [blz completions](#blz-completions)
  - [blz docs](#blz-docs)
  - [blz history](#blz-history)
  - [blz config](#blz-config)
  - [blz alias](#blz-alias)
  - [blz --prompt](#blz---prompt)
- [Output Formats](#output-formats)

---

## Core Commands

### `blz add`

Add a new llms.txt source to your local cache.

```bash
blz add <ALIAS> <URL> [OPTIONS]
```

**Arguments:**

- `<ALIAS>` - Short name to reference this source
- `<URL>` - URL to the llms.txt file

**Options:**

- `-y, --yes` - Skip interactive prompts
- `--aliases <ALIAS1,ALIAS2>` - Register additional lookup aliases
- `--dry-run` - Analyze the source and emit JSON without saving files
- `--manifest <FILE>` - Add multiple sources from a TOML manifest (batch mode)
- `--only <ALIAS1,ALIAS2>` - Restrict manifest processing to specific entries
- `--name <NAME>` - Override the display name (defaults to Title Case alias)
- `--description <TEXT>` - Set a description; omitted entries write an empty field
- `--category <CATEGORY>` - Category label (defaults to `uncategorized`)
- `--tags <TAG1,TAG2>` - Attach comma-separated tags for list filtering

When `--manifest` is used the positional `<ALIAS> <URL>` arguments are optional. Each source added (single or batch) writes a descriptor to
`~/.config/blz/sources/<alias>.toml`, capturing the resolved URL/path plus tags and metadata.

**Examples:**

```bash
# Add Bun documentation
blz add bun https://bun.sh/llms.txt

# Add with auto-confirmation
blz add node https://nodejs.org/llms.txt --yes

# Provide metadata inline
blz add react https://react.dev/llms.txt \
  --name "React" \
  --category framework \
  --tags javascript,ui,library

# Import a manifest of sources (remote + local)
blz add --manifest docs/blz.sources.toml

# Dry-run analysis for a manifest (no files written)
blz add --manifest docs/blz.sources.toml --dry-run
```

Minimal manifest example (`docs/blz.sources.toml`):

```toml
version = "1"

[[source]]
alias = "bun"
name = "Bun"
description = "Fast all-in-one JavaScript runtime"
url = "https://bun.sh/llms-full.txt"
category = "runtime"
tags = ["javascript", "runtime"]

  [source.aliases]
  npm = ["bun"]
  github = ["oven-sh/bun"]

[[source]]
alias = "internal-sdk"
name = "Internal SDK"
path = "./docs/internal-sdk.llms.txt"
description = "Private SDK docs"
category = "internal"
tags = ["sdk", "internal"]
```

You can copy this template directly from `registry/templates/batch-manifest.example.toml`.

### `blz lookup`

Search registries for available documentation sources.

```bash
blz lookup <QUERY> [--format text|json|jsonl]
```

> **Beta** · The bundled registry is still small. After each lookup you’ll see a reminder to open a PR with any llms.txt sources we’re missing.

**Arguments:**

- `<QUERY>` - Search term (tool name, partial name, etc.)

**Options:**

- `-f, --format <FORMAT>` - Output format (defaults to `text`; use `BLZ_OUTPUT_FORMAT=json` for agents)

**Examples:**

```bash
# Find TypeScript-related documentation
blz lookup typescript

# Search for web frameworks (JSON for scripting)
blz lookup react --json | jq '.[0]'
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
- `--max-chars <CHARS>` - Limit snippet length (default 200; clamps between 50 and 1000).
  - Environment: `BLZ_MAX_CHARS` adjusts the default for implicit searches.
- `--flavor <MODE>` - Override flavor for this run (`current`, `auto`, `full`, `txt`)
- `-f, --format <FORMAT>` - Output format: `text` (default), `json`, or `jsonl`
  - Environment default: set `BLZ_OUTPUT_FORMAT=json|text|jsonl` to avoid passing `--format` each time (alias `ndjson` still accepted)

> ⚠️ Compatibility: `--output`/`-o` is deprecated starting in v0.3. Use `--format`/`-f`. The alias remains temporarily for compatibility but emits a warning and will be removed in a future release.

**Examples:**

```bash
# Basic search
blz "test runner"

# Search only in Bun docs
blz "bundler" -s bun

# Get more results
blz "performance" -n100

# JSON output for scripting
blz "async" --json

# Top 10% of results only
blz "database" --top 10

# Exact phrase (Unix shells - single quotes around double quotes)
blz '"test runner"'

# Require both phrases
blz '+"test runner" +"cli output"'

# Windows CMD (use backslash escaping)
blz "\"test runner\""
blz "+\"test runner\" +\"cli output\""

# PowerShell (single quotes work as literals)
blz '"test runner"'
blz '+"test runner" +"cli output"'
```

> **Query tips:** Space-separated terms are ORed by default. Prefix them with `+`
> or use `AND` to require all words. Keep phrase searches intact by wrapping the
> phrase in double quotes and surrounding the whole query with single quotes (Unix)
> or escaping with backslashes (Windows CMD).

Aliases and resolution

- Use `--source <SOURCE>` (or `-s`) with either the canonical source or a metadata alias added via `blz alias add`.
- When running `blz QUERY SOURCE` or `blz SOURCE QUERY` without a subcommand, SOURCE may be a canonical name or a metadata alias; the CLI resolves it to the canonical source.

### `blz get`

Retrieve exact line ranges from an indexed source.

```bash
blz get <SOURCE:LINES> [OPTIONS]

# Back-compat form if you prefer flags:
blz get <SOURCE> --lines <RANGE> [OPTIONS]
```

**Arguments:**

- `<SOURCE:LINES>` - Preferred shorthand (matches search output, e.g., `bun:120-142`)
- `<SOURCE>` - Canonical source or metadata alias (use with `--lines`)

**Options:**

- `-l, --lines <RANGE>` – Line range(s) to retrieve (optional when using `source:lines`)
- `-c, --context <N>` – Include N context lines around each range, or use `all` to expand to the entire heading block
- `--context all` – Expand to the entire heading block that contains the first requested range
- `--block` – Legacy alias for `--context all`
- `--max-lines <N>` – Optional hard cap when using `--context all` (prevents oversized spans)
- `-f, --format <FORMAT>` – Output format: `text` (default), `json`, `jsonl`, or `raw`
- `--json`, `--jsonl` – Convenience shorthands for their respective formats
- `--copy` – Copy results to the clipboard via OSC 52 (useful in interactive shells)
- `--prompt` – Emit agent guidance JSON (e.g. `blz get --prompt`)

**Line Range Formats:**

- Single line: `42`
- Range: `120-142`
- Multiple ranges: `36-43,320-350`
- Relative: `36+20` (36 plus next 20 lines)

> ℹ️ When you supply multiple ranges (via `source:lines1,lines2` or `--lines "range1,range2"`), BLZ merges the distinct spans, removes duplicates, and keeps line numbers sorted. Combining this with `--context all` is supported—the heading containing the first range is returned, and `--max-lines` still applies.

**Examples:**

```bash
# Preferred shorthand (matches search output)
blz get bun:41994-42009

# Retrieve multiple spans for the same source
blz get bun --lines "41994-42009,42010-42020" --json

# Expand to the entire heading section (capped at 80 lines)
blz get bun:41994-42009 --context all --max-lines 80 --json

# Include 3 lines of context around the range (text output)
blz get bun:25760-25780 -c3

# Pipe structured output to jq
blz get bun:41994-42009 --json | jq '.content'
```

## Management Commands

### `blz list` / `blz sources`

List all indexed documentation sources.

```bash
blz list [OPTIONS]
```

**Options:**

- `-f, --format <FORMAT>` - Output format: `text` (default), `json`, or `jsonl`
  - Environment default: set `BLZ_OUTPUT_FORMAT=json|text|jsonl`
- `--status` - Include fetch metadata (fetched time, etag, last-modified, checksum)
- `--details` - Show descriptor metadata (description, category, npm/github aliases, origin)

JSON output always includes the descriptor payload (`descriptor` object) in addition to the standard summary fields (`alias`, `url`, `lines`, `headings`, `tags`, `aliases`, `origin`, `sha256`, etc.).

**Examples:**

```bash
# List all sources
blz list

# JSON output for scripting
blz list --json

# Verbose descriptor view
blz list --details
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
blz remove <ALIAS> [--yes]
```

By default BLZ prompts before deleting a source. Supply `--yes` in headless or scripted workflows.

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

## Utility Commands

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
blz docs

# Structured docs for agents / tooling
blz docs --json | jq '.subcommands[] | {name, usage}'

# Pipe docs into a file for offline reference
blz docs > BLZ-CLI.md

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
blz "hooks" | jq -r '.results[] | "\(.alias) \(.lines) \(.headingPath[-1])"'

# Top 10 results with score > 2.0
blz "sqlite" | jq '.results | map(select(.score > 2.0)) | .[:10]'
```

## Performance Profiling

Use global flags to analyze performance:

```bash
# Show detailed timing metrics
blz "performance" --debug

# Show memory and CPU usage
blz "bundler" --profile

# Generate CPU flamegraph (requires flamegraph feature)
blz "complex query" --flamegraph
```

## Exit Codes

- `0` - Success
- `1` - General error
- `2` - Invalid arguments
- `3` - Network/fetch error
- `4` - File system error

## Configuration

BLZ stores data in platform-specific locations:

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

**Note**: If upgrading from an earlier version, BLZ will automatically migrate your data from the old cache directory location.

## Tips

1. **Use aliases** - They make commands shorter and searches faster
2. **Combine with shell tools** - `blz "test" | grep -i jest`
3. **JSON output for scripts** - Easy to parse with `jq` or similar tools
4. **Set up completions** - Tab completion makes the CLI much more productive
5. **Regular updates** - Run `blz update --all` periodically for fresh docs

### `blz --prompt`

Emit JSON guidance for the CLI or a specific command. Replaces the legacy `blz instruct` command.

```bash
blz --prompt            # General overview
blz --prompt search     # Command-specific workflow
blz --prompt alias.add  # Dot notation for nested subcommands
```

The JSON payload is designed for agent consumption (fields include summaries, workflows, recommended flags, and examples).

### `blz history`

Display recent searches and CLI defaults.

```bash
blz history [--limit <N>] [-f text|json|jsonl]
```

**Options:**

- `--limit <N>` – Maximum number of entries to display (default: 20)
- `-f, --format <FORMAT>` – Output format (`text`, `json`, `jsonl`). Honors `BLZ_OUTPUT_FORMAT` when unset.

**Examples:**

```bash
# Show the most recent searches in text mode
blz history -n10

# Inspect history for agents in JSON
blz history --json | jq '.[0]'
```

Text output includes the stored defaults (show components, snippet lines, score precision) followed by the most recent entries (newest first).

### `blz config`

Manage configuration and per-scope preferences. Without subcommands, launches an interactive menu.

```bash
blz config [set|get]

# Non-interactive: set prefer_full globally
blz config set add.prefer_full true

# Override for current directory only
blz config set add.prefer_full false --scope local

# Inspect all scopes
blz config get
```

Scopes behave as follows:

- `global`: writes to the global `config.toml`
- `project`: writes to the project config (current `.blz/config.toml` or directory pointed to by `BLZ_CONFIG_DIR`/`BLZ_CONFIG`)
- `local`: stores overrides in `blz.json`, keyed by the working directory

Use this to quickly onboard agents without external rules files. For a longer guide, see `docs/agents/use-blz.md`.

### Setting a Global Default

Set a single environment variable to control default output across commands that support `--format` (deprecated alias: `-o`/`--output`; JSONL accepts `jsonl` or `ndjson`):

```bash
export BLZ_OUTPUT_FORMAT=json   # or text, jsonl

# Now these default to JSON unless overridden
blz "async"
blz list --status
```

## `blz alias`

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

---

## Output Formats

The BLZ CLI supports multiple output formats to suit different use cases and integrations.

### Available Formats

#### Text (default)

Human-readable colored output optimized for terminal display.

```bash
blz "async rust"
```

#### JSON

Machine-readable JSON output for programmatic consumption.

```bash
# JSON (aggregated with metadata)
blz "async rust" --json

# JSONL (one hit per line)
blz "async rust" --jsonl
```

Output structure (JSON):

```json
{
  "query": "async rust",
  "page": 1,
  "limit": 5,
  "totalResults": 42,
  "totalPages": 9,
  "totalLinesSearched": 123456,
  "searchTimeMs": 6,
  "sources": ["rust", "node"],
  "results": [
    {
      "alias": "rust",
      "file": "llms.txt",
      "headingPath": ["Async", "Futures"],
      "lines": "123-145",
      "lineNumbers": [123, 145],
      "snippet": "...",
      "score": 0.95,
      "sourceUrl": "https://...",
      "checksum": "..."
    }
  ],
  "suggestions": [
    { "alias": "rust", "heading": "Futures", "lines": "200-210", "score": 0.5 }
  ]
}
```

Notes:

- `suggestions` may be included when results are sparse or low-quality to aid discovery
- `jsonl` emits one SearchHit per line (no aggregation metadata)

#### Compact

Minimal output showing only essential information.

```bash
blz "async rust" --format compact
```

Format: `<alias>:<lines> <heading_path>`

### Environment Detection

The CLI automatically detects the output context:

- TTY: Uses colored text output
- Pipe: Uses plain text without colors
- CI: Adjusts formatting for CI environments

### Custom Formatting

Override automatic detection:

```bash
# Force colors even when piping
blz "async rust" --color always

# Disable colors for TTY
blz "async rust" --color never

# Let CLI decide (default)
blz "async rust" --color auto
```

### Environment Variables

Set a global default output format:

```bash
export BLZ_OUTPUT_FORMAT=json   # or text, jsonl

# Now these default to JSON unless overridden
blz "async"
blz list --status
```
