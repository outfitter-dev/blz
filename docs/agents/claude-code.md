---
title: Using BLZ with Claude Code
description: Guide for using the BLZ Claude Code plugin, including installation, commands, agents, and best practices.
---

# BLZ Claude Code Plugin

The BLZ Claude Code plugin integrates local documentation search directly into your Claude Code workflow, providing millisecond-latency access to technical documentation with exact line citations.

## Installation

### Local Development

```bash
# From the blz repository root
/plugin install /path/to/blz/.claude-plugin
```

### Via Marketplace (Coming Soon)

```bash
# Add Outfitter marketplace
/plugin marketplace add outfitter-dev/agents

# Install BLZ plugin
/plugin install blz@outfitter
```

## Prerequisites

The BLZ CLI must be installed and available in your PATH:

```bash
# Install BLZ CLI
curl -fsSL https://blz.run/install.sh | sh

# Verify installation
blz --version
```

## Available Commands

### `/blz <query>`

Search across all indexed documentation sources.

**Examples:**
```bash
/blz "test runner"
/blz useEffect cleanup
```

### `/blz add <source> <url>`

Add a new documentation source to your BLZ index.

**Examples:**
```bash
/blz add bun https://bun.sh/llms-full.txt
/blz add react https://react.dev/llms.txt
```

### `/blz find <citation>`

Retrieve exact lines from a citation returned by search.

**Examples:**
```bash
/blz find bun:304-324
/blz find react:2000-2050 -C 5
```

### `/blz list`

List indexed documentation sources.

**Examples:**
```bash
/blz list
/blz list --status
```

### `/blz refresh [source]`

Refresh documentation sources (all or specific).

**Examples:**
```bash
/blz refresh
/blz refresh bun
```

## Available Agents

### `@blz:blazer`

**Purpose**: Unified documentation search, retrieval, and source management for BLZ.

**Capabilities**:
- Search and retrieve citations
- Add sources by URL or name (with auto-discovery)
- List and refresh sources
- Handle complex research requests across sources

**When to use**:
- Any BLZ documentation task
- Source management workflows
- Multi-source research and comparisons

## Skills

### `blz-docs-search`

Core skill teaching effective use of the BLZ CLI and MCP server. Provides patterns for full-text search, multi-source retrieval, and efficient querying.

**Key concepts**:
- MCP-first approach (prefer `mcp__blz__*` tools over CLI)
- Full-text search awareness (keywords, not semantic queries)
- Batching operations for efficiency
- Context modes (symmetric, all)

**Activation**: Automatically available, used by commands and agents.

### `blz-source-management`

Skill teaching source discovery, validation workflows, web search patterns, and post-addition integration.

**Key concepts**:
- llms.txt vs llms-full.txt
- Validation with `--dry-run`
- Index file detection and expansion
- Web search patterns for discovery

**Activation**: Used by the `@blz:blazer` agent for source discovery and management.

## Workflow Patterns

### Quick Lookup

For simple API lookups or single-concept searches:

```bash
# Direct search
/blz "useState hook"

# Retrieve content
/blz find react:1234-1256
```

### Complex Research

For comparing libraries, synthesizing information, or multi-step research:

```bash
/blz "Compare authentication approaches in Bun, Deno, and Node.js"
```

The agent will:
1. Check available sources
2. Add missing sources if needed
3. Run multiple searches with different terms
4. Return citations for relevant sections
5. Provide synthesis and comparison

### Adding Sources

**Direct addition**:
```bash
/blz add bun https://bun.sh/llms-full.txt
```

**Discovery mode**:
```bash
/blz "Add React docs"
```

**Dependency scanning**:
```bash
/blz "Scan project dependencies for docs"
```

### Keeping Sources Updated

```bash
/blz refresh
/blz refresh bun
```

## Best Practices

### Search Strategy

1. **Use keywords, not questions**: Full-text search works best with specific technical terms
   - ✅ "useEffect cleanup"
   - ❌ "How do I clean up useEffect?"

