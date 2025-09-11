# Output Formats

The blz CLI supports multiple output formats to suit different use cases and integrations.

## Available Formats

### Text (Default)
Human-readable colored output optimized for terminal display.

```bash
blz search "async rust"
```

Features:
- Colored aliases for visual distinction
- Hierarchical heading paths
- Score display
- Content snippets with ellipsis for long results
- Search performance statistics

### JSON
Machine-readable JSON output for programmatic consumption.

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
```

Output structure:
```json
{
  "query": "async rust",
  "total_results": 42,
  "search_time_ms": 6,
  "hits": [
    {
      "alias": "rust",
      "file": "llms.txt",
      "heading_path": ["Async", "Futures"],
      "lines": "123-145",
      "snippet": "...",
      "score": 0.95,
      "source_url": "https://...",
      "checksum": "..."
    }
  ]
}
```

### Compact
Minimal output showing only essential information.

```bash
blz search "async rust" --output compact
```

Format: `<alias>:<lines> <heading_path>`

### Markdown (Planned)
Formatted markdown output suitable for documentation.

```bash
blz search "async rust" --output markdown
```

## Environment Detection

The CLI automatically detects the output context:
- TTY: Uses colored text output
- Pipe: Uses plain text without colors
- CI: Adjusts formatting for CI environments

## Custom Formatting

Override automatic detection:
```bash
# Force colors even when piping
blz search "async rust" --color always

# Disable colors for TTY
blz search "async rust" --color never

# Let CLI decide (default)
blz search "async rust" --color auto
```

## Pagination

Control result pagination:
```bash
# Show first 5 results
blz search "async rust" --limit 5

# Show results 10-20
blz search "async rust" --offset 10 --limit 10
```

## Integration Examples

### With jq
```bash
blz search "async rust" --output json | jq '.hits[0]'
```

Notes:

- Results are grouped by section to avoid duplicated headers; gaps are indicated as `... N more lines`.
- When `--output text,rank` is used, each group is prefixed with an ordinal.
```

### With fzf
```bash
blz search "async rust" --output compact | fzf
```

### In scripts
```bash
#!/bin/bash
results=$(blz search "$1" --output json)
count=$(echo "$results" | jq '.total_results')
echo "Found $count results"
```
