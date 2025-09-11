# blz ※

> **blaze** */bleɪz/* (verb, noun)
>
> 1. **verb** – Move or proceed at high speed; achieve something rapidly
> 2. **noun** – A trail marker, typically painted on trees with specific colors and patterns; a mark to guide explorers on their journey
> 3. **abbr.** – `blz` – A local-first search tool that indexes llms.txt documentation for instant, line-accurate retrieval

---

## What is `blz`?

A Rust + Tantivy-based CLI tool that downloads, parses, and indexes `llms.txt` files locally to enable fast documentation search with line-accurate retrieval.

## Usage For AI Agents

- Quick primer in your terminal: `blz instruct`
- Detailed instructions you can copy into CLAUDE.md or AGENTS.md: see `.agents/instructions/use-blz.md`
  - You can inline it directly or @‑mention it from your agent’s rules file

Typical agent flow:

```bash
# Ensure sources exist (add non-interactively)
blz add react https://react.dev/llms-full.txt -y

# Search and get exact lines
blz "react hooks" -o json | jq -r '.[0] | "\(.alias) \(.lines)"' | \
  xargs -n2 blz get --context 3
```

### Wait, what's `llms.txt`?

[`llms.txt`](https://llmstxt.org/) is a simple Markdown standard for making documentation accessible to AI agents. `llms-full.txt` is an expanded version that typically includes all of the documentation for a given project.

- Why they're great:
  - Comprehensive project documentation that's kept up to date (usually)
  - Single file in a standardized format makes for easy retrieval and indexing
- What's not great:
  - They're **huge**.
    - Example: the [Model Context Protocol llms-full.txt](https://modelcontextprotocol.io/llms-full.txt) is nearly 12,000 lines long, and is over 200,000 tokens, which coincidentally was Claude 3.7 Sonnet's token limit.
  - They can change often (which is a good thing), so if you want to download them as reference, keeping them up to date is a pain.

### Why `blz`?

`llms.txt` files are great, but they're not immediately useful for coding agents as a source for documentation. Context limits alone are enough to make them impractical. Using MCP servers to get docs is the gold-standard today, but they can often return lots of token-heavy results, which isn't ideal for context management in agents. So that's where `blz` comes in:

```bash
# Add Bun's llms.txt to blz
# (blz will default to try to get llms-full.txt if it's available)
blz add bun https://bun.sh/llms.txt

# Search for "bun:sqlite"
blz search "bun:sqlite"

# Get exact lines
blz get bun --lines 1853-1862
blz get bun --lines 34366 --context 3   # Adds 3 lines of context from either side
```

- Downloading and indexing is fast (often far less than 1 second)
- Searching is faster (10ms or less typically)
- Retrieving exact lines is fastest

See [docs/performance.md](docs/performance.md) for detailed benchmarks and methodology.

## Features

- **Fast Search**: 6ms typical search latency (yes, milliseconds)
- **Line-Accurate**: Returns exact `file#L120-L142` spans with heading context
- **Smart Sync**: Conditional fetches with ETag/If-None-Match to minimize bandwidth
- **Robust Parsing**: Handles imperfect `llms.txt` gracefully, always produces useful structure
- **Deterministic Search**: BM25 ranking with Tantivy (vectors optional, off by default)
- **Change Tracking**: Coming in v0.2 – diff journal with unified diffs and changed sections
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

# Install completions (Elvish)
blz completions elvish > ~/.local/share/elvish/lib/blz.elv
```

## Quick Start

```bash
# Add a source
blz add bun https://bun.sh/llms.txt

# Search across docs
blz search "test runner"
blz search "concurrency" --alias bun

# Get exact lines
blz get bun --lines 120-142
blz get bun --lines 120-142 --context 3

# List all sources
blz list

# Update sources (coming in v0.2)
#   blz update bun
#   blz update --all

# View changes (coming soon in v0.2)
# blz diff bun --since "2025-08-20"   # YYYY-MM-DD (RFC 3339 timestamps also supported)
```

## Architecture

```
┌────────────────────────────────────────────┐
│ blz CLI (MCP soon)                         │
└────────────────────────────────────────────┘
           │
┌──────────▼──────────────┐    ┌─────────────┐
│ blz Core  (Rust)        │◄──►│  blz Index  │
│ ├ Fetcher (ETag)        │    │  (Tantivy)  │
│ ├ Parser  (tree-sitter) │    └─────────────┘
│ ├ Search  (BM25)        │
│ └ *Diff   (v0.2, soon)  │
└──────────┬──────────────┘
           │
┌──────────▼──────────────┐
│ blz local cache         │
│ ├ llms.txt/json         │
│ ├ .index/               │
│ └ .archive/             │
└─────────────────────────┘
```

## IDE Agent Integration

### Direct CLI Usage (Recommended)

IDE agents can run `blz` commands directly for millisecond responses:

```bash
# Search for documentation
blz search "test runner" --alias bun --output json

# Get exact line ranges
blz get bun --lines 423-445

# List all indexed sources
blz list --output json | jq '.sources | length'
```

The JSON output is designed for easy parsing by agents:

```json
{
  "alias": "bun",
  "file": "llms.txt",
  "headingPath": ["CLI", "Flags"],
  "lines": "311-339",
  "snippet": "--concurrency<N> ...",
  "score": 12.47,
  "sourceUrl": "https://bun.sh/llms.txt#L311-L339",
  "checksum": "sha256:..."
}
```

### MCP Server (Coming Soon)

For deeper integration, an MCP server interface is in development that will expose tools like `search`, `get`, `update`, and `diff` (MCP protocol 2024-11-05) via stdio for Claude Code, Cursor MCP, and other MCP-compatible hosts.

## Storage Layout

Example showing Linux default data paths. See Configuration section for config locations.

```
~/.local/share/dev.outfitter.blz/
  bun/
    llms.txt                         # Latest upstream text
    llms.json                        # Parsed TOC + line map
    .index/                          # Tantivy search index
    .archive/                        # Historical snapshots
      2025-08-22T12-01-07Z-llms.txt
      2025-08-22T12-01-07Z.diff      # unified diff vs previous snapshot
    settings.toml                    # Per-source configuration
```

## Configuration

Config file discovery order:

- `$XDG_CONFIG_HOME/blz/config.toml` or `~/.config/blz/config.toml`
- Fallback: `~/.blz/config.toml`
- Explicit override (optional): `--config <FILE>` or env `BLZ_CONFIG`; or `--config-dir <DIR>` / env `BLZ_CONFIG_DIR` to use `<DIR>/config.toml`
- Optional overlay: `config.local.toml` in the same directory overrides keys

Environment overrides (per-key):

- `BLZ_REFRESH_HOURS` (u32)
- `BLZ_MAX_ARCHIVES` (usize)
- `BLZ_FETCH_ENABLED` (`true`/`false`/`1`/`0`)
- `BLZ_FOLLOW_LINKS` (`none` | `first_party` | `allowlist`)
- `BLZ_ALLOWLIST` (comma-separated domains)
- `BLZ_ROOT` (path)

### Global Settings

```toml
[defaults]
refresh_hours = 24
max_archives = 10
fetch_enabled = true
follow_links = "first_party"  # none|first_party|allowlist

[paths]
# Platform-specific defaults (examples):
# Linux (XDG):     ~/.local/share/dev.outfitter.blz/
# macOS (AppData): ~/Library/Application Support/dev.outfitter.blz/
# Windows:         %APPDATA%\dev.outfitter.blz\
```

### Per-Source Settings (`<alias>/settings.toml`)

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
blz completions elvish  # Elvish

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
# Or use standard cargo test as fallback:
#   cargo test --workspace

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
- [Shell Integration](docs/shell-integration/README.md) - Completions for Fish, Bash, Zsh, Elvish
- [Architecture](docs/architecture.md) - Technical deep dive

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## Roadmap

- [x] v0.1: Core CLI with search and retrieval (MVP)
- [ ] v0.2: Diff tracking and change journal
- [ ] v0.3: MCP server with stdio transport
- [ ] v0.4+: Optional vector search, fuzzy matching

For detailed architecture and implementation details, see [docs/architecture.md](docs/architecture.md).
