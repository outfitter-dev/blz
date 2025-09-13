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

## JSON and NDJSON

```bash
# JSON (aggregated with metadata)
blz search "async rust" --output json

# NDJSON (one hit per line)
blz search "async rust" --output ndjson
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
      "snippet": "...",
      "score": 0.95,
      "sourceUrl": "https://...",
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

```bash
# Show first 5 results
blz search "async rust" --limit 5

# Show page 2
blz search "async rust" --page 2 --limit 5

# Jump to last page
blz search "async rust" --last --limit 5
```

## Integration Examples

### With jq
```bash
blz search "async rust" --output json | jq '.results[0]'
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
count=$(echo "$results" | jq '.totalResults')
echo "Found $count results"
```
