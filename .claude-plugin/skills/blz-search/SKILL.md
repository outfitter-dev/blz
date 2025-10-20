---
name: blz-search
description: Teaches effective documentation search using the blz CLI tool. Use when searching documentation with blz, looking up APIs, finding code examples, retrieving citations, or when questions mention libraries, frameworks, "how to", or documentation topics. Covers BM25 full-text search patterns, citation retrieval, and efficient querying.
---

# BLZ Search Patterns

Fast local documentation search using blz. Search is local, free, and fast (~6ms) - try many queries.

## Key Concepts

**Full-text search, not semantic**: blz uses BM25 ranking. Query with keywords that appear in docs:
- Good: `"useEffect cleanup"`, `"test configuration"`, `"HTTP server"`
- Bad: `"How do I use useEffect?"`, `"What's the best way to..."`

**Citations**: Results include citations like `bun:304-324` (source:start-end lines). Use these with `blz get` to retrieve content.

## Quick Patterns

```bash
# Check available sources first
blz list --status --json

# Basic search
blz "test runner" --json

# Search specific source
blz "hooks" --source react --json

# Retrieve by citation
blz get bun:304-324 --json

# Retrieve with full section context
blz get bun:304-324 --context all --json

# Retrieve with surrounding lines
blz get bun:304-324 -C 5 --json

# Batch retrieve multiple citations
blz get bun:304-324 deno:500-520 --json
```

## Search Strategy

1. **Start specific**: Use precise technical terms
2. **Try variations**: Synonyms, abbreviations, alternate terms
3. **Check sources**: Verify relevant docs are indexed
4. **Multiple searches**: Run 3-5 different queries - it's fast
5. **Narrow by source**: Use `--source` when you know the library

## Retrieval Options

| Flag | Use When |
|------|----------|
| `--json` | Always (structured output) |
| `--context all` | Need full section |
| `-C N` | Need N lines before/after |
| `-A N` / `-B N` | Asymmetric context |
| `--max-lines N` | Limit large sections |

## Common Pitfalls

- **Semantic queries**: "Compare X vs Y" won't work. Search `"X"` and `"Y"` separately.
- **Too broad**: "authentication" returns too much. Try `"JWT auth"`, `"OAuth flow"`.
- **Missing sources**: Check `blz list` first. Add sources with `blz add`.
- **One search only**: Try multiple query variations.

## MCP Alternative

For structured operations, MCP tools are also available:
```
mcp__blz__blz_find({ query: "test runner" })
mcp__blz__blz_find({ snippets: ["bun:304-324"] })
mcp__blz__blz_list_sources()
```
