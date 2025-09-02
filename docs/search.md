# Search Guide

Master the art of searching your indexed documentation with blz's fast local search.

## Basic Search

### Quick Search Patterns

The fastest way to search - just type `blz` followed by your query:

```bash
# Search all sources
blz "test"
blz "http server"
blz "typescript"

# Search specific source (source first)
blz bun "test"
blz node "http server"
blz deno "typescript"

# Search specific source (source last)
blz "test" bun
blz "http server" node
blz "typescript" deno
```

### Full Search Command

For more control, use the explicit `search` command:

```bash
blz search "your query"
blz search "test" --alias bun
```

### Pattern Summary

```bash
# Quick patterns (most common)
blz QUERY                    # Search all sources
blz SOURCE QUERY            # Source-specific (source first)
blz QUERY SOURCE            # Source-specific (source last)

# Explicit command (more options)
blz search QUERY
blz search QUERY --alias SOURCE
```

## Search Syntax

### Single Terms

Simple word searches:

```bash
blz search "bundler"      # Finds: bundler, bundlers, bundling
blz search "test"         # Finds: test, testing, tests
```

### Multiple Terms

Space-separated terms create an AND query:

```bash
blz search "test runner"   # Must contain both "test" AND "runner"
blz search "http server"   # Must contain both "http" AND "server"
```

### Phrase Search (Coming Soon)

Future support for exact phrases:

```bash
blz search '"test runner"'  # Exact phrase (not yet implemented)
```

## Search Options

### Limit Results

Control how many results you get:

```bash
blz search "test" --limit 5    # Default: 10
blz search "test" --limit 20   # Get more results
blz search "test" --limit 1    # Just the best match
```

### Output Format

#### Pretty (Default)
Human-readable output with colors:

```bash
blz search "test"
```

Output:

```
Search results for 'test':

1. bun (score: 4.09)
   Path: Bun Documentation > Guides > Test runner
   Lines: L304-324
   Snippet: ### Guides: Test runner...
```

#### JSON
Machine-readable for scripting:

```bash
blz search "test" --output json
```

Output:

```json
{
  "hits": [
    {
      "alias": "bun",
      "file": "llms.txt",
      "heading_path": ["Bun Documentation", "Guides", "Test runner"],
      "lines": "304-324",
      "snippet": "### Guides: Test runner...",
      "score": 4.09,
      "source_url": null,
      "checksum": ""
    }
  ]
}
```

## Understanding Results

### Result Structure

Each result contains:

- **Alias** - Which source it's from
- **Score** - Relevance score (higher is better)
- **Path** - Heading hierarchy to the content
- **Lines** - Exact line range in the source
- **Snippet** - Preview of the content

### Relevance Scoring

Results are ranked by BM25 score:

- Higher scores = better matches
- Scores > 4.0 = excellent match
- Scores 2.0-4.0 = good match
- Scores < 2.0 = partial match

### Heading Paths

Shows the document structure:

```
Path: Bun Documentation > Guides > Test runner
      ^-- Top level     ^-- Section  ^-- Subsection
```

## Advanced Patterns

### Find Commands

Search for CLI commands:

```bash
# Quick patterns
blz "bun test"
blz "npm install"
blz "--watch flag"

# Or target specific sources
blz bun "test command"
blz node "npm install"
```

### Find Configuration

Search for config options:

```bash
# Quick patterns
blz "tsconfig"
blz "package.json"
blz "bundler config"

# Or target specific sources
blz typescript "tsconfig"
blz bun "package.json fields"
```

### Find APIs

Search for specific APIs:

```bash
# Quick patterns
blz "fetch API"
blz "file system"
blz "process.env"

# Or target specific sources
blz deno "fetch API"
blz node "file system"
```

## Search Performance

### Speed Expectations

| Scenario | Expected Time |
|----------|--------------|
| Single source, <1MB | 4-8ms |
| Single source, 1-5MB | 8-15ms |
| All sources, <10MB total | 10-30ms |
| Large corpus, >100MB | 30-50ms |

### Performance Tips

1. **Use aliases** - Searching one source is faster
   ```bash
   blz bun "test"                 # Fastest - quick pattern
   blz search "test" --alias bun  # Fast - explicit command
   blz "test"                     # Slower - searches all
   ```

2. **Limit results** - Get results faster
   ```bash
   blz search "test" --limit 3
   ```

3. **Cache warmup** - First search may be slower as OS caches the index

## Scripting with Search

### Extract Best Match

```bash
#!/bin/bash
# Get the best match for a query

result=$(blz search "test runner" --limit 1 --output json)
alias=$(echo "$result" | jq -r '.hits[0].alias')
lines=$(echo "$result" | jq -r '.hits[0].lines')

echo "Best match in $alias at lines $lines"
blz get "$alias" --lines "$lines"
```

### Search and Open

```bash
#!/bin/bash
# Search and display the top result

query="$1"
result=$(blz search "$query" --limit 1 --output json | jq -r '.hits[0]')

if [ "$result" != "null" ]; then
  alias=$(echo "$result" | jq -r '.alias')
  lines=$(echo "$result" | jq -r '.lines')

  echo "Opening $alias at lines $lines..."
  blz get "$alias" --lines "$lines"
else
  echo "No results found for: $query"
fi
```

### Build Context for AI

```bash
#!/bin/bash
# Gather context for an AI prompt

query="typescript config"
results=$(blz search "$query" --limit 5 --output json)

echo "Context for query: $query"
echo "$results" | jq -r '.hits[] |
  "Source: \(.alias)\nSection: \(.heading_path | join(" > "))\n\(.snippet)\n"'
```

## Common Searches

### By Topic

```bash
# Testing
blz search "test"
blz search "test runner"
blz search "unit test"

# Performance
blz search "performance"
blz search "benchmark"
blz search "optimization"

# Configuration
blz search "config"
blz search "settings"
blz search "options"

# APIs
blz search "API"
blz search "http"
blz search "fetch"
```

### By Technology

```bash
# Languages
blz search "typescript"
blz search "javascript"
blz search "jsx"

# Tools
blz search "bundler"
blz search "transpiler"
blz search "compiler"

# Frameworks
blz search "react"
blz search "vue"
blz search "express"
```

## Troubleshooting

### No Results

If search returns nothing:

1. Check you have sources: `blz list`
2. Try simpler terms: `"test"` instead of `"testing framework"`
3. Check spelling

### Too Many Results

If overwhelmed with results:

1. Use `--alias` to focus on one source
2. Use more specific terms
3. Reduce `--limit`

### Unexpected Results

BM25 scoring considers:

- Term frequency in document
- Inverse document frequency
- Document length normalization

Short documents with many occurrences score higher.

## Search Internals

### How It Works

1. **Query parsing** - Tokenizes your search terms
2. **Index lookup** - Tantivy searches the inverted index
3. **BM25 scoring** - Ranks results by relevance
4. **Result assembly** - Adds snippets and metadata
5. **Output formatting** - Pretty or JSON output

### Index Structure

Each source has its own Tantivy index with:

- Heading-based document chunks
- Full-text searchable content
- Stored heading paths and line ranges
- BM25 relevance scoring

## Next Steps

- Learn about the "get" command in [CLI documentation](cli.md) for retrieving exact content
- Set up [Shell Integration](shell-integration.md) for better productivity
- Understand the [Architecture](architecture.md) for deeper knowledge
