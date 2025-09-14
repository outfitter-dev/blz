# Lookup

Search registries for documentation to add.

```bash
blz lookup <QUERY> [--output text|json|ndjson]
```
Examples

```bash
blz lookup typescript
blz lookup react -o json | jq '.[0]'
```
