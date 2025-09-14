# Anchors

Stable anchors and anchor-based retrieval.

```bash
blz anchors <SOURCE> [--mappings] [--output text|json|ndjson]
blz anchor get <SOURCE> <ANCHOR> [--context N] [--output text|json|ndjson]
```
Examples

```bash
blz anchors react --mappings
blz anchor list react -o json | jq '.[0]'
blz anchor get react <ANCHOR> -o json | jq '.content'
```
