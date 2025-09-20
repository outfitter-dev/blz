# Update

Update sources to the latest content.

Usage:
```bash
blz update [SOURCE] [--all] [--flavor current|auto|full|txt] [-y]
```

Options:
- `SOURCE`           Update only the specified source; omit to update current or use `--all`
- `--all`            Update all sources
- `--flavor`         Flavor policy during update:
  - `current` (default): keep existing URL/flavor
  - `auto`: prefer best available (llms-full.txt > llms.txt > others)
  - `full`: switch to llms-full.txt if available
  - `txt`: switch to llms.txt if available
- `-y, --yes`        Apply flavor changes without prompting/log hints (non-interactive)

Notes:
- The global config key `defaults.prefer_llms_full = true` (or `BLZ_PREFER_LLMS_FULL=1`) makes `full` the implied default when `--flavor` is not provided.
- Updates perform a HEAD preflight with size/ETA and fail fast on non-2xx responses.
- When multiple flavors exist, `blz update` refreshes and reindexes each (e.g., `llms.txt` and `llms-full.txt`) in a single pass so both stay in sync.
- Explicit `--flavor full` / `--flavor txt` writes a per-source override in `blz.json`, so future searches default to that variant. Use `--flavor auto` (or `current`) to clear the override and fall back to scope/global preferences.

See also:
- [Global options](./global.md)
