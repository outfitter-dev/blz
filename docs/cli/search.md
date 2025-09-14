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
- `-o, --output <FORMAT>`: `text` (default), `json`, `ndjson`

Examples

```bash
blz search "hooks" --source react -o json
blz "async await" react
blz react "server actions"
```
