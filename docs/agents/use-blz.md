---
title: Using the `blz` CLI to search llms.txt docs
description: Use the `blz` CLI tool to search and retrieve documentation from locally cached llms.txt files in milliseconds.
---

Use the [`blz`](https://github.com/outfitter-dev/blz) CLI to keep documentation local, search it in milliseconds (P50 ≈ 6ms), and return grounded spans with exact citations. Keep payloads lean so agents spend fewer tokens per query compared to traditional page-level search.

## TL;DR

```bash
blz add bun https://bun.sh/llms.txt -y              # Add source
blz query "test runner" --json                       # Search
blz get bun:41994-42009 -C 5 --json                 # Retrieve with context
blz query "error handling" -H 2,3 --context all --json  # Filter headings + expand section
blz map bun --tree -H 1-2 --json                    # Browse structure
```

## Instructions

1. **Verify the CLI**: `blz --version`
2. **Use explicit commands**: `blz query` for search, `blz get` for retrieval
3. **Prefer `--json` outputs**: Set `export BLZ_OUTPUT_FORMAT=json` for session-wide JSON output
4. **Choose `llms-full.txt` when available**: Provides complete docs with best search quality; fall back to `llms.txt` if `-full` variant doesn't exist
5. **Control context**: Use `-C <N>` for line context, `--context all` to expand to the containing heading section (falls back to requested lines if no heading), `--max-lines <N>` to cap output
6. **Filter by heading level**: Use `-H 1,2,3` or `-H <=2` to target specific section depths

## Setup & Sources

```bash
# Add source non-interactively
blz add bun https://bun.sh/llms.txt -y

# List indexed sources
blz list --json

# Check source health before heavy retrieval
blz info bun --json
```

## Search & Retrieve

Use `blz query` for text search and `blz get` for citation retrieval:

```bash
# Search mode: text queries find relevant sections
blz query "test runner" --json
blz query "authentication" -H 2 --json               # Filter to h2 headings only
blz query "error handling" -H 3 --headings-only --json  # Match headings, not body text

# Retrieve mode: citations fetch exact line ranges
blz get bun:41994-42009 -C 5 --json                          # Add 5 lines of context
blz get bun:41994-42009 --context all --max-lines 80 --json  # Expand to heading section, capped
# Note: --context all expands to the containing heading section.
# If no heading encompasses the range, returns only the requested lines.

# Mix multiple citations from different sources
blz get bun:41994-42009 turbo:2656-2729 -C 2 --json

# Control snippet length (default: 200 chars, range: 50-1000)
blz query "api documentation" --max-chars 300 --json   # Longer snippets
blz query "quick reference" --max-chars 100 --json     # Shorter snippets to save tokens

# Seamless workflow: search then retrieve
span=$(blz query "test runner" --json | jq -r '.results[0] | "\(.alias):\(.lines)"')
blz get "$span" -C 10 --json
```

**Snippet length tuning**: Use `--max-chars` (50-1000, default 200) to control token usage. Set `BLZ_MAX_CHARS` environment variable to change default.

## TOC Navigation

```bash
# Browse all headings
blz map bun --json

# Filter by heading level
blz map bun -H 1-2 --json              # H1 and H2 (outline mode)
blz map bun --max-depth 2 --json       # Same as -H <=2
blz map bun -H 1,3,5 --json            # Specific levels only

# Tree view with hierarchical visualization
blz map bun --tree -H 1-3 --json

# Multi-source TOC
blz map --source bun,node,deno --tree -H 1-2 --json
blz map --all -H 1-2 --json            # All sources
```

## Source Management

```bash
# Sync sources
blz sync bun --json                 # Single source
blz sync --all --json               # All sources

# When hunting for new sources:
# - Use searches like "llms-full.txt" site:example.com
# - Prefer llms-full.txt (complete docs) over llms.txt when both exist
# - Inspect candidates to ensure substantial text content (thousands of lines)
```

## JSON Pipelines

```bash
# Extract citation from search
span=$(blz query "test runner" --json | jq -r '.results[0] | "\(.alias):\(.lines)"')
blz get "$span" --json | jq -r '.requests[0].snippet'

# Complete workflow: search → extract → retrieve → format
blz query "error handling" --json \
  | jq -r '.results[0] | "\(.alias):\(.lines)"' \
  | xargs -I {} blz get {} -C 5 --json \
  | jq -r '.requests[0].snippet'

# Multi-range helper: join snippets when ranges[] is present
blz get bun:41994-42009,42010-42020 -C 2 --json \
  | jq -r '.requests[0] | .snippet // ((.ranges // []) | map(.snippet) | join("\n\n"))'

# Filter high-confidence matches
blz query "test reporters" --json | jq '[.results[] | select(.score >= 60)]'

# Heading-level filtering for precision
blz query "configuration options" -H 2,3 --json | jq '.results[0:5]'
```

## Exit Codes

- `0` – Success
- `1` – Command ran but detected a problem (missing source, no matches, etc.)
- `2` – Clap/usage error (bad args)
- `124` – Parent guard timeout (usually CI / harness)
- `129` – Parent process disappeared; orphan guard shut down `blz`

## Deprecated

**Legacy commands**: The following are deprecated and will be removed in a future release:
- `blz search <query>` → use `blz query <query>`
- `blz find <input>` → use `blz query` for search or `blz get` for retrieval
- `blz toc` → use `blz map`
- `blz refresh` → use `blz sync`

**Legacy options**:
- `--block` → use `--context all` (expands to containing heading section; returns only requested lines if no heading encompasses the range)
- `-c<N>` → use `-C <N>` (grep-style)
