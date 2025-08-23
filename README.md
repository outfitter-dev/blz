# @outfitter/blzr

## Why "Blazer"?

Like wilderness explorers who blaze trails by marking paths through uncharted territory, `@outfitter/blzr` ("Blazer") creates clear, marked routes through your documentation landscape. The name captures the dual meaning: blazing trails by establishing clear paths to knowledge, and blazing speed in finding exactly what you need. Just as trail blazers mark trees and rocks to guide future travelers, blzr marks and indexes documentation to guide your coding journey.

A local-first, line-accurate docs blz and MCP server for fast lookups of `llms.txt` ecosystems. Search in milliseconds, cite exact lines, keep diffs, and stay fresh via conditional fetches. Powered by Rust + Tantivy for speed and determinism.

## Features

- **Fast Search**: 6ms typical search latency (yes, milliseconds)
- **Line-Accurate**: Returns exact `file#L120-L142` spans with heading context
- **Smart Sync**: Conditional fetches with ETag/If-None-Match to minimize bandwidth
- **Robust Parsing**: Handles imperfect `llms.txt` gracefully, always produces useful structure
- **Deterministic Search**: BM25 ranking with Tantivy (vectors optional, off by default)
- **Change Tracking**: Built-in diff journal with unified diffs and changed sections
- **MCP Integration**: Official Rust SDK for IDE/agent consumption

## Installation

### From Source
```bash
# Clone and install
git clone https://github.com/outfitter-dev/blzr
cd blzr
cargo install --path crates/blzr-cli

# Or install directly from GitHub
cargo install --git https://github.com/outfitter-dev/blzr --branch main blzr-cli
```

### Shell Setup

#### Fish
```fish
# Add to PATH
set -gx PATH $HOME/.cargo/bin $PATH

# Install completions
blz completions fish > ~/.config/fish/completions/blz.fish
```

#### Bash/Zsh
```bash
# Add to PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Install completions (Bash)
blz completions bash > ~/.local/share/bash-completion/completions/blz

# Install completions (Zsh)
blz completions zsh > ~/.zsh/completions/_blz
```

## Quick Start

```bash
# Add a source
blz add bun https://bun.sh/llms.txt

# Search across docs
blz "test concurrency" bun
# Or: blz bun "test concurrency"

# Get exact lines
blz get bun --lines 120-142
# Or with context: blz get bun -l 120+20

# List all sources
blz sources

# Update all sources (not yet implemented)
blz update --all

# View changes (not yet implemented)
blz diff bun --since "2025-08-20T00:00:00Z"
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
│ ~/.outfitter/blzr/ │
│ - llms.txt/json     │
│ - .index/           │
│ - .archive/         │
└─────────────────────┘
```

## MCP Server Usage

The blz includes an MCP server for integration with AI agents and IDEs:

```json
{
  "mcpServers": {
    "outfitter-blzr": {
      "command": "blz",
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
~/.outfitter/blzr/
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

### Global Settings (`~/.outfitter/blzr/global.toml`)

```toml
[defaults]
refresh_hours = 24
max_archives = 10
fetch_enabled = true
follow_links = "first_party"  # none|first_party|allowlist

[paths]
root = "~/.outfitter/blzr"
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

The `blz` command includes built-in shell completion support with dynamic alias completion:

```bash
# Generate completions for your shell
blz completions fish    # Fish shell
blz completions bash    # Bash
blz completions zsh     # Zsh

# Fish users get dynamic alias completion
blz <TAB>                 # Shows your cached aliases
blz get <TAB>             # Completes with your cached aliases
```

### Auto-updating Completions

For Fish users, completions can auto-regenerate when the binary updates:
```bash
# Run the install script after updates
./scripts/install-completions.sh
```

## Performance

- **Index Build**: ~50-150ms per 1MB markdown
- **Search**: P50 6ms on typical queries
- **Update**: Conditional fetch + no-op reindex < 30ms

See [PERFORMANCE.md](PERFORMANCE.md) for detailed benchmarks and methodology.

## Building from Source

```bash
# Clone the repository
git clone https://github.com/outfitter-dev/blzr
cd blzr

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