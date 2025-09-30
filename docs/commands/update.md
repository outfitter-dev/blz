# Update

Update sources to the latest content.

Usage:
```bash
blz update [SOURCE] [--all] [-y]
```

Options:
- `SOURCE`           Update only the specified source; omit to update current or use `--all`
- `--all`            Update all sources
- `-y, --yes`        Skip confirmation prompts (non-interactive)

## How updates work

BLZ automatically uses the best documentation available:
- When both `llms.txt` and `llms-full.txt` are available, BLZ uses `llms-full.txt`
- Updates check for new content using ETags and only re-fetch when content has changed
- Updates perform a HEAD preflight and fail fast on non-2xx responses

## Upgrading to llms-full.txt

If a source only has `llms.txt` but the upstream now provides `llms-full.txt`, use the upgrade command:

```bash
blz upgrade <source>
```

See `blz upgrade --help` for more details.

See also:
- [Global options](./global.md)
- [Upgrade command](./upgrade.md)
