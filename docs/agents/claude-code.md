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
/plugin install /path/to/blz/claude-plugin
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

### `/blz-add <source> <url>`

Add a new documentation source to your BLZ index.

**Examples:**
```bash
/blz-add bun https://bun.sh/llms-full.txt
/blz-add react https://react.dev/llms.txt
```

### `/blz-quick-search <query>`

Quick search across all indexed documentation sources.

**Examples:**
```bash
/blz-quick-search "test runner"
/blz-quick-search "useEffect cleanup"
```

### `/blz-retrieve <citation>`

Retrieve exact lines from a citation returned by search.

**Examples:**
```bash
/blz-retrieve bun:304-324
/blz-retrieve react:2000-2050 -C 5
```

### `/blz-manage <action> [source]`

Manage documentation sources (list, update, remove, stats).

**Examples:**
```bash
/blz-manage list              # List all sources
/blz-manage update            # Update all sources
/blz-manage update bun        # Update specific source
/blz-manage remove old-docs   # Remove a source
/blz-manage stats             # Show statistics
```

### `/add-source [name] [url]`

Intelligent source addition with discovery and dependency scanning. Delegates to the `@blz-source-manager` agent.

**Examples:**
```bash
/add-source bun https://bun.sh/llms-full.txt   # Direct add
/add-source react                               # Discover URL
/add-source                                     # Scan dependencies
```

### `/search-docs-with-blz <query>`

Advanced search using the `@blz-docs-searcher` agent for complex research, synthesis, and multi-source queries.

**Examples:**
```bash
/search-docs-with-blz Find information about Bun's HTTP server
/search-docs-with-blz Compare authentication in Bun vs Deno
```

## Available Agents

### `@blz-docs-searcher`

**Purpose**: Complex documentation research, synthesis, and comparison.

**Capabilities**:
- Multi-source research and comparison
- Citation-based returns (keeps context clean)
- Parallel subagent execution for independent queries
- Handles adding new sources autonomously
- Deep dives into specific topics

**When to use**:
- Comparing multiple libraries/frameworks
- Synthesizing information across sources
- Multi-step research requiring context building
- Deep exploration of complex topics

**Example invocation**:
```javascript
Task({
  subagent_type: "blz-docs-searcher",
  prompt: "Compare routing approaches in Next.js, Remix, and TanStack Router",
  description: "Compare framework routing"
})
```

**Returns**: Citations (e.g., `next:123-456`, `remix:789-900`) rather than full content, allowing the orchestrating agent to retrieve content as needed.

### `@blz-source-manager`

**Purpose**: Intelligent documentation source management with discovery and validation.

**Capabilities**:
- Add sources by URL or name (with auto-discovery)
- Expand index files to find full documentation
- Scan project dependencies (Cargo.toml, package.json)
- Batch additions with validation
- Proactive suggestions for comprehensive coverage

**When to use**:
- Adding new documentation sources
- Discovering llms.txt URLs from library names
- Scanning dependencies for documentation candidates
- Validating and managing existing sources

**Example invocation**:
```javascript
Task({
  subagent_type: "blz-source-manager",
  prompt: "Scan project dependencies and add any documentation not yet indexed",
  description: "Add missing docs from dependencies"
})
```

## Skills

### `blz-search`

Core skill teaching effective use of blz CLI and MCP server. Provides patterns for full-text search, multi-source retrieval, and efficient querying.

**Key concepts**:
- MCP-first approach (prefer `mcp__blz__*` tools over CLI)
- Full-text search awareness (keywords, not semantic queries)
- Batching operations for efficiency
- Context modes (symmetric, all)

**Activation**: Automatically available, used by commands and agents.

### `add-blz-source`

Skill teaching source discovery, validation workflows, web search patterns, and post-addition integration.

**Key concepts**:
- llms.txt vs llms-full.txt
- Validation with `--dry-run`
- Index file detection and expansion
- Web search patterns for discovery

**Activation**: Used by `@blz-source-manager` agent and `/add-source` command.

## Workflow Patterns

### Quick Lookup

For simple API lookups or single-concept searches:

