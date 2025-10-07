# Lookup

Search registries for documentation to add.

```bash
blz lookup <QUERY> [--format text|json|jsonl]
```

> **Beta** · BLZ’s built-in llms.txt registry is still small. You can always add any llms-full.txt manually with `blz add <alias> <url>`. If you want it included for everyone, open a PR at https://github.com/outfitter-dev/blz/.
Examples

```bash
blz lookup typescript
blz lookup react -f json | jq '.[0]'
```
