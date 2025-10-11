# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.1] - 2025-10-10

### Added
- **Fuzzy-matched source warnings**: When searching with a non-existent source filter, `blz` now suggests similar source names
  - Shows top 3 closest matches sorted by similarity score
  - Warnings print to stderr only (preserves JSON output on stdout)
  - Respects quiet mode (`-q` flag) to suppress warnings
  - Exit code remains 0 for backward compatibility
- **Bundled documentation hub**: New `blz docs` command with subcommands for embedded documentation
  - `blz docs search`: Search the bundled blz-docs source without touching other aliases
  - `blz docs sync`: Sync or resync embedded documentation files and index
  - `blz docs overview`: Quick-start guide for humans and agents
  - `blz docs cat`: Print entire bundled llms-full.txt to stdout
  - `blz docs export`: Export CLI docs in markdown or JSON (replaces old `blz docs --format`)
- **Internal documentation source**: `blz-docs` alias (also `@blz`) ships with the binary
  - Hidden from default search with `internal` tag
  - Auto-syncs on first use or when version changes
  - Full CLI reference and user guide embedded in the binary
- **Linear integration rules**: Added `.agents/rules/LINEAR.md` for Linear project management workflow
- **Configurable snippet length** ([BLZ-117](https://linear.app/outfitter/issue/BLZ-117)): New `--max-chars` flag controls snippet length
  - Default: 200 characters (increased from ~100)
  - Range: 50-1000 characters with automatic clamping
  - Environment variable: `BLZ_MAX_CHARS`
  - Counts total characters including newlines, not per-line column width
- **Backward pagination** ([BLZ-137](https://linear.app/outfitter/issue/BLZ-137)): New `--previous` flag complements `--next` for pagination
  - Navigate backward through search results without repeating queries
  - Stateful pagination: `--next` (forward), `--previous` (backward), `--last` (jump to end)
  - Error handling: "Already on first page" when at page 1
  - Maintains query and source context automatically
- **Grep-style context flags** ([BLZ-132](https://linear.app/outfitter/issue/BLZ-132)): Industry-standard short options for context
  - `-C <N>`: Print N lines of context (both before and after)
  - `-A <N>`: Print N lines after each match
  - `-B <N>`: Print N lines before each match
  - Flags can be combined (e.g., `-C5 -A2` merges to max values)
  - Legacy `-c` flag maintained for backward compatibility
- **Read-only command enhancements and format shortcuts** ([BLZ-123](https://linear.app/outfitter/issue/BLZ-123)): Consistent, ergonomic output controls across commands
  - Format aliases: `--json`, `--jsonl`, `--text`, and `--raw` map to their respective `--format` values
  - `--limit` flag added to `list`, `stats`, `lookup`, and `anchor list`
  - All read-only commands now support the new format shortcuts
  - JSON output is pure (no mixed stderr/stdout) for clean piping
- **Language filtering** ([BLZ-111](https://linear.app/outfitter/issue/BLZ-111)): Automatic filtering of non-English documentation
  - URL-based locale detection (path markers: `/de/`, `/ja/`, subdomain patterns)
  - 60-90% bandwidth and storage reduction for multilingual sources
  - Opt-out with `--no-language-filter` flag
  - Zero dependencies, <1μs per URL performance
- **Section expansion improvements** ([BLZ-115](https://linear.app/outfitter/issue/BLZ-115)): `--context all` now consistent
  - Single line queries now expand to full heading blocks (previously only ranges worked)
  - Behavior matches search command expectations
  - Legacy `--block` flag maintained as alias
- **Prompt enhancements** ([BLZ-116](https://linear.app/outfitter/issue/BLZ-116)): New "Try this" section in search prompt
  - 5 practical examples with explanations
  - Emphasizes one-shot retrieval workflow with `--context all`
  - Shows optimal snippet sizing, pagination navigation, and noise reduction techniques

### Changed
- `blz docs` command now uses subcommands instead of single `--format` flag
  - Old `blz docs --format json` still works for backward compatibility
  - New preferred syntax: `blz docs export --format json`
- **Short flag consistency** ([BLZ-113](https://linear.app/outfitter/issue/BLZ-113)): Audited and fixed across all commands
  - `-s` for `--source` works universally where defined
  - `-f` for `--format` available on all commands
  - `-C/-c` for `--context` (uppercase is new standard, lowercase maintained for compatibility)
  - `-l` for `--lines` on get command
  - `-n` for `--limit` on commands with pagination
  - Help text consistently shows all available short flags

### Deprecated
- **`--snippet-lines` flag** ([BLZ-133](https://linear.app/outfitter/issue/BLZ-133)): Use `--max-chars` instead
  - Hidden from help output
  - Still functional for backward compatibility
  - Will be removed in future major version
  - `BLZ_SNIPPET_LINES` environment variable also deprecated

### Fixed
- **Context flag parsing**: `-C`, `-A`, and `-B` now parse correctly with concatenated values (e.g., `-C5`)
- **Single-line block expansion**: `blz get <source>:<line> --context all` now expands to full section

### Internal
- Added `DocsCommands` enum for `blz docs` subcommands
- Added `DocsSearchArgs` for bundled docs search functionality
- New `docs_bundle.rs` module for managing embedded documentation
- Added `ContextMode` enum with `All`, `Symmetric`, and `Asymmetric` variants
- Added `merge_context_flags` function for grep-style flag merging
- Comprehensive test suites for pagination (`--next`, `--previous`), context flags, and format shortcuts

## [1.0.0-beta.1] - 2025-10-03

### Breaking Changes
- Removed dual-flavor system (llms.txt vs llms-full.txt). BLZ now intelligently auto-prefers llms-full.txt when available.
- Removed backwards compatibility for v0.4.x cache format. Use `blz clear --force` to migrate from older versions.

### Added
- **One-line installation**: New install script with SHA-256 verification and platform detection
  - Download via: `curl -fsSL https://blz.run/install.sh | sh`
  - Support for macOS (x64, arm64) and Linux (x64)
  - SHA-256 checksum verification (use `--skip-check` to bypass)
  - Custom install directory with `--dir` flag
  - `--dry-run` mode for testing
- **Clipboard support**: Copy search results directly with `--copy` flag (OSC 52 escape sequences)
- **Search history**: New `blz history` command to view and manage persistent search history
  - History filtering by date, source, and query
  - Configurable retention (default: 1000 entries)
  - Clean command with date-based pruning
- **Source insights**: New commands for better visibility
  - `blz stats`: Cache statistics including source count, storage size, and index metrics
  - `blz info <source>`: Detailed source information with metadata
  - `blz validate`: Verify source integrity with URL accessibility, checksum validation, and staleness detection
  - `blz doctor`: Comprehensive health checks with auto-fix capability for cache and sources
- **Batch operations**: Add multiple sources via TOML manifest files
  - Template at `registry/templates/batch-manifest.example.toml`
  - Supports aliases, tags, npm/github mappings
  - Parallel indexing for faster setup
- **Rich metadata**: Source descriptors with name, description, and category
  - `blz list --details`: View extended source information
  - Auto-populated from registry or customizable
  - Persisted in `.blz/descriptor.toml` per source
- **Enhanced search**:
  - Multi-source filtering with `--source` flag (comma-separated)
  - Improved snippet extraction with configurable context lines
  - Search history integration with `.blz_history` replay

### Changed
- **URL intelligence**: Automatically prefers llms-full.txt when available (no manual configuration needed)
- **Simplified CLI**: Removed confusing `--flavor` flags from all commands
- **Better defaults**: Intelligent fallback to llms.txt if llms-full.txt unavailable
- **Descriptor defaults**: Sources added without explicit metadata get sensible auto-generated values

### Fixed
- **Exit codes**: Commands now properly return exit code 1 on errors for better scripting support
  - `blz get` with non-existent source now exits with code 1
  - `blz remove` with non-existent source now exits with code 1
  - `blz get` with out-of-range lines now exits with code 1 and provides helpful error message
- 40+ code quality improvements from strict clippy enforcement
- Redundant clones and inefficient Option handling eliminated
- Float precision warnings properly annotated
- All `.unwrap()` usage replaced with proper error handling
- Format string optimizations throughout CLI
- Documentation URL formatting fixed

### Performance
- Optimized format! string usage in hot paths
- Reduced unnecessary allocations in search results formatting
- Improved clipboard copy performance with write! macro

### Developer Experience
- All tests passing (224/224)
- Zero clippy warnings with strict configuration
- Clean release builds (~42s)
- Comprehensive v1.0-beta release checklist

## [0.5.0] - 2025-10-02

### Breaking Changes
- Removed backwards compatibility for v0.4.x cache format. Users upgrading from v0.4.x will need to clear their cache with `blz clear --force` and re-add sources. The CLI will detect old cache format and display helpful error message with migration instructions.

### Added
- New `blz clear` command to remove all cached sources and indices.
  - `--force` flag to skip confirmation prompt for non-interactive use.
  - Helpful error detection when old v0.4.x cache format is found.
- New `upgrade` command to migrate sources from llms.txt to llms-full.txt (#234).
- Automatic preference for llms-full.txt when available via `FORCE_PREFER_FULL` feature flag (#234).
- Comprehensive test suite for automatic llms-full preference behavior (5 new tests) (#234).
- CLI refactoring with testable seams for `clear`, `list`, `remove`, and `update` commands.

### Changed
- **XDG-compliant paths**: Both config and data now respect XDG Base Directory specification:
  - Config: `$XDG_CONFIG_HOME/blz/` (if set) or `~/.blz/` (fallback)
  - Data: `$XDG_DATA_HOME/blz/` (if set) or `~/.blz/` (fallback)
  - Environment overrides: `BLZ_GLOBAL_CONFIG_DIR` and `BLZ_DATA_DIR`
- **Reorganized data directory**: Source directories now organized under `sources/` subdirectory for cleaner structure.
- **Renamed state file**: `blz.json` renamed to `data.json` to distinguish runtime state from configuration files.
- Simplified flavor selection to automatically prefer llms-full.txt without user configuration (#234).
- Hidden `--flavor` flags across add, search, and update commands for cleaner user experience (#234).
- Updated `--yes` flag help text to be flavor-agnostic: "Skip confirmation prompts (non-interactive mode)" (#234).
- Removed `BLZ_PREFER_LLMS_FULL` environment variable (automatic preference replaces manual configuration) (#234).
- Removed custom LlmsJson deserializer for v0.4.x format (141 lines removed).

### Fixed
- Restored metadata alias propagation for update and add flows.
- Addressed security and portability issues identified in code review.
- Normalized heading counts with accurate recursive counting.
- Parser now filters out placeholder "404" pages.

### Documentation
- Updated 11 documentation files to reflect flavor simplification and automatic llms-full preference (#234).
- Added comprehensive `docs/cli/commands.md#upgrade` documentation (#234).
- Fixed 5 broken internal links in documentation index (#234).
- Added `SCRATCHPAD.md` for tracking session work and progress.

## [0.4.1] - 2025-09-29

### Added
- Search CLI pagination with history-aware `--next`/`--last`, improved JSON metadata, and stricter batch `get` span handling (#229).

### Changed
- JSON output now always includes both rounded `score` and `scorePercentage`, plus compatibility fields mirrored for downstream tooling (#229).
- Pagination flow now treats `--limit` as optional, enforces consistent page size when continuing with `--next`, and surfaces friendlier tips for text output (#229).
- Release automation can be manually dispatched without a full publish run (#228).

### Fixed
- Search history writes use fsync + atomic rename with advisory locking to avoid corruption when multiple CLI processes exit simultaneously (#229).

## [0.4.0] - 2025-09-26

### Changed
- Unified flavor resolution across `list`, `search`, and `get` so CLI commands respect stored preferences consistently (#227).
- Relaxed release coverage requirements to streamline the automated publish pipeline (#226).

## [0.3.3] - 2025-09-25

### Added
- Enhanced phrase search ergonomics, including `--source` flag migration, better highlighting, and improved snippet ordering (#224).

### Fixed
- Snippet extraction now handles quoted phrases without truncation (#225).

### CI
- Hardened the coverage cache cleanup to prevent flaky report uploads (#223).

## [0.3.2] - 2025-09-24

### Added
- SHA256 parameter support for the Homebrew workflow and expanded release automation documentation (#213, #217).

### Changed
- CLI shorthand parsing now dynamically discovers known subcommands and respects hidden entries (#215).
- Release workflows consolidated with parameterized modes and rewritten semver tooling in Rust for deterministic versioning (#218, #221).

### Fixed
- DotSlash generation and Homebrew publishing now retry transient errors to stabilize CI (#214, #212).

## [0.3.1] - 2025-09-24

### Added
- Linux binaries are now published alongside macOS and Windows in the Homebrew formula (#204).

### Fixed
- Search shorthand parsing correctly handles flags and hidden subcommands without misrouting queries (#203).

## [0.3.0] - 2025-09-21

### Added
- Dual-flavor ingestion for both `llms.txt` and `llms-full.txt`, including automatic
  detection, interactive selection, and flavor-aware indexing.
- CLI enhancements for the v0.3 release (refined help output, quiet mode polish,
  and centralized format flag handling).
- Release automation updates with coverage notes and BLZ stylization guidance for
  agent integrations.

### Changed
- Workspace crates bumped to version 0.3.0 to align with the release artifacts.
- Tests and documentation refreshed for the v0.3 feature set, including expanded
  integration coverage.

## [0.2.4] - 2025-09-18

### Fixed
- Added raw platform-specific binaries to GitHub release assets so npm postinstall can download them directly (was failing with 404s on v0.2.1).

### Changed
- Publish workflow now extracts archives while flattening artifacts to upload both compressed bundles and uncompressed binaries.

## [0.2.2] - 2025-09-17

### Changed
- Bumped workspace and npm packages to version 0.2.2 in preparation for the next patch release train.

### Fixed
- Hardened the publish workflow’s artifact flatten step by downloading into per-target directories, deep-searching for archives, and safely replacing existing files when identical assets already exist.

## [0.2.1] - 2025-09-17

### Changed
- Automated releases via label-driven workflows that build cross-platform artifacts, upload them, and publish npm/crates/Homebrew in sequence.
- Added asset readiness guards for the Homebrew job and tightened release undraft conditions to avoid incomplete releases.
- Cached `cargo-edit` in CI and documented local `act` rehearsals for release workflows.

### Fixed
- Windows npm postinstall now imports `package.json` via URL (no `ERR_UNSUPPORTED_ESM_URL_SCHEME`) and the package requires Node ≥ 18.20.0.

## [0.2.0] - 2025-09-15

### Added
- **`blz diff` command**: Compare current and archived versions of sources to see what's changed
- **`blz alias` command**: Manage source aliases with `add` and `rm` subcommands for better organization
- **`blz docs` command**: Generate CLI documentation in markdown or JSON format
- **Targeted cache invalidation**: Optimized search cache that invalidates only affected aliases on updates
- **Anchors support**: Parse and index anchor links from llms.txt files for better navigation
- **HEAD preflight checks**: Verify remote availability and size before downloads with retry logic
- **Windowed segmentation fallback**: Handle large documents that exceed indexing limits gracefully
- **Dynamic shell completions**: Enhanced completion support with metadata-aware suggestions
- **Flavor policy for updates**: Control update behavior with `--flavor` (auto, full, txt, current)

### Changed
- **JSON output improvements**: Consistent camelCase field names, added sourceUrl and checksum fields
- **CLI improvements**: Added `-s` as short alias for `--source`, improved error messages
- **Documentation restructure**: Split CLI docs into organized sections under `docs/cli/`
- **Performance**: Optimized search with granular cache invalidation per alias

### Fixed
- **JSON stability**: Proper stderr/stdout separation for clean JSON output
- **Panic handling**: Graceful handling of broken pipe errors (SIGPIPE)
- **Large document handling**: Fallback to windowed segmentation for documents exceeding limits

### Developer Experience
- **`blz --prompt` flag**: Emit JSON guidance for agents (replaces the old `blz instruct` output)
- **Improved logging**: All logs go to stderr, keeping stdout clean for JSON/JSONL output
- **Better error messages**: More actionable error messages with suggestions

## [0.1.7] - 2025-09-12

### Changed
- Bump workspace and npm versions to 0.1.7 for the next release train.

### CI
- Track Cargo.lock in release workflow and restore `--locked` enforcement.
- Finalize GitHub Release steps and tidy workflow titles.

## [0.1.6] - 2025-01-12

### Added
- Comprehensive CI/CD release workflows with GitHub Actions
- Support for automated releases to multiple platforms (macOS, Linux, Windows)
- Cargo.lock tracking for deterministic builds
- Draft release workflow with proper asset management
- Homebrew tap integration for macOS installations
- npm package publishing support
- Automated crates.io publishing with proper dependency ordering

### Fixed
- Security vulnerability RUSTSEC-2025-0055 in tracing-subscriber (updated to 0.3.20)
- CI/CD workflow robustness with proper error handling
- Draft release asset downloads using authenticated GitHub CLI
- Build reproducibility with --locked flag enforcement

### Changed
- Improved CI/CD workflows with reusable components
- Enhanced cache key strategy including Cargo.lock hash
- Standardized error message formats across workflows
- Better handling of annotated vs lightweight tags

### Security
- Updated tracing-subscriber from 0.3.19 to 0.3.20 to address log poisoning vulnerability

## [0.1.5] - 2025-01-05

### Added
- Initial public release of BLZ
- Fast local search for llms.txt documentation
- Support for multiple documentation sources
- Line-accurate search results with BM25 ranking
- ETag-based conditional fetching for efficiency
- Local filesystem storage with archive support

[1.0.1]: https://github.com/outfitter-dev/blz/releases/tag/v1.0.1
[1.0.0-beta.1]: https://github.com/outfitter-dev/blz/releases/tag/v1.0.0-beta.1
[0.5.0]: https://github.com/outfitter-dev/blz/releases/tag/v0.5.0
[0.4.1]: https://github.com/outfitter-dev/blz/releases/tag/v0.4.1
[0.4.0]: https://github.com/outfitter-dev/blz/releases/tag/v0.4.0
[0.3.3]: https://github.com/outfitter-dev/blz/releases/tag/v0.3.3
[0.3.2]: https://github.com/outfitter-dev/blz/releases/tag/v0.3.2
[0.3.1]: https://github.com/outfitter-dev/blz/releases/tag/v0.3.1
[0.3.0]: https://github.com/outfitter-dev/blz/releases/tag/v0.3.0
[0.2.4]: https://github.com/outfitter-dev/blz/releases/tag/v0.2.4
[0.2.2]: https://github.com/outfitter-dev/blz/releases/tag/v0.2.2
[0.2.1]: https://github.com/outfitter-dev/blz/releases/tag/v0.2.1
[0.2.0]: https://github.com/outfitter-dev/blz/releases/tag/v0.2.0
[0.1.7]: https://github.com/outfitter-dev/blz/releases/tag/v0.1.7
[0.1.6]: https://github.com/outfitter-dev/blz/releases/tag/v0.1.6
[0.1.5]: https://github.com/outfitter-dev/blz/releases/tag/v0.1.5