```bash
# Direct search
/blz-quick-search "useState hook"

# Retrieve content
/blz-retrieve react:1234-1256
```

### Complex Research

For comparing libraries, synthesizing information, or multi-step research:

```bash
# Use the specialized agent
/search-docs-with-blz Compare authentication approaches in Bun, Deno, and Node.js
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
/blz-add bun https://bun.sh/llms-full.txt
```

**Discovery mode**:
```bash
/add-source react  # Agent finds URL
```

**Dependency scanning**:
```bash
/add-source  # Scans Cargo.toml, package.json
```

### Keeping Sources Updated

```bash
/blz-manage update           # Update all
/blz-manage update bun deno  # Update specific sources
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
   /blz-quick-search "hooks" --source react
   ```

4. **Batch retrievals**: Get multiple citations in one call
   ```bash
   blz get bun:123-456 deno:789-900 react:2000-2050 --json
   ```

### Source Management

1. **Prefer llms-full.txt**: More comprehensive than llms.txt
2. **Scan dependencies**: Use `/add-source` without args to discover project needs
3. **Keep updated**: Run `/blz-manage update` periodically
4. **Check health**: Use `blz list --status --json` to verify sources

### Agent Usage

1. **Simple lookups**: Use commands directly (`/blz-quick-search`)
2. **Complex research**: Delegate to `@blz-docs-searcher`
3. **Source management**: Use `@blz-source-manager` for batch operations
4. **Citation-based flow**: Let agents return citations, retrieve content as needed

## Troubleshooting

### BLZ Not Installed

```bash
curl -fsSL https://blz.run/install.sh | sh
source ~/.bashrc  # or ~/.zshrc
blz --version
```

### No Sources Available

```bash
/blz-manage list
# If empty:
/add-source bun
/add-source react
```

### Slow Searches

BLZ searches should be 6-10ms. If slow:
- Check disk space
- Update sources: `/blz-manage update`
- Check for index corruption: `blz info <source> --json`

### Agent Not Found

Ensure plugin is installed correctly:
```bash
/plugin list  # Verify blz plugin is listed
```

Reinstall if needed:
```bash
/plugin uninstall blz
/plugin install /path/to/blz/claude-plugin
```

## Integration with Other Tools

### With Context7

For libraries without llms.txt:
1. Try blz first: `/blz-quick-search "<library> <topic>"`
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
2. Add via `/add-source`
3. Now available for instant local search

## Development Notes

### Plugin Structure

```
claude-plugin/
├── .claude-plugin/
│   └── plugin.json          # Plugin metadata
├── agents/
│   ├── blz-docs-searcher.md # Research agent
│   └── blz-source-manager.md # Source management agent
├── commands/
│   ├── blz-add.md
│   ├── blz-quick-search.md
│   ├── blz-retrieve.md
│   ├── blz-manage.md
│   ├── add-source.md
│   └── search-docs-with-blz.md
├── skills/
│   ├── blz-search/         # Core search skill
│   └── add-blz-source/     # Source addition skill
└── README.md
```

### Canonical Sources

- **Agents**: `.claude/agents/` (synced to `claude-plugin/agents/`)
- **Commands**: `claude-plugin/commands/` (canonical)
- **Skills**: `claude-plugin/skills/` (canonical)

Build script syncs agents from `.claude/` to plugin directory.

### Testing Changes

```bash
# Build plugin
./scripts/build-plugin.sh

# Reinstall in Claude Code
/plugin uninstall blz
/plugin install /path/to/blz/claude-plugin

# Test commands
/blz-quick-search "test"
/add-source bun
```

## Additional Resources

- [BLZ Repository](https://github.com/outfitter-dev/blz)
- [BLZ CLI Usage Guide](./use-blz.md)
- [llms.txt Standard](https://llmstxt.org/)
- [Plugin README](../../claude-plugin/README.md)

## Contributing

Found issues or want to improve the plugin?

- **Issues**: https://github.com/outfitter-dev/blz/issues
- **PRs**: https://github.com/outfitter-dev/blz/pulls
- **Discussions**: https://github.com/outfitter-dev/blz/discussions
