---
title: Using the `blz` CLI to search llms.txt docs
description: Use the `blz` CLI tool to search and retrieve documentation from locally cached llms.txt files in milliseconds.
---


Use the [`blz`](https://github.com/outfitter-dev/blz) CLI to keep documentation local, search it in milliseconds (P50 ≈ 6ms), and return grounded spans with exact citations. Keep payloads lean so agents spend fewer tokens per query compared to traditional page-level search.

## Instructions

1. Verify the CLI: `blz --version`
2. Pull guidance on demand: `blz --prompt` or `blz --prompt <command>`
3. Inspect indexed sources: `blz list --json`
4. Prefer `--json` / `--jsonl` outputs so tooling can parse them cleanly
5. Feed `alias:lines` citations straight into `blz get`
6. Use `--block` (plus `--max-lines <N>` if needed) for whole sections, or `-c<N>` to add a small context window
7. Expect tight payloads: the standard `search` → `get` flow typically returns 20–80 lines instead of multi-kilobyte pages, keeping token usage low
8. Set a default format when you want every command in JSON: `export BLZ_OUTPUT_FORMAT=json`
9. Check source health with `blz info <alias> --json` (or `blz list --status --json`) before a heavy retrieval session

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

## Search

```bash
# Ranked matches with minimal effort
blz "test runner" --json

# Narrow the scope
blz "test reporters" --json
```

## Retrieve Content

```bash
# Copy alias:lines from search output and fetch the span with context
blz get bun:41994-42009 -c5 --json

# Combine multiple spans into one payload
blz get bun --lines "41994-42009,42010-42020" --json

# Expand to the whole heading (and cap the output to 80 lines)
blz get bun:41994-42009 --block --max-lines 80 --json
```

## Keep Sources Fresh

```bash
blz update bun --json        # Refresh a single source
blz update --all --json      # Refresh everything
```

## Common JSON Pipelines

```bash
# Pull the first alias:lines citation
span=$(blz "test runner" --json | jq -r '.[0] | "\(.alias):\(.lines)"')

# Fetch the content straight into your pipeline
blz get "$span" --json | jq '.content'

# Filter to high-confidence matches
blz "test reporters" --json | jq '[.[] | select(.score >= 60)]'
```

## Exit Codes

- `0` – Success
- `1` – Command ran but detected a problem (missing source, no matches, etc.)
- `2` – Clap/usage error (bad args)
- `124` – Parent guard timeout (usually CI / harness)
- `129` – Parent process disappeared; orphan guard shut down `blz`
