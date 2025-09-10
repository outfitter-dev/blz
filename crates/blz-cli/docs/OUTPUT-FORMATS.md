# Output Formats

The `blz` CLI supports multiple output formats optimized for both human use and automation.

## Text (default)

Human-friendly “brief” layout that’s fast to scan. Colors use ANSI when writing to a TTY (values only in the summary line).

```bash
❯ blz search "async rust"
rust:123-145 (score: 9)
Async > Futures
... async/await lets you write asynchronous code that looks synchronous ...

50/150 results shown, 57325 lines searched, took 5ms
Tip: use "blz search --next" to see the next page (or "--page 2" in a full query)
```

Text modifiers are provided via `--output`:

```bash
# Default brief layout
blz search "async rust" --output text

# Brief + rank numbers + URL lines header
blz search "async rust" --output text,rank,url
```

Notes:
- Shows a header `<alias>:<start-end> (score: N)`
- Heading path on its own line (links and markdown stripped)
- Three-line snippet: one line before, match line, one line after
- Query matches highlighted in red (bold red for exact phrase, dim red for token matches)
- When results are paginated, summary shows `shown/total`; otherwise `total results found`
- When `url` modifier is present, prints a page sources header above results:

```
Results 50/150:
[rust] https://doc.rust-lang.org/llms.txt
[node] https://nodejs.org/llms.txt
```

## JSON and JSONL

For programmatic use:

```bash
# JSON array of hits (shortcut)
blz search "async rust" --json

# NDJSON (one hit per line)
blz search "async rust" --jsonl

# Equivalent long-form
blz search "async rust" --output json
blz search "async rust" --output jsonl
```

Each hit has the structure:

```json
{
  "alias": "rust",
  "file": "llms.txt",
  "heading_path": ["Async", "Futures"],
  "lines": "123-145",
  "snippet": "...",
  "score": 0.95,
  "source_url": null,
  "checksum": ""
}
```

## JSON Full (Envelope)

For scripts that want metadata and hits together:

```bash
blz search "async rust" --output json-full
```

Example structure:

```json
{
  "query": "async rust",
  "total_results": 42,
  "search_time_ms": 6,
  "sources": ["rust"],
  "hits": [
    { "alias": "rust", "file": "llms.txt", "heading_path": ["Async", "Futures"], "lines": "123-145", "snippet": "...", "score": 0.95, "source_url": null, "checksum": "" }
  ]
}
```

## Pagination

```bash
# Limit and page
blz search "async rust" --limit 5 --page 2

# Show all results (up to 10k)
blz search "async rust" --all

# Continue to next/previous page using history
blz search --next
blz search --prev

## Options that affect text output

```bash
# Suppress bottom summary stats
blz search "rust" --output text --no-stats
```

Notes:
- Results are grouped by section to avoid duplicated headers; gaps are indicated as `... N more lines`.
- When `--output text,rank` is used, each group is prefixed with an ordinal.
```

## Tips

- Use `-s, --source` to focus on one source: `blz search "async" -s rust`
- Use `--json` or `--jsonl` for scripting with `jq`/pipes
