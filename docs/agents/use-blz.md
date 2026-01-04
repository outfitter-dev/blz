---
title: Using the `blz` CLI to search llms.txt docs
description: Use the `blz` CLI tool to search and retrieve documentation from locally cached llms.txt files in milliseconds.
---

Use the [`blz`](https://github.com/outfitter-dev/blz) CLI to keep documentation local, search it in milliseconds (P50 ≈ 6ms), and return grounded spans with exact citations. Keep payloads lean so agents spend fewer tokens per query compared to traditional page-level search.

## TL;DR

```bash
blz add bun https://bun.sh/llms.txt -y              # Add source
blz find "test runner" --json                        # Search
blz find bun:41994-42009 -C 5 --json                # Retrieve with context
blz find "error handling" -H 2,3 --context all --json  # Filter headings + expand section
blz toc bun --tree -H 1-2 --json                    # Browse structure
```

## Instructions

1. **Verify the CLI**: `blz --version`
2. **Use `blz find` for everything**: Automatically dispatches to search (text query) or retrieve (citation pattern like `alias:lines`)
3. **Prefer `--json` outputs**: Set `export BLZ_OUTPUT_FORMAT=json` for session-wide JSON output
4. **Choose `llms-full.txt` when available**: Provides complete docs with best search quality; fall back to `llms.txt` if `-full` variant doesn't exist
5. **Control context**: Use `-C <N>` for line context, `--context all` for full sections, `--max-lines <N>` to cap output
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

The `find` command unifies search and retrieval into a single interface:

```bash
# Search mode: text queries find relevant sections
blz find "test runner" --json
blz find "authentication" -H 2 --json               # Filter to h2 headings only
blz find "error handling" -H 3 --headings-only --json  # Match headings, not body text

# Retrieve mode: citations fetch exact line ranges
blz find bun:41994-42009 -C 5 --json               # Add 5 lines of context
blz find bun:41994-42009 --context all --max-lines 80 --json  # Full section, capped

# Mix multiple citations from different sources
blz find bun:41994-42009 turbo:2656-2729 -C 2 --json

# Control snippet length (default: 200 chars, range: 50-1000)
blz find "api documentation" --max-chars 300 --json   # Longer snippets
blz find "quick reference" --max-chars 100 --json     # Shorter snippets to save tokens

# Seamless workflow: search then retrieve with same command
span=$(blz find "test runner" --json | jq -r '.results[0] | "\(.alias):\(.lines)"')
blz find "$span" -C 10 --json
```

**Snippet length tuning**: Use `--max-chars` (50-1000, default 200) to control token usage. Set `BLZ_MAX_CHARS` environment variable to change default.

## TOC Navigation

```bash
# Browse all headings
blz toc bun --json

# Filter by heading level
blz toc bun -H 1-2 --json              # H1 and H2 (outline mode)
blz toc bun --max-depth 2 --json       # Same as -H <=2
blz toc bun -H 1,3,5 --json            # Specific levels only

# Tree view with hierarchical visualization
blz toc bun --tree -H 1-3 --json

# Multi-source TOC
blz toc --source bun,node,deno --tree -H 1-2 --json
blz toc --all -H 1-2 --json            # All sources
```

## Source Management

```bash
# Refresh sources
blz refresh bun --json                 # Single source
blz refresh --all --json               # All sources

# When hunting for new sources:
# - Use searches like "llms-full.txt" site:example.com
# - Prefer llms-full.txt (complete docs) over llms.txt when both exist
# - Inspect candidates to ensure substantial text content (thousands of lines)
```

## JSON Pipelines

```bash
# Extract citation from search
span=$(blz find "test runner" --json | jq -r '.results[0] | "\(.alias):\(.lines)"')
blz find "$span" --json | jq -r '.requests[0].snippet'

# Complete workflow: search → extract → retrieve → format
blz find "error handling" --json \
  | jq -r '.results[0] | "\(.alias):\(.lines)"' \
  | xargs -I {} blz find {} -C 5 --json \
  | jq -r '.requests[0].snippet'

# Multi-range helper: join snippets when ranges[] is present
blz find bun:41994-42009,42010-42020 -C 2 --json \
  | jq -r '.requests[0] | .snippet // ((.ranges // []) | map(.snippet) | join("\n\n"))'

# Filter high-confidence matches
blz find "test reporters" --json | jq '[.results[] | select(.score >= 60)]'

# Heading-level filtering for precision
blz find "configuration options" -H 2,3 --json | jq '.results[0:5]'
```

## Exit Codes

- `0` – Success
- `1` – Command ran but detected a problem (missing source, no matches, etc.)
- `2` – Clap/usage error (bad args)
- `124` – Parent guard timeout (usually CI / harness)
- `129` – Parent process disappeared; orphan guard shut down `blz`

## Deprecated

**Legacy commands**: The `search` and `get` commands are deprecated and will be removed in a future release. Use `find` instead:
- `blz search <query>` → use `blz find <query>`
- `blz get <citation>` → use `blz find <citation>`

**Legacy options**:
- `blz update` → use `blz refresh` (update still works with warning)
- `--block` → use `--context all`
- `-c<N>` → use `-C <N>` (grep-style)
