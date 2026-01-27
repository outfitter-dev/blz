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
| `query` | | Full-text search across cached documentation |
| `get` | | Retrieve exact lines from a source by citation |
| `map` | `toc` *(deprecated)*, `anchors` *(deprecated)* | Browse documentation structure (headings and sections) |
| `add` | | Add a new llms.txt source |
| `lookup` | | Search registries for documentation to add |
| `list` | `sources` | List all indexed sources |
| `sync` | `refresh` *(deprecated)*, `update` *(deprecated)* | Fetch latest documentation from sources |
| `rm` | `remove`, `delete` | Remove a source and its cached content |
| `info` | | Show detailed information about a source |
| `check` | | Validate source integrity and availability |
| `completions` | | Generate shell completions |
| `docs` | | Bundled documentation hub and CLI reference |
| `alias` | | Manage aliases for a source |
| `--prompt` | | Emit agent-focused JSON guidance for the CLI or specific commands |
| `history` | | Show recent searches and CLI defaults |
| `stats` | | Show cache statistics and overview |
| `doctor` | | Run health checks on cache and sources |
| `find` | `search` *(deprecated)* | *(deprecated)* Unified search/retrieve command |

## Table of Contents

- [Global Options](#global-options)
- [Commands Overview](#commands-overview)
- [Querying Commands](#querying-commands)
  - [blz query](#blz-query)
  - [blz get](#blz-get)
  - [blz map](#blz-map)
- [Source Management Commands](#source-management-commands)
  - [blz add](#blz-add)
  - [blz lookup](#blz-lookup)
  - [blz list](#blz-list--blz-sources)
  - [blz sync](#blz-sync)
  - [blz rm](#blz-rm--blz-remove--blz-delete)
  - [blz info](#blz-info)
  - [blz check](#blz-check)
- [Utility Commands](#utility-commands)
  - [blz completions](#blz-completions)
  - [blz docs](#blz-docs)
  - [blz history](#blz-history)
  - [blz alias](#blz-alias)
  - [blz --prompt](#blz---prompt)
  - [blz stats](#blz-stats)
  - [blz doctor](#blz-doctor)
- [Deprecated Commands](#deprecated-commands)
  - [blz find](#blz-find-deprecated)
  - [blz search](#blz-search-deprecated)
  - [blz toc](#blz-toc-deprecated)
  - [blz refresh](#blz-refresh-deprecated)
- [Output Formats](#output-formats)

---

## Querying Commands

### `blz query`

Full-text search across cached documentation. Use this for text queries; for retrieving specific lines by citation, use `blz get` instead.

```bash
blz query <QUERY>... [OPTIONS]
```

**Arguments:**

- `<QUERY>...` - Search query terms (not citations)

**Query Syntax:**

- `"exact phrase"` - Match exact phrase (use single quotes: `blz query '"exact phrase"'`)
- `+term` - Require term (AND)
- `term1 term2` - Match any term (OR - default)
- `+api +key` - Require both terms

**Options:**

- `-s, --source <SOURCE>` - Filter to specific source(s), comma-separated
- `-n, --limit <N>` - Maximum results per page
- `--all` - Show all results (no limit)
- `--page <N>` - Page number for pagination (default: 1)
- `--top <N>` - Show only top N percentile of results (1-100)
- `-H, --heading-level <FILTER>` - Filter by heading level (e.g., `-H 2,3`, `-H <=2`, `-H 1-3`)
- `--headings-only` - Restrict matches to heading text only
- `-C, --context <N>` - Lines of context around matches
- `--max-chars <CHARS>` - Maximum snippet length (50-1000, default: 200)
- `-f, --format <FORMAT>` - Output format: `text`, `json`, `jsonl`, `raw`
- `--json` - Shorthand for `--format json`
- `--show <COLUMNS>` - Additional columns: `rank`, `url`, `lines`, `anchor`, `raw-score`

**Examples:**

```bash
# Basic search
blz query "test runner"                   # Search all sources
blz query "react hooks"                   # Search for phrase
blz query useEffect cleanup               # Search for terms (OR)
blz query +async +await                   # Require both terms (AND)

# Filter by source
blz query "useEffect" -s react            # Search in specific source
blz query "bundler" -s bun,node           # Search multiple sources

# Filter by heading level
blz query "api" -H 2,3                    # Only h2/h3 headings
blz query "config" -H <=2 --headings-only # Match h1/h2 heading text only

# Output control
blz query "performance" --json            # JSON for scripting
blz query "database" --top 10             # Top 10% of results only
blz query "error handling" -C 3           # With 3 lines context

# Can omit 'query' - it's the default for text queries
blz "test runner"                         # Implicit search
```

> **Note**: The `find` and `search` commands are deprecated. Use `query` for searching and `get` for retrieval.

### `blz get`

Retrieve exact lines from a source by citation. Use for fetching specific line ranges from indexed documentation.

```bash
blz get <ALIAS:LINES>... [OPTIONS]
blz get <ALIAS> --lines <RANGE> [OPTIONS]
```

**Arguments:**

- `<ALIAS:LINES>...` - One or more `alias:start-end` targets (e.g., `bun:120-142`)
- Multiple spans can be comma-separated: `bun:120-142,200-210`
- Multiple sources: `bun:120-142 deno:5-10`

**Line Range Formats:**

- Single line: `42`
- Range: `120-142`
- Multiple ranges: `36-43,320-350`
- Relative: `36+20` (36 plus next 20 lines)

**Options:**

- `-s, --source <SOURCE>` - Explicit source alias (when positional is ambiguous)
- `-l, --lines <RANGE>` - Line range(s) to retrieve (alternative to colon syntax)
- `-C, --context <N>` - Lines of context before and after (or `all` for full section)
- `-A, --after-context <N>` - Lines of context after only
- `-B, --before-context <N>` - Lines of context before only
- `--max-lines <N>` - Cap output when using `--context all`
- `--copy` - Copy output to clipboard using OSC 52
- `-f, --format <FORMAT>` - Output format: `text`, `json`, `jsonl`, `raw`
- `--json` - Shorthand for `--format json`

**Context Flags (grep-style):**

The context flags follow grep/ripgrep conventions and can be combined:

- `-C 5` - 5 lines before and after (symmetric context)
- `-A 3` - 3 lines after only
- `-B 5` - 5 lines before only
- `-A 3 -B 5` - 5 lines before, 3 lines after (asymmetric context)

**Examples:**

```bash
# Basic retrieval (matches search output format)
blz get bun:120-142                       # Single range
blz get bun:120-142 -C 5                  # With 5 lines context
blz get bun:120-142 -C all                # Full section expansion
blz get bun:120-142 -C all --max-lines 80 # Full section, capped

# Multiple spans
blz get bun:120-142,200-210               # Same source, multiple ranges
blz get bun:120-142 deno:5-10             # Multiple sources

# Asymmetric context
blz get bun:120-142 -B 5 -A 3             # 5 before, 3 after

# JSON for scripting
blz get bun:120-142 --json | jq -r '.requests[0].snippet'

# Iterate ranges for multi-span request
blz get bun:120-142,200-210 --json \
  | jq -r '.requests[0].ranges[] | "\(.lineStart)-\(.lineEnd)"'

# Can omit 'get' - it's the default for citation patterns
blz bun:120-142                           # Implicit retrieve
```

**JSON Response (single range):**

```json
{
  "requests": [
    {
      "alias": "bun",
      "source": "bun",
      "snippet": "...",
      "lineStart": 120,
      "lineEnd": 142,
      "checksum": "...",
      "contextApplied": 0
    }
  ],
  "executionTimeMs": 6,
  "totalSources": 1
}
```

**JSON Response (multi-range):**

```json
{
  "requests": [
    {
      "alias": "bun",
      "source": "bun",
      "ranges": [
        { "lineStart": 120, "lineEnd": 142, "snippet": "..." },
        { "lineStart": 200, "lineEnd": 210, "snippet": "..." }
      ],
      "checksum": "..."
    }
  ],
  "executionTimeMs": 9,
  "totalSources": 1
}
```

### `blz map`

Browse documentation structure (headings and sections). Navigate the table of contents for indexed sources.

```bash
blz map [ALIAS] [OPTIONS]
```

**Arguments:**

- `[ALIAS]` - Source alias (optional when using `--source` or `--all`)

**Options:**

- `--filter <EXPR>` - Boolean expression for heading text (AND/OR/NOT supported)
- `--max-depth <1-6>` - Limit to headings at or above this level
- `-H, --heading-level <FILTER>` - Filter by heading level (e.g., `<=2`, `>3`, `1-3`, `1,2,3`)
- `-s, --source <ALIASES>` - Search specific sources (comma-separated)
- `--all` - Include all sources
- `--tree` - Display as hierarchical tree with box-drawing characters
- `--anchors` - Show anchor metadata and remap history
- `-a, --show-anchors` - Show anchor slugs in normal output
- `-n, --limit <N>` - Headings per page (enables pagination)
- `--page <N>` - Jump to specific page
- `--next`, `--previous`, `--last` - Navigate relative to last paginated view
- `-f, --format <FORMAT>` - Output format: `text`, `json`, `jsonl`
- `--json` - Shorthand for `--format json`

**Examples:**

```bash
# Browse structure
blz map bun                               # Browse bun docs structure
blz map bun --tree                        # Hierarchical tree view
blz map bun --tree -H 1-2                 # Tree with h1/h2 only

# Filter headings
blz map react --filter "API AND NOT deprecated"
blz map astro --max-depth 1               # Top-level headings only
blz map bun -H <=2                        # H1 and H2 only

# Multi-source
blz map --all -H 1-2 --json               # All sources, outline
blz map -s bun,node,deno --tree           # Specific sources

# Pagination
blz map bun --limit 20                    # First 20 headings
blz map bun --limit 20 --page 2           # Second page
blz map bun --next                        # Continue to next page

# Inspect anchors
blz map bun --anchors --json              # Anchor metadata
```

> **Note**: The `toc` and `anchors` commands are deprecated aliases for `map`.

---

## Source Management Commands

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
blz lookup <QUERY> [--json|--jsonl|--text]
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

### `blz list` / `blz sources`

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

### `blz sync`

Fetch latest documentation from sources. Syncs cached documentation with upstream llms.txt files.

> The `blz refresh` and `blz update` commands remain available as deprecated aliases and will emit warnings when used.

```bash
blz sync [ALIAS]... [OPTIONS]
```

**Arguments:**

- `[ALIAS]...` - Source aliases to sync (syncs all if omitted)

**Options:**

- `--all` - Sync all sources
- `-y, --yes` - Apply changes without prompting (e.g., auto-upgrade to llms-full)
- `--reindex` - Force re-index even if content unchanged

**Examples:**

```bash
# Sync all sources
blz sync

# Sync specific sources
blz sync bun react

# Force re-index
blz sync bun --reindex
```

### `blz rm` / `blz remove` / `blz delete`

Remove a source and its cached content.

```bash
blz rm <ALIAS> [OPTIONS]
```

By default BLZ prompts before deleting a source. Supply `--yes` in headless or scripted workflows.

**Arguments:**

- `<ALIAS>` - Source to remove (canonical or metadata alias)

**Options:**

- `-y, --yes` - Skip confirmation prompt

**Examples:**

```bash
# Remove Bun documentation
blz rm bun

# Remove without confirmation (for scripts)
blz rm bun --yes

# Alternative commands (same effect)
blz remove bun
blz delete bun
```

### `blz info`

Show detailed information about a source.

```bash
blz info <ALIAS> [OPTIONS]
```

**Arguments:**

- `<ALIAS>` - Source alias to inspect

**Options:**

- `-f, --format <FORMAT>` - Output format: `text`, `json`, `jsonl`
- `--json` - Shorthand for `--format json`

**Examples:**

```bash
# Show source details
blz info bun

# JSON for scripting
blz info bun --json
```

### `blz check`

Validate source integrity and availability.

```bash
blz check [ALIAS]... [OPTIONS]
```

**Arguments:**

- `[ALIAS]...` - Sources to check (checks all if omitted)

**Options:**

- `--all` - Check all sources
- `-f, --format <FORMAT>` - Output format: `text`, `json`, `jsonl`

**Examples:**

```bash
# Check all sources
blz check --all

# Check specific source
blz check bun

# JSON output for CI
blz check --all --json
```

## Utility Commands

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

Bundled documentation hub with subcommands for embedded documentation.

```bash
blz docs <subcommand>
```

**Subcommands:**

- `search <query>` – Search the bundled blz-docs source
- `sync` – Sync or resync embedded documentation files and index
- `overview` – Display quick-start guide
- `cat` – Print entire bundled llms-full.txt to stdout
- `export` – Export CLI docs in markdown or JSON

**Examples:**

```bash
# Search bundled docs (stays scoped to internal docs)
blz docs search "context flags"

# Sync embedded docs after upgrade
blz docs sync

# Export CLI reference as JSON (for agents/tooling)
blz docs export --json | jq '.subcommands[] | {name, usage}'

# Export as markdown (default)
blz docs export > BLZ-CLI.md

# Legacy syntax (still works)
blz docs --format json  # Equivalent to: blz docs export --json
```

**Notes:**

- The `blz-docs` alias (also `@blz`) is internal and hidden from default search
- Use `blz docs search` to query this source specifically
- Legacy `blz docs --format <FORMAT>` is mapped to `blz docs export --format <FORMAT>`

### `blz stats`

Show cache statistics and overview.

```bash
blz stats [OPTIONS]
```

**Options:**

- `-f, --format <FORMAT>` - Output format: `text`, `json`, `jsonl`
- `--json` - Shorthand for `--format json`

**Examples:**

```bash
# Show cache overview
blz stats

# JSON for scripting
blz stats --json
```

### `blz doctor`

Run health checks on cache and sources.

```bash
blz doctor [OPTIONS]
```

**Options:**

- `-f, --format <FORMAT>` - Output format: `text`, `json`, `jsonl`
- `--fix` - Attempt to fix detected issues

**Examples:**

```bash
# Run health checks
blz doctor

# Attempt auto-fixes
blz doctor --fix
```

## Default Behavior

When you run `blz` without a subcommand, it automatically detects the mode:

- **Text queries** run as `blz query` (search)
- **Citation patterns** (e.g., `alias:123-456`) run as `blz get` (retrieve)

```bash
# These are equivalent
blz "test runner"                         # Implicit search
blz query "test runner"                   # Explicit search

blz bun:120-142                           # Implicit retrieve
blz get bun:120-142                       # Explicit retrieve

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
5. **Regular syncs** - Run `blz sync --all` periodically for fresh docs

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

---

## Deprecated Commands

The following commands are deprecated and will be removed in a future release. They remain available for backward compatibility but emit deprecation warnings.

### `blz find` *(deprecated)*

> **Deprecated**: Use `blz query` for searches and `blz get` for retrievals instead.

The unified `find` command that auto-detected search vs retrieve mode based on input pattern. Now split into explicit `query` and `get` commands for clarity.

```bash
# Old (deprecated)
blz find "test runner"                    # Search mode
blz find bun:120-142                      # Retrieve mode

# New (preferred)
blz query "test runner"                   # Explicit search
blz get bun:120-142                       # Explicit retrieve
```

### `blz search` *(deprecated)*

> **Deprecated**: Use `blz query` instead.

Legacy search command. Replaced by `blz query` with improved query syntax and options.

```bash
# Old (deprecated)
blz search "test runner"

# New (preferred)
blz query "test runner"
```

### `blz toc` *(deprecated)*

> **Deprecated**: Use `blz map` instead.

Legacy table of contents command. Replaced by `blz map` with improved tree visualization and filtering.

```bash
# Old (deprecated)
blz toc bun --limit 20

# New (preferred)
blz map bun --limit 20
```

### `blz refresh` *(deprecated)*

> **Deprecated**: Use `blz sync` instead.

Legacy refresh command. Replaced by `blz sync` with improved options.

```bash
# Old (deprecated)
blz refresh bun
blz refresh --all

# New (preferred)
blz sync bun
blz sync --all
```

### `blz update` *(deprecated)*

> **Deprecated**: Use `blz sync` instead.

Legacy alias for `refresh`, now both replaced by `blz sync`.

### `blz anchors` *(deprecated)*

> **Deprecated**: Use `blz map --anchors` instead.

Legacy command for viewing anchor metadata. Now available as a flag on `blz map`.