2. **Try multiple queries**: Local search is fast and free, so try 3-5 variations
   - "authentication JWT"
   - "auth token"
   - "login session"

3. **Use source filters**: When you know the library, narrow the search
   ```bash
   /blz "hooks" --source react
   ```

4. **Batch retrievals**: Get multiple citations in one call
   ```bash
   blz find bun:123-456 deno:789-900 react:2000-2050 --json
   ```

### Source Management

1. **Prefer llms-full.txt**: More comprehensive than llms.txt
2. **Scan dependencies**: Ask `/blz` to scan project dependencies when needed
3. **Keep updated**: Run `/blz refresh` periodically
4. **Check health**: Use `blz list --status --json` to verify sources

### Agent Usage

1. **All operations**: Use `/blz` (it invokes `@blz:blazer`)
2. **Complex research**: Ask the question directly via `/blz`
3. **Source management**: Use `/blz add`, `/blz list`, and `/blz refresh`
4. **Citation-based flow**: Let the agent return citations, retrieve content as needed

## Troubleshooting

### BLZ Not Installed

```bash
curl -fsSL https://blz.run/install.sh | sh
source ~/.bashrc  # or ~/.zshrc
blz --version
```

### No Sources Available

```bash
/blz list
# If empty:
/blz add bun https://bun.sh/llms-full.txt
/blz add react https://react.dev/llms.txt
```

### Slow Searches

BLZ searches should be 6-10ms. If slow:
- Check disk space
- Update sources: `/blz refresh`
- Check for index corruption: `blz info <source> --json`

### Agent Not Found

Ensure plugin is installed correctly:
```bash
/plugin list  # Verify blz plugin is listed
```

Reinstall if needed:
```bash
/plugin uninstall blz
/plugin install /path/to/blz/.claude-plugin
```

## Integration with Other Tools

### With Context7

For libraries without llms.txt:
1. Try blz first: `/blz "<library> <topic>"`
2. If no results, use context7 as fallback
3. Consider requesting llms.txt from the project

### With Firecrawl

For very recent information:
1. Use blz for stable API documentation
2. Use firecrawl for latest blog posts, changelogs
3. Combine: "According to blz docs... and recent changes from firecrawl..."

### With Web Search

For discovering new sources:
1. Search for: `"llms-full.txt" site:docs.example.com`
2. Add via `/blz add <alias> <url>` (or ask `/blz` to discover the URL)
3. Now available for instant local search

## Development Notes

### Plugin Structure

```
.claude-plugin/
├── README.md
├── plugin.json
├── agents/
│   └── blazer.md                 # Unified BLZ agent
├── commands/
│   └── blz.md
├── skills/
│   ├── blz-docs-search/          # Core search skill
│   └── blz-source-management/    # Source management skill
```

### Canonical Sources

- **Agents**: `.claude-plugin/agents/`
- **Commands**: `.claude-plugin/commands/`
- **Skills**: `.claude-plugin/skills/`

Build script can sync plugin files into a separate output directory when needed.

### Testing Changes

```bash
# Build plugin
./scripts/build-plugin.sh

# Reinstall in Claude Code
/plugin uninstall blz
/plugin install /path/to/blz/.claude-plugin

# Test commands
/blz "test"
/blz add bun https://bun.sh/llms-full.txt
```

## Additional Resources

- [BLZ Repository](https://github.com/outfitter-dev/blz)
- [BLZ CLI Usage Guide](./use-blz.md)
- [llms.txt Standard](https://llmstxt.org/)
- [Plugin README](../../.claude-plugin/README.md)

## Contributing

Found issues or want to improve the plugin?

- **Issues**: https://github.com/outfitter-dev/blz/issues
- **PRs**: https://github.com/outfitter-dev/blz/pulls
- **Discussions**: https://github.com/outfitter-dev/blz/discussions
