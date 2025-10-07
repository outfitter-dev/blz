# Search llms.txt files locally with blz

`blz` is a CLI tool that downloads, parses, and indexes `llms.txt`* files locally to enable fast documentation search with line-accurate retrieval. You can use it in addition to or in place of MCP servers for documentation search.

> *(defaults to `llms-full.txt` if available)*

## Setup & Add Sources

```bash
# Add source by alias and URL
blz add anthropic https://docs.anthropic.com/en/llms-full.txt
blz add nextjs https://nextjs.org/llms-full.txt
blz add tanstack https://tanstack.com/llms-full.txt

# Add with auto-yes (non-interactive)
blz add react https://react.dev/llms-full.txt -y

# List all sources
blz list
blz list --format json  # machine-readable
```

## Search

```bash
# Basic search (all sources)
blz "react hooks"
blz "useEffect cleanup"

# Exact phrase (single quotes around double quotes)
blz '"claude code"'

# Require phrases/terms
blz '+"claude code" +"computer use"'

# Search specific source
blz "server components" -s nextjs
blz "query invalidation" -s tanstack

# Pagination
blz "async" --limit 20 --page 2
blz "async" --last  # jump to last page

# Output formats
blz "routing" --format json   # JSON array
blz "routing" --format jsonl  # newline-delimited
blz "routing" --json    # shortcut

> ⚠️ Compatibility: `--output`/`-o` is deprecated starting in v0.3. Use `--format`/`-f`. The alias remains temporarily for compatibility but emits a warning and will be removed in a future release.
```

## Get Exact Lines

```bash
# Get specific line range from source
blz get anthropic:100-150
blz get nextjs:2000-2100

# With context lines
blz get react:500-510 --context 5  # ±5 lines around range
```

## Advanced Usage

```bash
# Pipe to jq for JSON processing
blz "useState" --json | jq '.[] | {alias, score, lines}'

# Search and extract high-scoring results
blz "authentication" --json | jq '.[] | select(.score > 50)'

# Count results per source
blz "typescript" --json | jq 'group_by(.alias) | map({alias: .[0].alias, count: length})'

# Find and open in editor (macOS)
blz "useReducer" --json | jq -r '.[0] | "\(.alias):\(.lines)"' | read target && \
  blz get "$target" | pbcopy && echo "Copied to clipboard"

# Update sources
blz update --all        # update all sources
blz update anthropic    # update specific source

# Check for stale sources (>7 days old)
blz list --json | jq '.[] | select((.last_updated | fromdate) < (now - 604800))'

# Pull JSON prompts for tooling/agents
blz --prompt           # Global overview and workflows
blz --prompt search    # Retrieval playbook
blz --prompt add       # Onboarding checklist for new sources
```

## Search Tips

1. **OR by default**: Space-separated terms are ORed (any term matches)
2. **Phrase search**: Use `blz '"exact phrase"'` (single quotes outside, double inside)
3. **Require terms**: Prefix with `+` or use `AND`, e.g. `blz '+api +key'`
4. **Case-insensitive**: "React" = "react" = "REACT"
5. **Scoring**: Higher scores = better matches (BM25 algorithm)
6. **Line citations**: Results show exact line numbers for verification

## Performance Expectations

- Search: <10ms typical
- Add source: 1-3s (depends on size)
- Update: Only fetches if changed (ETag)
- Storage: ~2x source size (includes index)

## Common Patterns for Agents

```bash
# 1. Setup documentation for a project
for source in react typescript eslint prettier; do
  blz add $source https://${source}.dev/llms-full.txt -y
done

# 2. Search → Get full context
result=$(blz "custom hooks" --json | jq -r '.[0] | "\(.alias):\(.lines)"')
blz get "$result" --context 10

# 3. Build knowledge base
blz "api reference" --json > api_refs.json
blz "examples" --json > examples.json

# 4. Monitor for documentation updates
blz update --all 2>&1 | grep "Updated"
```

## Troubleshooting

```bash
# No results?
blz list  # check sources exist
blz update --all  # refresh sources

# Slow search?
blz "query" --debug  # show performance metrics

# Storage location
# macOS: ~/Library/Application Support/dev.outfitter.blz/
# Linux: ~/.local/share/dev.outfitter.blz/
```

## Exit Codes

- 0: Success
- 1: User error (bad args, no sources)
- 2: System error (network, filesystem)
