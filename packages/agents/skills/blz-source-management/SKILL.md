---
name: blz-source-management
description: Teaches how to discover, validate, and add llms.txt documentation sources to the blz CLI. Use when adding documentation to blz, finding llms.txt or llms-full.txt files, validating sources, refreshing content, or managing the documentation index. Covers source discovery, dry-run validation, and index file handling.
---

# BLZ Source Management

Add and manage llms.txt documentation sources for local search.

## Source Types

| Type | Description | Action |
|------|-------------|--------|
| `llms-full.txt` | Complete documentation (preferred) | Add directly |
| `llms.txt` (full) | Complete docs under standard name | Add directly |
| `llms.txt` (index) | Links to other docs (< 100 lines) | Expand linked files |

## Adding Sources

### Standard Workflow

```bash
# 1. Dry-run to validate
blz add react https://react.dev/llms.txt --dry-run --quiet

# 2. Check output
# Good: contentType: "full", lineCount > 1000
# Index: contentType: "index", lineCount < 100 → find linked docs

# 3. Add if good
blz add react https://react.dev/llms.txt -y

# 4. Verify
blz list --json | jq '.[] | select(.alias=="react")'
```

### Discovering URLs

Web search patterns:
```
"llms-full.txt" site:docs.example.com
"llms.txt" OR "llms-full.txt" <library-name>
site:github.com/org/repo "llms.txt"
```

Common URL patterns:
```
https://docs.example.com/llms-full.txt
https://example.com/llms.txt
https://example.com/llms-full.txt
```

## Index File Handling

If dry-run shows `contentType: "index"` with < 100 lines:

```bash
# Inspect the index
curl -s <index-url> | head -50

# Look for .txt references like:
# ./guides.txt
# ./js.txt
# ./python.txt

# Resolve and add each linked file
blz add supabase-guides https://supabase.com/llms/guides.txt -y
blz add supabase-js https://supabase.com/llms/js.txt -y
```

## Quality Criteria

| Criteria | Good | Index | Skip |
|----------|------|-------|------|
| contentType | "full" | "index" | "unknown" |
| lineCount | > 1000 | < 100 | < 50 |
| Action | Add | Expand | Investigate |

## Alias Best Practices

Good aliases:
- `bun`, `react`, `deno` (short, clear)
- `supabase-guides` (variant-specific)
- `langchain-python` (language-specific)

Bad aliases:
- `b` (too short)
- `my-docs` (vague)
- `the-react-docs-2024` (too long)

## Management Commands

```bash
# List sources
blz list --json
blz list --status --json    # With freshness

# Refresh sources
blz refresh --all --json    # Update all
blz refresh bun --json      # Update specific

# Remove source
blz remove bun

# Source details
blz info bun --json
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| 404 Not Found | Try `/llms.txt` vs `/llms-full.txt`, check GitHub repo |
| Small line count | Likely index file - expand linked docs |
| contentType unknown | Inspect content manually, verify it's markdown/text |
| Already exists | Use `blz refresh <alias>` to update |
