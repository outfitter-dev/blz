# Changelog

All notable changes to blz will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-08-27

### Added

- **Core Features**
  - Fast local search with millisecond latency (6ms typical)
  - Full-text search powered by Tantivy with BM25 ranking
  - Exact line citations for every search result
  - Support for multiple documentation sources with aliases
  - Offline-first design - works without internet connection
  - Tree-sitter based markdown parsing for accurate structure extraction
  - Archive support for historical documentation snapshots

- **CLI Commands**
  - `add <alias> <url>` - Add and index a documentation source
  - `search <query> [--alias <ALIAS>]` - Search across indexed documentation
  - `list` - List all indexed sources with metadata
  - `remove <alias>` - Remove a source and its index
  - `update [alias]` - Update sources (not yet implemented in v0.1)
  - `completions <shell>` - Generate shell completions (fish, bash, zsh, elvish, powershell)

- **Storage & Configuration**
  - Platform-specific data directories using OS conventions
  - Per-source data organization with metadata tracking
  - Global configuration at platform-specific config directories
  - Automatic migration from old `~/.outfitter/cache/` paths

- **Performance**
  - Search latency: P50 < 10ms
  - Indexing speed: < 150ms per MB of markdown
  - Concurrent search across multiple sources
  - Memory-efficient operation with streaming where possible

- **MCP Server Integration** (planned; not included in v0.1)
  - JSON-RPC 2.0 compliant server implementation
  - Structured output formats (JSON, NDJSON, Text)
  - Stable response shapes for programmatic consumption

### Changed

- Storage paths migrated from `~/.outfitter/cache/` to `~/.outfitter/blz/`
- User agent updated from `outfitter-cache` to `outfitter-blz`

### Known Limitations

- **No incremental indexing/updates**: Full re-index required on updates (ETag/Last-Modified headers are used for conditional fetching)
- **`diff` command disabled**: Currently experimental, will be enabled in future release
- **Single file format**: Only supports llms.txt Markdown format
- **No search history**: Search queries are not persisted
- **Limited query syntax**: Basic text and field queries only

### Platform Support

- **macOS**: Full support with native ARM64 and Intel builds
- **Linux**: Full support (x86_64, ARM64)
- **Windows**: Full support (requires PowerShell for completions)

### Storage Paths by Platform

- **macOS**: `~/Library/Application Support/dev.outfitter.blz/` (data and config)
- **Linux**: `~/.local/share/outfitter/blz/` (data), `~/.config/outfitter/blz/` (config)  
- **Windows**: `%APPDATA%\outfitter\blz\data\` (data), `%APPDATA%\outfitter\blz\config\` (config)

### Dependencies

- Rust 1.75+ (for building from source)
- No runtime dependencies - single static binary

[0.1.0]: https://github.com/outfitter-dev/blz/releases/tag/v0.1.0