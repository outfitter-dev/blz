# @outfitter/cache

A local-first, line-accurate docs cache and MCP server for lightning-fast lookups of `llms.txt` ecosystems. Search in milliseconds, cite exact lines, keep diffs, and stay fresh via conditional fetches. Powered by Rust + Tantivy for speed and determinism.

## Features

- 🚀 **Blazing Fast**: P50 search < 80ms on 10-50 MB corpus
- 📍 **Line-Accurate**: Returns exact `file#L120-L142` spans with heading context
- 🔄 **Smart Sync**: Conditional fetches with ETag/If-None-Match to minimize bandwidth
- 📚 **Durable Parsing**: Handles imperfect `llms.txt` gracefully, always produces useful structure
- 🔍 **Lexical Search**: Deterministic BM25 ranking with Tantivy (vectors optional, off by default)
- 📝 **Change Tracking**: Built-in diff journal with unified diffs and changed sections
- 🤖 **MCP Integration**: Official Rust SDK for IDE/agent consumption

## Installation

### From Source
```bash
# Clone and install
git clone https://github.com/outfitter-dev/cache
cd cache
cargo install --path crates/cache-cli

# Or install directly from GitHub
cargo install --git https://github.com/outfitter-dev/cache --branch main cache-cli
```

### Shell Setup

#### Fish
```fish
# Add to PATH
set -gx PATH $HOME/.cargo/bin $PATH

# Install completions
cache completions fish > ~/.config/fish/completions/cache.fish
```

#### Bash/Zsh
```bash
# Add to PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Install completions (Bash)
cache completions bash > ~/.local/share/bash-completion/completions/cache

# Install completions (Zsh)
cache completions zsh > ~/.zsh/completions/_cache
```

## Quick Start

```bash
# Add a source
cache add bun https://bun.sh/llms.txt

# Search across docs (6ms latency!)
cache search "test concurrency" --alias bun

# Get exact lines
cache get bun --lines 120-142

# List all sources
cache sources

# Update all sources (not yet implemented)
cache update --all

# View changes (not yet implemented)
cache diff bun --since "2025-08-20T00:00:00Z"
```

## Architecture

```
┌─────────────────────┐
│ CLI / MCP Server    │
└──────────┬──────────┘
           │
┌──────────▼──────────┐      ┌─────────────────┐
│ Core Engine (Rust)  │◄────►│ Tantivy Index   │
│ - Fetcher (ETag)    │      └─────────────────┘
│ - Parser (tree-sitter)
│ - Search (BM25)     │
│ - Diff (similar)    │
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│ Storage             │
│ ~/.outfitter/cache/ │
│ - llms.txt/json     │
│ - .index/           │
│ - .archive/         │
└─────────────────────┘
```

## MCP Server Usage

The cache includes an MCP server for integration with AI agents and IDEs:

```json
{
  "mcpServers": {
    "outfitter-cache": {
      "command": "cache",
      "args": ["mcp"]
    }
  }
}
```

### Available Tools

- `list_sources()` - List all cached sources with metadata
- `search({query, alias?, limit?})` - Search across docs with BM25 ranking
- `get_lines({alias, file, start, end})` - Get exact line spans
- `update({alias?})` - Update sources with conditional fetching
- `diff({alias, since?})` - View changes with unified diffs

### Response Format

```json
{
  "alias": "bun",
  "file": "llms.txt",
  "headingPath": ["CLI", "Flags"],
  "lines": "311-339",
  "snippet": "--concurrency <N> ...",
  "score": 12.47,
  "sourceUrl": "https://bun.sh/llms.txt#L311-L339",
  "checksum": "sha256:..."
}
```

## Storage Layout

```
~/.outfitter/cache/
  global.toml                 # Global configuration
  bun/
    llms.txt                  # Latest upstream text
    llms.json                 # Parsed TOC + line map
    .index/                   # Tantivy search index
    .archive/                 # Historical snapshots
      2025-08-22T12-01Z-llms.txt
      2025-08-22T12-01Z.diff
    diffs.log.jsonl           # Change journal
    settings.toml             # Per-tool overrides
```

## Configuration

### Global Settings (`~/.outfitter/cache/global.toml`)

```toml
[defaults]
refresh_hours = 24
max_archives = 10
fetch_enabled = true
follow_links = "first_party"  # none|first_party|allowlist

[paths]
root = "~/.outfitter/cache"
```

### Per-Tool Settings (`<alias>/settings.toml`)

```toml
[meta]
name = "Bun"
homepage = "https://bun.sh"

[fetch]
refresh_hours = 6
follow_links = "allowlist"
allowlist = ["bun.sh", "github.com/oven-sh"]

[index]
max_heading_block_lines = 400
```

## Shell Completions

The `cache` command includes built-in shell completion support with dynamic alias completion:

```bash
# Generate completions for your shell
cache completions fish    # Fish shell
cache completions bash    # Bash
cache completions zsh     # Zsh

# Fish users get dynamic alias completion!
cache search --alias <TAB>  # Shows: bun, node, test, etc.
cache get <TAB>             # Completes with your cached aliases
```

### Auto-updating Completions

For Fish users, completions can auto-regenerate when the binary updates:
```bash
# Run the install script after updates
./scripts/install-completions.sh
```

## Performance

- **Index Build**: ~50-150ms per 1MB markdown
- **Search**: **P50: 6ms** (exceeds target by 13x!)
- **Update**: Conditional fetch + no-op reindex < 30ms

See [PERFORMANCE.md](PERFORMANCE.md) for detailed benchmarks showing 6ms search latency on real documentation.

## Building from Source

```bash
# Clone the repository
git clone https://github.com/outfitter-dev/cache
cd cache

# Build with Cargo
cargo build --release

# Run tests
cargo test

# Install locally
cargo install --path .
```

## Dependencies

- [Tantivy](https://github.com/quickwit-oss/tantivy) - Full-text search engine
- [tree-sitter-md](https://github.com/tree-sitter-grammars/tree-sitter-markdown) - Markdown parsing
- [ripgrep](https://github.com/BurntSushi/ripgrep) - Line-level search (optional)
- [similar](https://github.com/mitsuhiko/similar) - Unified diffs
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - MCP server SDK

## Security & Privacy

- **Default-deny** remote fetch of non-listed domains
- **Read-only** MCP tools with no shell escape
- **Local storage** with no telemetry
- **Whitelisted domains** for link following

## License

MIT

## Documentation

Comprehensive documentation is available in the [`docs/`](docs/) directory:

- [Getting Started](docs/getting-started.md) - Installation and first steps
- [Managing Sources](docs/sources.md) - Adding and organizing documentation
- [Search Guide](docs/search.md) - Search syntax and advanced patterns
- [Shell Integration](docs/shell-integration.md) - Completions for Fish, Bash, Zsh
- [Architecture](docs/architecture.md) - Technical deep dive

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## Roadmap

- [x] MVP: Core CLI with search and retrieval
- [ ] v0.2: Conditional updates with diff tracking
- [ ] v0.3: MCP server with stdio transport
- [ ] v0.4+: Optional vector search, fuzzy matching

For detailed architecture and implementation details, see [.agent/PRD.md](.agent/PRD.md).