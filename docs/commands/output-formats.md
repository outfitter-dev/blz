# Output formats

The blz CLI supports multiple output formats to suit different use cases and integrations.

## Available formats

### Text (default)
Human-readable colored output optimized for terminal display.

```bash
blz search "async rust"
```

### JSON
Machine-readable JSON output for programmatic consumption.

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
- `suggestions` may be included when results are sparse or low-quality to aid discovery.
- `ndjson` emits one SearchHit per line (no aggregation metadata).

## Compact
Minimal output showing only essential information.

```bash
blz search "async rust" --output compact
```

Format: `<alias>:<lines> <heading_path>`

## Environment detection

The CLI automatically detects the output context:
- TTY: Uses colored text output
- Pipe: Uses plain text without colors
- CI: Adjusts formatting for CI environments

## Custom formatting

Override automatic detection:
```bash
# Force colors even when piping
blz search "async rust" --color always

# Disable colors for TTY
blz search "async rust" --color never

# Let CLI decide (default)
blz search "async rust" --color auto
```
