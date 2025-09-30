# Upgrade

Upgrade sources from `llms.txt` to `llms-full.txt` when the full version becomes available upstream.

Usage:
```bash
blz upgrade [SOURCE] [--all] [-y]
```

Options:
- `SOURCE`           Upgrade only the specified source; omit to check all sources
- `--all`            Upgrade all eligible sources
- `-y, --yes`        Skip confirmation prompts (non-interactive)

## How upgrades work

The upgrade command:
1. Checks which sources are currently using only `llms.txt`
2. Queries upstream to see if `llms-full.txt` is now available
3. Prompts for confirmation (unless `--yes` is used)
4. Fetches and indexes the `llms-full.txt` content
5. Preserves the existing `llms.txt` for backward compatibility

## Examples

Check if a specific source can be upgraded:
```bash
blz upgrade react
```

Upgrade all eligible sources:
```bash
blz upgrade --all
```

Non-interactive upgrade:
```bash
blz upgrade react --yes
```

## When to use upgrade

Use `blz upgrade` when:
- You initially added a source that only had `llms.txt`
- The upstream now provides `llms-full.txt`
- You want more comprehensive documentation coverage

## Automatic upgrades

When adding new sources, BLZ automatically fetches `llms-full.txt` if available. The upgrade command is only needed for sources that were added before `llms-full.txt` became available upstream.

See also:
- [Update command](./update.md)
- [Global options](./global.md)