# Diff

Show changes between the current source and the most recent archived version.

Usage:
```bash
blz diff <alias> [--since TIMESTAMP]
```

Behavior:
- Compares the current `llms.json` (and `llms.txt`) against the newest archived snapshot under `<cache_root>/<alias>/.archive/`.
- If `--since` is provided (e.g., `2025-09-14T10-00-00Z`), compares against the first archive whose timestamp is ≥ the provided value.
- Reports moved sections (based on stable anchors), added sections, and removed sections.
- Prints a text summary and a JSON payload.

JSON shape:
```json
{
  "alias": "react",
  "previous": { "sha256": "..." },
  "current": { "sha256": "..." },
  "moved": [
    {
      "anchor": "...",
      "headingPath": ["Getting Started", "Install"],
      "oldLines": "100-120",
      "newLines": "105-125",
      "oldContent": "...",
      "newContent": "..."
    }
  ],
  "added": [
    { "anchor": "...", "headingPath": ["..."], "lines": "200-230", "content": "..." }
  ],
  "removed": [
    { "anchor": "...", "headingPath": ["..."], "lines": "50-60", "content": "..." }
  ]
}
```

Notes:
- Moved detection uses stable anchors derived from headings.
- Content slices are provided for quick inspection; they reflect the specified line ranges.
