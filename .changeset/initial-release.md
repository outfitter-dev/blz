---
"@outfitter/blz": major
---

Initial v0.1.0 release of blz - Fast local search for llms.txt documentation

### Core Features
- Fast local search with millisecond latency (6ms typical)
- Full-text search powered by Tantivy with BM25 ranking
- Exact line citations for every search result
- Support for multiple documentation sources with aliases
- Offline-first design - works without internet connection
- Tree-sitter based markdown parsing for accurate structure extraction
- Archive support for historical documentation snapshots
- Deterministic BM25 ranking (vectors optional, off by default)

### CLI Commands
- `add <alias> <url>` - Add and index a documentation source
- `search <query> [--alias <ALIAS>]` - Search across indexed documentation
- `lookup` - Line-accurate citations with exact file ranges
- `get` - Retrieve specific content
- `list` - List all indexed sources with metadata
- `remove <alias>` - Remove a source and its index
- `update [alias]` - Update sources with conditional fetching (ETag/Last-Modified)
- `completions <shell>` - Generate shell completions (fish, bash, zsh, elvish, powershell)

### Storage & Configuration
- Unified storage path at `~/.outfitter/blz/` with automatic migration
- Platform-specific data directories using OS conventions
- Per-source data organization with metadata tracking
- Global configuration at platform-specific config directories

### Performance
- Search latency: P50 < 10ms, 6ms typical for cached queries
- Indexing speed: < 150ms per MB of markdown
- Concurrent search across multiple sources (up to 8 concurrent)
- Memory-efficient operation with streaming where possible
- Smart pagination and result limiting to prevent over-fetching

### Infrastructure
- Robust error handling with graceful fallbacks
- Comprehensive test coverage for edge cases
- GitHub Actions CI/CD pipeline
- Security auditing with cargo-deny
- Detailed progress reporting for bulk operations