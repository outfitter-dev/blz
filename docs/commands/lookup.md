# Lookup

Search registries for documentation to add.

```bash
blz lookup <QUERY> [--format text|json|jsonl]
```

> **Beta** · Results come from BLZ’s built-in registry, which is still tiny. Every lookup prints a reminder to open a PR with any helpful llms.txt manifests.
Examples

```bash
blz lookup typescript
blz lookup react -f json | jq '.[0]'
```
