# blz Documentation

Welcome to the comprehensive documentation for blz, a fast local-first search cache for `llms.txt` ecosystems.

## 📚 Documentation Index

### Getting Started

- [**Quick Start Guide**](getting-started.md) - Installation, first steps, and basic usage
- [**Shell Integration**](shell-integration.md) - Setting up completions for Fish, Bash, and Zsh

### Core Features

- [**Managing Sources**](sources.md) - Adding, updating, and organizing documentation sources
- [**Search Guide**](search.md) - Search syntax, performance tips, and advanced queries
- [**Line-Accurate Retrieval**](retrieval.md) - Getting exact content with line ranges

### Technical Details

- [**Architecture**](architecture.md) - How it works under the hood
- [**Storage Format**](storage.md) - Understanding the blz structure

### Development

- [**API Reference**](api.md) - Rust API documentation
- [**MCP Integration**](mcp.md) - Using the Model Context Protocol server
- [**Contributing**](../CONTRIBUTING.md) - Development guidelines

## Key Concepts

### What is llms.txt?
`llms.txt` is a standardized format for making documentation accessible to AI agents. Sites like Bun.sh provide their docs in this format at URLs like `https://bun.sh/llms.txt`.

### Why @outfitter/blz?

- **6ms search latency** - Orders of magnitude faster than network requests
- **Line-accurate citations** - Reference exact `file#L120-L142` spans
- **Offline-first** - Works without internet after initial fetch
- **Smart updates** - Uses ETags to minimize bandwidth

## Quick Example

```bash
# Add Bun's documentation
blz add bun https://bun.sh/llms.txt

# Search instantly (6ms!)
blz search "test concurrency" --source bun

# Get specific lines
blz get bun --lines 304-324
```

## Performance at a Glance

| Operation | Time | Notes |
|-----------|------|-------|
| Search | 6ms | P50 on real docs |
| Add source | 373ms | Includes fetch + parse + index |
| Get lines | <1ms | Direct file access |

## Need Help?

- Check the [Getting Started Guide](getting-started.md)
- Read about [common patterns](search.md#common-patterns)
- File an issue on [GitHub](https://github.com/outfitter-dev/blz)
