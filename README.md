# BLZ ※

> **blaze** */bleɪz/* (verb, noun)
>
> 1. **verb** – Move or proceed at high speed; achieve something rapidly
> 2. **noun** – A trail marker, typically painted on trees with specific colors and patterns; a mark to guide explorers on their journey
> 3. **abbr.** – BLZ – A local-first search tool that indexes llms.txt documentation for instant, line-accurate retrieval

---

## What is BLZ?

A Rust + Tantivy-based CLI tool that downloads, parses, and indexes `llms.txt` files locally to enable fast documentation search with line-accurate retrieval.

## Quick Start

```bash
# Install (one line)
curl -fsSL https://blz.run/install.sh | sh

# Add Bun's docs
blz add bun https://bun.sh/llms.txt

# Search (results in 6ms)
blz "test runner"

# Pull exact lines (matches the search citation format)
blz get bun:304-324 --json
```

**What you'll see:**

```
✓ Added bun (1,926 headings, 43,150 lines) in 890ms

Search results for 'test runner' (6ms):

1. bun:304-324 (score: 92%)
   📍 Bun Documentation > Guides > Test runner

   ### Test runner
   Bun includes a fast built-in test runner...
```

## Docs

- [Documentation index](docs/README.md) – Overview of every guide, reference, and technical deep dive.
- [Quickstart guide](docs/QUICKSTART.md) – Install BLZ and run your first searches in minutes.
- [Agent playbook](docs/agents/README.md) – Best practices for using BLZ inside AI workflows.
- [Architecture overview](docs/architecture/README.md) – Core components, storage layout, and performance notes.

## What's llms.txt?

[`llms.txt`](https://llmstxt.org/) is a simple Markdown standard for making documentation accessible to AI agents. `llms-full.txt` is an expanded version that includes all documentation for a project.

**Why they're great:**

- Comprehensive documentation that's kept up to date
- Single file in a standardized format makes for easy retrieval and indexing

**The challenge:**

- They're **huge** (12K+ lines, 200K+ tokens)
- Too context-heavy for agents to use directly
- Keeping them up to date is manual work

## Why BLZ?

BLZ indexes [`llms.txt`](https://llmstxt.org/) documentation files locally:

- **6ms search** across locally saved docs (vs. seconds for web requests)
- **Exact line citations** (e.g., `bun:304-324`) for copy-paste accuracy
- **Works offline** after initial download
- **Smart updates** with HTTP caching (only fetches when changed)

### The Problem

Projects publish complete docs as `llms-full.txt` files, but:

- They're massive (12K+ lines, 200K+ tokens)
- Too context-heavy for agents to use directly

But what about MCP servers for searching docs?

- They're great, and we use them too! but...
- Results can take up a lot of an agent's context window
- May require multiple searches to find critical info

### BLZ's Solution

Cache & index `llms.txt` locally → search in ms → retrieve only needed lines

With BLZ, agents can get the docs they need in a fraction of the time, and context.

See [docs/architecture/PERFORMANCE.md](docs/architecture/PERFORMANCE.md) for detailed benchmarks and methodology.

## Features

- **One-line installation**: Install script with SHA-256 verification and platform detection
- **Fast search**: 6ms typical search latency with exact line citations
- **Offline-first**: Works offline after initial download, smart updates with HTTP caching
- **Clipboard support**: Copy search results directly with `--copy` flag
- **Source insights**: Commands for visibility (`blz stats`, `blz info`, `blz history`)
- **Direct CLI integration**: IDE agents run commands directly for instant JSON results
- **MCP server** (coming soon): stdio-based integration via official Rust SDK

## Installation

### Quick Install (macOS/Linux)

```bash
curl -fsSL https://blz.run/install.sh | sh
```

This installs the latest release to `~/.local/bin`. Override the target location with `BLZ_INSTALL_DIR=/path`, or pin a version via `BLZ_VERSION=v0.4.1`. Run `sh install.sh --help` for additional options (e.g., `--dir`, `--version`, `--dry-run`).

### From Source

```bash
# Clone and install
git clone https://github.com/outfitter-dev/blz
cd blz
cargo install --path crates/blz-cli

# Or install directly from GitHub
cargo install --git https://github.com/outfitter-dev/blz --branch main blz-cli

# Optional dev build (installs `blz-dev` only)
./install-dev.sh --root "$HOME/.local/share/blz-dev"
# See docs/development/README.md for full local workflow guidance.
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

## Documentation

Comprehensive documentation is available in the [`docs/`](docs/) directory:

### Getting Started

- [Quick Start](docs/QUICKSTART.md) - Installation and first steps
- [CLI Overview](docs/cli/README.md) - Installation, flags, and binaries
- [How-To Guide](docs/cli/howto.md) - Task-oriented "I want to…" solutions

### CLI Reference

- [Command Reference](docs/cli/commands.md) - Complete command catalog
- [Search Guide](docs/cli/search.md) - Search syntax and advanced patterns
- [Managing Sources](docs/cli/sources.md) - Adding and organizing documentation
- [Configuration](docs/cli/configuration.md) - Global, per-source, and env settings
- [Shell Integration](docs/cli/shell_integration.md) - Completions for Bash, Zsh, Fish, PowerShell, Elvish

### Technical Details

- [Storage Layout](docs/architecture/STORAGE.md) - Directory structure and disk management
- [Architecture](docs/architecture/README.md) - System design and performance
- [Performance](docs/architecture/PERFORMANCE.md) - Benchmarks and optimization

## Usage For AI Agents

- **Quick primer**: `blz --prompt` in your terminal
- **Programmatic CLI docs**: `blz docs export --format json` (legacy: `blz docs --format json`)
- **Detailed instructions**: See `docs/agents/use-blz.md` (copy into CLAUDE.md or AGENTS.md)

### Typical Agent Flow

```bash
# Get caught up with blz's features and capabilities
blz --prompt

# List available sources
blz list --status --json

# Add sources non-interactively
blz add bun https://bun.sh/llms.txt -y

# Search Bun docs and capture the first alias:lines citation
span=$(blz "test runner" --json | jq -r '.results[0] | "\(.alias):\(.lines)"')

# Retrieve the exact line with 5 lines of context on either side
blz get "$span" -C 5 --json

# Need more than one range? Comma-separate them after the alias
blz get bun:41994-42009,42010-42020 --json

# Want the full heading section? Expand with --context all (and cap the output)
blz get bun:41994-42009 --context all --max-lines 80 --json
```

## IDE Agent Integration

### Direct CLI Usage (Recommended)

IDE agents can run `blz` commands directly for millisecond responses:

```bash
# Search for documentation
blz "test runner" -s bun --json

# Get exact line ranges
blz get bun:423-445

# Merge multiple spans for the same source (comma-separated)
blz get bun:41994-42009,42010-42020 --json

# Expand to the entire heading block when the agent needs full prose
blz get bun:41994-42009 --context all --max-lines 80 --json

# List all indexed sources (note: list returns array; search returns object with .results)
blz list --json | jq 'length'
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

For deeper integration, BLZ will expose an MCP server with resources, prompts, and tools for `search`, looking up a `snippet`, and finally `command` to expose the full set of capabilities.

## Shell Completions

The `blz` command includes built-in shell completion support. You can also enable dynamic alias/anchor completion helpers for richer UX.

```bash
# Generate completions for your shell
blz completions fish    # Fish shell
blz completions bash    # Bash
blz completions zsh     # Zsh
blz completions elvish  # Elvish

# Dynamic completions (optional)
#  - Zsh:  source ./scripts/blz-dynamic-completions.zsh (after compinit)
#  - Fish: source ./scripts/blz-dynamic-completions.fish
#  - PS:   . ./scripts/blz-dynamic-completions.ps1

# Example: dynamic alias completion
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

See [PERFORMANCE.md](docs/architecture/PERFORMANCE.md) for detailed benchmarks and methodology.

**Reproducing**: Performance claims based on warm cache, hyperfine benchmarks with 100+ runs. See PERFORMANCE.md for:

- Exact benchmark commands (`hyperfine --warmup 20 --min-runs 100 './target/release/blz search "test" -s bun -f json'`)
- Test environment details (CPU, OS, cache state)
- Representative query set and data sizes

## Building from Source

```bash
git clone https://github.com/outfitter-dev/blz
cd blz
cargo build --release
cargo nextest run --workspace  # or: cargo test --workspace
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

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## Roadmap

- [x] Core CLI with search and retrieval (MVP)
- [ ] Diff tracking and change journal
- [ ] `llms.txt` registry for faster onboarding
- [ ] MCP server
- [ ] Optional vector search, fuzzy matching
