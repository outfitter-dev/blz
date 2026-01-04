# BLZ Claude Code Plugin

Fast local documentation search with llms.txt indexing. Search 12K+ line docs in 6ms with line-accurate citations.

## Installation

### Via Outfitter Marketplace (Recommended)

```bash
# Add the Outfitter marketplace
/plugin marketplace add outfitter-dev/agents

# Install BLZ plugin
/plugin install blz@outfitter
```

### Direct Installation

```bash
# Add this repository as a marketplace
/plugin marketplace add outfitter-dev/blz

# Install the plugin
/plugin install blz@blz
```

## Prerequisites

The BLZ CLI must be installed on your system:

```bash
# Quick install
curl -fsSL https://blz.run/install.sh | sh

# Verify installation
blz --version
```

## Usage

The plugin provides one command and one agent that handle all BLZ operations:

### Command: `/blz`

All BLZ operations go through a single command. The agent interprets your request and executes the appropriate operation.

```bash
# Search documentation
/blz "test runner"
/blz how do I write tests in Bun

# Add a documentation source
/blz add bun https://bun.sh/llms-full.txt

# List indexed sources
/blz list

# Retrieve content by citation
/blz find bun:304-324

# Refresh documentation sources
/blz refresh
/blz refresh bun

# Complex research
/blz Compare React hooks vs Vue composition API
```

### Agent: `@blz:blazer`

The blazer agent handles all documentation operations. You can invoke it directly or let the `/blz` command route to it.

**Capabilities:**
- Search across all indexed documentation
- Retrieve exact content by citation (e.g., `bun:304-324`)
- Add new documentation sources
- List, refresh, and manage sources
- Complex multi-source research and comparison

## Skills

The plugin includes two skills that teach BLZ usage patterns:

### `blz-docs-search`

Teaches effective documentation search patterns:
- Full-text search strategies (BM25, not semantic)
- Citation-based retrieval
- Context modes and flags
- MCP tool alternatives

### `blz-source-management`

Teaches source management:
- Discovering llms.txt URLs
- Validating sources with dry-run
- Handling index files
- Alias best practices

## Workflow Examples

### Adding documentation sources

```bash
# Add popular JavaScript runtimes
/blz add bun https://bun.sh/llms-full.txt
/blz add deno https://docs.deno.com/llms-full.txt
/blz add react https://react.dev/llms.txt

# Check what's installed
/blz list
```

### Searching and retrieving

```bash
# Search for test patterns
/blz "test runner configuration"

# After search returns citation bun:304-324
/blz find bun:304-324

# Get full section context
/blz find bun:304-324 --context all
```

### Keeping documentation fresh

```bash
# Refresh all sources
/blz refresh

# Refresh specific source
/blz refresh bun
```

## Tips

1. **Use specific keywords**: BLZ uses full-text search (BM25), not semantic search. Use keywords that appear in the documentation.

2. **Try multiple queries**: Local search is fast (~6ms). Run several variations to find what you need.

3. **Check sources first**: Run `/blz list` to see what documentation you have available.

4. **Use citations**: BLZ provides exact line ranges (e.g., `bun:304-324`) for precise retrieval.

5. **Prefer llms-full.txt**: When adding sources, look for `llms-full.txt` over `llms.txt` for complete documentation.

## Troubleshooting

### BLZ not installed

```bash
curl -fsSL https://blz.run/install.sh | sh
source ~/.bashrc  # or ~/.zshrc
blz --version
```

### No sources available

```bash
/blz list
# If empty, add some sources:
/blz add bun https://bun.sh/llms-full.txt
```

### Slow searches

BLZ searches should be very fast (6-10ms). If they're slow:
- Check disk space
- Refresh sources: `/blz refresh`
- Rebuild index if needed (see BLZ CLI docs)

## Links

- [BLZ Repository](https://github.com/outfitter-dev/blz)
- [BLZ Documentation](https://github.com/outfitter-dev/blz/tree/main/docs)
- [llms.txt Standard](https://llmstxt.org/)
- [Outfitter Marketplace](https://github.com/outfitter-dev/agents)

## Contributing

Found an issue or want to improve the plugin? Contributions are welcome!

- Report issues: https://github.com/outfitter-dev/blz/issues
- Submit PRs: https://github.com/outfitter-dev/blz/pulls

## License

MIT License - see [LICENSE](../LICENSE) for details.
