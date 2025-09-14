# Get

Retrieve exact line ranges from a source.

```bash
blz get <SOURCE> --lines <RANGE> [--context N] [--output text|json|ndjson]
```
- `<SOURCE>`: canonical source or metadata alias
- `--lines`: range(s), e.g. `120-142`, `36:43,320:350`, `36+20`
- `--context`: lines around each range
- `--output`: default `text`; JSON/NDJSON for agents

Examples

```bash
blz get bun --lines 120-142
blz get node --lines "10:20,50:60"
blz get deno --lines 100-110 --context 3
blz get bun --lines 42-55 -o json | jq '.content'
```
