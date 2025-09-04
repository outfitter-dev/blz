# blz

> **blaze** */bleɪz/* (verb, noun)
>
> 1. **verb** – Move or proceed at high speed; achieve something rapidly
> 2. **noun** – A trail marker, typically painted on trees with specific colors and patterns; a mark to guide explorers on their journey
> 3. **abbr.** – `blz` – A local-first search tool that indexes llms.txt documentation for instant, line-accurate retrieval

---

Local-first search for `llms.txt` ecosystems. Returns exact line citations in milliseconds. Built with Rust + Tantivy for deterministic, fast searches that work offline.

## Features

- **Fast Search**: 6ms typical search latency (yes, milliseconds)
- **Line-Accurate**: Returns exact `file#L120-L142` spans with heading context
- **Smart Sync**: Conditional fetches with ETag/If-None-Match to minimize bandwidth
- **Efficient Updates**: Archives previous versions before updating; checks ETag/Last-Modified headers
- **Parallel Search**: Searches multiple sources concurrently for comprehensive results  
- **Robust Parsing**: Handles imperfect `llms.txt` gracefully, always produces useful structure
- **Deterministic Search**: BM25 ranking with Tantivy (vectors optional, off by default)
- **Version Archiving**: Automatic backup of previous versions before updates
- **Direct CLI Integration**: IDE agents run commands directly for instant results
- **MCP Server** (coming soon): stdio-based integration via official Rust SDK

## Installation

### From Source

```bash
# Clone and install
git clone https://github.com/outfitter-dev/blz
cd blz
cargo install --path crates/blz-cli

# Or install directly from GitHub
cargo install --git https://github.com/outfitter-dev/blz --branch main blz-cli
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
blz list

# Update a single source (checks for changes with ETag)
blz update bun

# Update all sources at once
blz update --all
```

## Architecture

```
┌─────────────────────┐
│ CLI (MCP soon)      │
└──────────┬──────────┘
           │
┌──────────▼──────────┐      ┌─────────────────┐
│ Core Engine (Rust)  │◄────►│ Tantivy Index   │
│ - Fetcher (ETag)    │      └─────────────────┘
│ - Parser (tree-sitter)
│ - Search (BM25)     │
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│ Storage             │
│ ~/.outfitter/blz/ │
│ - llms.txt/json     │
│ - .index/           │
│ - .archive/         │
└─────────────────────┘
```

## IDE Agent Integration

### Direct CLI Usage (Recommended)

IDE agents can run `blz` commands directly for millisecond responses:

```bash
# Search for documentation
blz search "test runner" --alias bun --format json

# Get exact line ranges
blz get bun --lines 423-445

# List all indexed sources
blz list --format json
```

The JSON output is designed for easy parsing by agents:

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

### MCP Server (Coming Soon)

For deeper integration, an MCP server interface is in development that will expose tools like `search`, `get_lines`, and `update` via stdio for Claude Code, Cursor MCP, and other MCP-compatible hosts.

## Storage Layout

```
~/.outfitter/blz/
  global.toml                 # Global configuration
  bun/
    llms.txt                  # Latest upstream text
    llms.json                 # Parsed TOC + line map
    .index/                   # Tantivy search index
    .archive/                 # Historical snapshots
      2025-08-22T12-01Z-llms.txt
      2025-08-22T12-01Z-llms.json
    settings.toml             # Per-tool overrides
```

## Configuration

### Global Settings (`~/.outfitter/blz/global.toml`)

```toml
[defaults]
refresh_hours = 24
max_archives = 10
fetch_enabled = true
follow_links = "first_party"  # none|first_party|allowlist

[paths]
root = "~/.outfitter/blz"
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
blz <TAB>                 # Shows your indexed aliases
blz get <TAB>             # Completes with your indexed aliases
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
git clone https://github.com/outfitter-dev/blz
cd blz

# Build with Cargo
cargo build --release

# Run tests
# Fast local test run (nextest)
# If you don't have nextest installed:
#   cargo install cargo-nextest
cargo nextest run --workspace
# Fallback:
# cargo test --workspace

# Install locally
cargo install --path .
```

## Dependencies

- [Tantivy](https://github.com/quickwit-oss/tantivy) - Full-text search engine
- [tree-sitter-md](https://github.com/tree-sitter-grammars/tree-sitter-markdown) - Markdown parsing
- [ripgrep](https://github.com/BurntSushi/ripgrep) - Line-level search (optional)
- [similar](https://github.com/mitsuhiko/similar) - Unified diffs
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - MCP server SDK (coming soon)

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
- [x] v0.1: Conditional updates with ETag/Last-Modified, archive support, parallel search
- [ ] v0.2: Full diff tracking and change journal
- [ ] v0.3: MCP server with stdio transport
- [ ] v0.4+: Optional vector search, fuzzy matching

For detailed architecture and implementation details, see [docs/architecture.md](docs/architecture.md).
