# Lookup

Search registries for documentation to add.

```bash
blz lookup <QUERY> [--format text|json|jsonl]
```
Examples

```bash
blz lookup typescript
blz lookup react -f json | jq '.[0]'
```
