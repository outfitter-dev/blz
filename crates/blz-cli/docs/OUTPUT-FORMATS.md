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