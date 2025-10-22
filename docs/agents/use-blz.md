---
title: Using the `blz` CLI to search llms.txt docs
description: Use the `blz` CLI tool to search and retrieve documentation from locally cached llms.txt files in milliseconds.
---


Use the [`blz`](https://github.com/outfitter-dev/blz) CLI to keep documentation local, search it in milliseconds (P50 ≈ 6ms), and return grounded spans with exact citations. Keep payloads lean so agents spend fewer tokens per query compared to traditional page-level search.

The `blz find` command unifies search and retrieval into a single interface that automatically dispatches based on input pattern. Use `find` for all workflows.

## Instructions

1. Verify the CLI: `blz --version`
2. Pull guidance on demand: `blz --prompt` or `blz --prompt <command>`
3. Inspect indexed sources: `blz list --json`
4. Prefer `--json` / `--jsonl` outputs so tooling can parse them cleanly
5. Use `blz find` for both search (text query) and retrieval (citation). Pass `alias:lines` citations directly or use search terms
6. Use `--context all` (plus `--max-lines <N>` if needed) for whole sections, or `-C <N>` to add a small context window
7. Pair `--headings-only` with full-heading queries (optionally prefixed with `#`) to target section titles explicitly before expanding with `--context all`
8. Expect tight payloads: the standard `find` flow typically returns 20–80 lines instead of multi-kilobyte pages, keeping token usage low
9. Set a default format when you want every command in JSON: `export BLZ_OUTPUT_FORMAT=json`
10. Check source health with `blz info <alias> --json` (or `blz list --status --json`) before a heavy retrieval session

### Choosing llms.txt vs llms-full.txt

- `llms-full.txt` is usually the safest choice—full docs, no surprises, best search quality. Use it if it exists.
- Don’t skip plain `llms.txt` if there’s no `-full` variant. Many projects publish the complete content under that filename, or use it as an index that links to heavier `llms.txt` files (add those linked files too).
- When hunting for new sources:
  - Use searches like `"llms-full.txt" site:example.com` or `("llms.txt" OR "llms-full.txt") <product> docs`
  - Inspect each candidate to ensure it’s text-based and substantial (thousands of lines). Indexed aggregator pages still count if they link to the real docs you can add individually.

## Setup & Sources

```bash
# Add Bun docs (non-interactive)
blz add bun https://bun.sh/llms.txt -y

# Confirm what you have indexed
blz list --json
```

## Unified Find Command

The `find` command automatically dispatches to search or retrieve mode based on input pattern:

- **Search mode**: Input is a text query (e.g., `"test runner"`)
- **Retrieve mode**: Input matches citation pattern (e.g., `bun:123-456`)

```bash
# Search mode: text queries find relevant sections
blz find "test runner" --json

# Retrieve mode: citations fetch exact line ranges
blz find bun:41994-42009 -C 5 --json

# Filter by heading level (1-6) to target specific depths
blz find "authentication" -H 2 --json       # Only h2 headings
blz find "API reference" -H 1,2 --json      # h1 and h2 headings

# Combine heading filter with full section retrieval
blz find "error handling" -H 3 --headings-only --context all --json

# Mix multiple citations from different sources
blz find bun:41994-42009 turbo:2656-2729 -C 2 --json
```

**Deprecated commands**: The `search` and `get` commands are deprecated and will be removed in a future release. Use `find` instead:
- `blz search <query>` → use `blz find <query>`
- `blz get <citation>` → use `blz find <citation>`

## Search

```bash
# Ranked matches with minimal effort
blz find "test runner" --json

# Narrow the scope
blz find "test reporters" --json

# Control snippet length (default: 200 chars, range: 50-1000)
blz find "api documentation" --max-chars 300 --json  # Longer snippets for better context
blz find "quick reference" --max-chars 100 --json     # Shorter snippets to save tokens

# Filter by heading level to target specific sections
blz find "configuration" -H 1 --json           # Only top-level headings
blz find "API methods" -H 2,3 --json           # h2 and h3 headings only
blz find "#Advanced Usage" -H 2 --json         # Exact heading at level 2

# Combine heading-only mode with section expansion
blz find "Skip tests with the Bun test runner" --headings-only --context all --max-lines 120 --json
```

### Tuning Snippet Length

The `--max-chars` flag controls the total character count of snippets returned in search results, including newlines and all text:

- **Default**: 200 characters provides good balance between context and token efficiency
- **Range**: 50-1000 characters (values outside this range are automatically clamped)
- **Environment**: Set `BLZ_MAX_CHARS` to change the default for all searches
- **Use cases**:
  - **50-100 chars**: Minimal snippets when you just need to identify relevant sections
  - **200 chars** (default): Good balance for assessing relevance without fetching full content
  - **300-500 chars**: More context for complex topics or when you need better relevance signals
  - **500-1000 chars**: Maximum context before fetching full sections with `blz find`

Example workflow:

```bash
# Quick scan with short snippets
blz "error handling" --max-chars 100 --json | jq -r '.results[0:3] | .[] | .alias + ":" + .lines'

# Detailed assessment with longer snippets
blz "authentication flow" --max-chars 400 --json | jq '.results[0] | {heading: .headingPath, snippet}'
```

## Retrieve Content

```bash
# Copy alias:lines from search output and fetch the span with context
blz find bun:41994-42009 -C 5 --json

# Combine multiple spans into one payload
blz find bun:41994-42009,42010-42020 -C 2 --json
#   └─ JSON replies with `requests[0].ranges[]`; aggregate snippets manually

# Mix sources in one call
blz find bun:41994-42009,42010-42020 turbo:2656-2729 -C 2 --json

# Expand to the whole section (and cap the output to 80 lines)
blz find bun:41994-42009 --context all --max-lines 80 --json

# Seamless workflow: search then retrieve with same command
span=$(blz find "test runner" --json | jq -r '.results[0] | "\(.alias):\(.lines)"')
blz find "$span" -C 10 --json

# Note: --block is a legacy alias for --context all
# Note: -c<N> is legacy syntax; prefer -C <N> (grep-style)
```

## Table of Contents Navigation

```bash
# Browse all headings
blz toc bun --json

# Filter by heading level - exact match
blz toc bun -H 1              # Only H1s
blz toc bun -H 2              # Only H2s

# Filter by level - range
blz toc bun -H 1-2            # H1 and H2 (outline mode)
blz toc bun -H 2-4            # H2 through H4

# Filter by level - comparison operators
blz toc bun -H <=2            # Levels 1-2 (alternative to 1-2)
blz toc bun -H >2             # Levels 3+ (details only)

# Convenient shorthand for max depth
blz toc bun --max-depth 2     # Same as -H <=2

# Filter by level - list
blz toc bun -H 1,3,5          # Specific levels only

# Tree view with hierarchical visualization
blz toc bun --tree

# Tree view with level filter
blz toc bun --tree -H 1-3

# Combine level and text filters
blz toc bun -H 1-2 --filter "api"

# Multi-source TOC (source argument is optional)
blz toc --source bun,node,deno --tree -H 1-2

# All sources (when --all is used, the source argument is omitted)
blz toc --all -H 1-2 --json
```

**Note**: `--max-depth N` is a convenient alias for `-H <=N`, making it easier to limit the depth of headings shown.

**Multi-source usage**: When using `--source` or `--all`, the source alias argument is optional. If not provided, the default is all sources (equivalent to `--all`).

**JSON output**: JSON responses include both `alias` and `source` fields for each entry, making it easy to identify which source a heading comes from when working with multiple sources.

## Keep Sources Fresh

```bash
blz refresh bun --json       # Refresh a single source
blz refresh --all --json     # Refresh everything
```

> `blz update` still works as a deprecated alias and will emit a warning.

## Common JSON Pipelines

```bash
# Pull the first alias:lines citation
span=$(blz find "test runner" --json | jq -r '.results[0] | "\(.alias):\(.lines)"')

# Fetch the content straight into your pipeline (single command does both!)
blz find "$span" --json | jq -r '.requests[0].snippet'

# Complete workflow: search → extract citation → retrieve → format
blz find "error handling" --json \
  | jq -r '.results[0] | "\(.alias):\(.lines)"' \
  | xargs -I {} blz find {} -C 5 --json \
  | jq -r '.requests[0].snippet'

# Multi-range helper: join snippets when `ranges[]` is present
blz find bun:41994-42009,42010-42020 -C 2 --json \
  | jq -r '.requests[0] | .snippet // ((.ranges // []) | map(.snippet) | join("\n\n"))'

# Filter to high-confidence matches
blz find "test reporters" --json | jq '[.results[] | select(.score >= 60)]'

# Heading-level filtering for better precision
blz find "configuration options" -H 2,3 --json | jq '.results[0:5]'
```

## Exit Codes

- `0` – Success
- `1` – Command ran but detected a problem (missing source, no matches, etc.)
- `2` – Clap/usage error (bad args)
- `124` – Parent guard timeout (usually CI / harness)
- `129` – Parent process disappeared; orphan guard shut down `blz`
