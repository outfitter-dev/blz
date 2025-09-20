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
- `--flavor <MODE>`: override flavor for this run (`current`, `auto`, `full`, `txt`)

Examples

```bash
blz search "hooks" --source react -f json
blz "async await" react
blz react "server actions"
```

## Flavor resolution

`blz search` automatically scopes each source to its resolved flavor:

- Per-source overrides stored in `blz.json` (set via `blz update --flavor …`) win first.
- Otherwise the order is local → project → global config, then built-in defaults.
- When a preferred flavor is missing on disk, search quietly falls back to `llms`.

Use `--flavor` to bypass the resolved preference for a single invocation. For example, `--flavor full`
forces `llms-full` even if the default is `llms`, while `--flavor txt` returns to the base flavor when
full is preferred.

This keeps searches consistent with update preferences while still allowing future explicit `--flavor` controls.

See also:
- [Output formats](./output-formats.md)
- [Global options](./global.md)
