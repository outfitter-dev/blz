# Search

Search across all indexed documentation sources.

```bash
blz search <QUERY> [OPTIONS]
```
- `<QUERY>`: search terms
- `--source, -s <SOURCE>`: restrict to a source (canonical or metadata alias)
- `-n, --limit <N>`: limit results (default: 50)
- `--page <N>`: paginate (default: 1)
- `--top <N>`: keep top percentile (1–100)
- `-f, --format <FORMAT>`: `text` (default), `json`, `jsonl`

Examples

```bash
blz search "hooks" --source react -f json
blz "async await" react
blz react "server actions"
```

## How BLZ selects documentation

BLZ automatically uses the best documentation available for each source:
- When `llms-full.txt` is available, BLZ uses it for more comprehensive coverage
- Otherwise, BLZ uses `llms.txt` as a fallback
- This happens transparently with no configuration needed

Use `blz upgrade <source>` to fetch `llms-full.txt` for sources that only have `llms.txt`.

See also:
- [Output formats](./output-formats.md)
- [Global options](./global.md)
