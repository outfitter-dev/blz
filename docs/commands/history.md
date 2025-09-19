# `blz history`

Display recent searches and the CLI presentation defaults that will be applied to future runs. History entries are stored in `cli-preferences.json` under the active configuration directory (defaults to the platform config dir, or the directory pointed to by `BLZ_CONFIG_DIR` / `BLZ_CONFIG`).

## Usage

```bash
blz history [--limit <N>] [-f text|json|jsonl]
```

## Options

- `--limit <N>` – Maximum number of entries to display (default: 20)
- `-f, --format <FORMAT>` – Output format (`text`, `json`, `jsonl`). Inherits `BLZ_OUTPUT_FORMAT` when not provided.

## Examples

```bash
# Show the five most recent searches with defaults
blz history --limit 5

# Inspect history in JSON for automation
blz history -f json | jq '.[0]'
```

Text output prints the current defaults (show components, snippet lines, score precision) followed by the most recent searches in reverse chronological order. JSON/JSONL output returns the raw history entries, newest first.

## Persistence

History is scoped to the configuration directory:
- Default: platform config dir (e.g., `~/Library/Application Support/dev.outfitter.blz/` on macOS)
- Custom: set `BLZ_CONFIG_DIR` or `BLZ_CONFIG` to isolate preferences per project or agent

Each entry keeps the query, optional alias, output format, show components, snippet lines, score precision, and ISO‑8601 timestamp.
