# Get

Retrieve exact line ranges from a source.

```bash
blz get <SOURCE:LINES> [--context N] [--format text|json|jsonl]

# Back-compat form:
blz get <SOURCE> --lines <RANGE> [...]
```
- `<SOURCE:LINES>`: preferred shorthand (matches search hits, e.g. `bun:120-142`)
- `<SOURCE>`: canonical source or metadata alias (use with `--lines`)
- `--lines`: range(s), e.g. `120-142`, `36:43,320:350`, `36+20`
- `--context`: lines around each range
- `--format`: default `text`; JSON/JSONL for agents

Examples

```bash
blz get bun:120-142
blz get node:10:20,50:60
blz get deno:100-110 --context 3
blz get bun:42-55 -f json | jq '.content'
```

See also:
- [Output formats](./output-formats.md)
