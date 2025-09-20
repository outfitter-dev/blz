# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
- **`blz instruct` command**: Append live CLI documentation to agent instructions
- **Improved logging**: All logs go to stderr, keeping stdout clean for JSON/NDJSON output
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

[0.1.6]: https://github.com/outfitter-dev/blz/releases/tag/v0.1.6
[0.1.5]: https://github.com/outfitter-dev/blz/releases/tag/v0.1.5
[0.1.7]: https://github.com/outfitter-dev/blz/releases/tag/v0.1.7
[0.2.0]: https://github.com/outfitter-dev/blz/releases/tag/v0.2.0
[0.2.1]: https://github.com/outfitter-dev/blz/releases/tag/v0.2.1
[0.2.2]: https://github.com/outfitter-dev/blz/releases/tag/v0.2.2
[0.2.4]: https://github.com/outfitter-dev/blz/releases/tag/v0.2.4
[0.3.0]: https://github.com/outfitter-dev/blz/releases/tag/v0.3.0
