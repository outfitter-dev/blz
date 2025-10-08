# Scratchpad

## 2025-10-08

- Added MCP `linear` server configuration to `.codex/config.toml` pointing at `https://mcp.linear.app/sse` via `npx mcp-remote`.

## 2025-10-07

- Added `blz-dev` secondary binary with isolated profile routing to keep config/data under `blz-dev` directories while sharing core CLI logic.
- Introduced `blz_core::profile` helpers for profile detection; storage/config/store logic now derives paths from profile slug so dev builds avoid clobbering release state.
- Gated `blz-dev` behind optional `dev-profile` feature and created manual `install-dev.sh`; refreshed README docs and reran `cargo check -p blz-cli` / `cargo check -p blz-cli --features dev-profile`.
- Documented the local dev workflow (content now lives in `docs/development/README.md`) and linked it from the development index + README snippet.
- Refined `blz get` docs/help to recommend the `source:lines` shorthand (matches search output) and re-enabled `blz lookup` with a beta footnote plus registry invitation.
- Replaced `blz instruct` with a global `--prompt` flag that emits JSON guidance. Added prompt JSON files alongside each command, wired `prompt.rs` loader, updated docs/tests, and refreshed registry note text.
- Created `hydrate-dev.sh` script to copy production blz data to blz-dev for testing with realistic data; script is XDG-aware and supports selective copying (config-only, sources-only) with dry-run mode.
- Fixed hydration script path detection to always use XDG paths for blz-dev (preventing fallback to dot-directory) while still detecting legacy dot-directory for production blz.
- Installed blz-dev binary and successfully hydrated with production sources (bun, local-test); verified with doctor command showing 2 healthy sources.
- Ran comprehensive testing with blz-tester agent; identified and fixed 3 critical bugs:
  1. Updated help text and documentation to clarify JSON-for-pipes behavior (keeping the smart default, just documenting it properly)
  2. Fixed checksum validation format mismatch (was comparing hex to base64, now both use base64)
  3. Fixed local file validation to use filesystem checks instead of HTTP requests
- All fixes tested and verified: checksums now match, local files validate correctly, help text accurately describes behavior.
- Ran cargo fmt and cargo clippy (all passing).
- Second round of testing revealed checksum format inconsistency: local files stored checksums in hex while remote used base64.
- Fixed local file checksum storage in `crates/blz-cli/src/commands/add.rs:498` to use base64 encoding (matching remote sources).
- Re-added local-test source via manifest, verified both remote (bun) and local (local-test) sources now validate with `checksum_matches: true`.
- Ran comprehensive verification testing with blz-tester agent on blz-dev binary; confirmed `--prompt` flag works excellently across all commands.
- Fixed 3 exit code issues found during testing:
  1. Fixed `get` command to exit with code 1 when source not found (was returning code 0) in `get.rs:183-206`
  2. Fixed `remove` command to exit with code 1 when source not found (was returning code 0) in `remove.rs:164-169`
  3. Added validation for out-of-range line requests in `get` command with helpful error message in `get.rs:222-246`
- All error cases now properly exit with code 1, success cases exit with code 0.
- Ran cargo fmt and cargo clippy (all passing), rebuilt blz-dev binary successfully.
- Reinstalled both `blz` and `blz-dev` binaries; ran comprehensive verification testing confirming all exit code fixes working correctly.
- Audited `.agents/logs/` directory and archived 11 outdated logs (v0.2-v0.5.0 era documents, completed flavor work, old release notes) to `.agents/logs/.archive/`.
- Remaining logs are current: v1.0-beta testing reports, planning docs, and reference material (AGENTS.md, CLAUDE.md, MIGRATION-NOTES.md).
- Queried Linear MCP for Blaze team metadata and current BLZ issues list (BLZ-100 through BLZ-106 plus backlog items).
- Reviewed `snapshot-20251008T192218Z.json`; identified additional CodeRabbit findings (diff metadata SHA mismatch, doctor symlink recursion, history ISO parsing, info error context) not yet represented in Linear backlog.
- Logged follow-up tickets in Linear: BLZ-107 (metadata.sha256), BLZ-108 (doctor symlink guard), BLZ-109 (history ISO parsing), BLZ-110 (info metadata context); all set to Todo with priorities.
- Implemented release blockers for BLZ-101 through BLZ-110: restored anchors CLI, sanitized registry create-source inputs, ensured JSON outputs/history parsing, updated stats/prompt/lookup behavior, fixed metadata checksum usage, added build.rs help-file watcher, and hardened doctor/info flows. Ran `cargo test -p blz-cli` (all passing).

## 2025-10-05

- Verified formatting via `cargo fmt -- --check`.
- Ran `cargo bench`; Tantivy index and search benchmarks completed within targets (noted expected Criterion warnings about sample counts).
- Ran full workspace suite via `cargo test` (139 core + CLI tests plus doctests, all green).
- Ran full CLI suite via `cargo test -p blz-cli` (all 110 unit + 28 integration tests passed).
- Verified workspace lints via `cargo clippy --all-targets --all-features -- -D warnings` (clean).
- Verified metadata alias end-to-end flow by running `cargo test -p blz-cli --test alias_resolver_update_remove`; integration scenario succeeded.

Quick notes and links to detailed work logs.

## 2025-10-04

- Restored GET fallback in URL resolver for providers that reject HEAD (405/501), matching legacy `blz add` behavior so URL intelligence continues to work for S3/GitBook-style hosts; verified with local fixture that returns 405 to HEAD but 200 to GET.
- Preserved resolved variant during `blz update` so llms.json keeps `llms-full` metadata after upgrades.
- Kept metadata aliases out of persisted `Source.aliases` during update to restore removal-by-alias behavior and satisfy `alias_resolver_update_remove` expectations.

## 2025-10-02

- Kicked off CLI refactor/testing initiative to improve seams for automation and coverage.
  - Decision: break large command functions into testable helpers; abstract prompts/subprocess/fs.
  - Next: start with `blz clear` to prototype dependency injection pattern.
- Refactored `blz clear` into testable core (`execute_clear`) with injectable storage/prompt and
  added unit coverage; updated `.gitignore` to drop ad-hoc coverage artifacts.
- Completed `blz list` refactor with storage trait, pure rendering helpers, and unit tests covering
  text/json paths and empty states.
- Followed up with `blz remove` refactor (storage seam, confirmation hook, richer tests). Command
  now exercises delete paths without touching the filesystem in unit tests.
- Refactored `blz update`: introduced storage/indexer seams, separated pure `apply_update`, and
  added unit coverage for content persistence. CLI flow now orchestrates fetch/parse/index via the
  shared helpers.
- **Major Initiative**: Flavor Elimination & URL Intelligence for v1.0.0-beta.1
  - Comprehensive audit of all flavor-related code (335+ lines identified for removal)
  - Designed URL intelligence system to auto-prefer llms-full.txt when available
  - Planned content detection warnings for low-value "index" files
  - Designed upgrade detection for existing sources
  - See: [202510021153-feature-flavor-elimination-and-url-intelligence-v1-0.md](.agents/logs/202510021153-feature-flavor-elimination-and-url-intelligence-v1-0.md)

## 2025-10-01

- Cherry-picked heading count fixes and 404 filtering from old branch
  - Created `count_headings()` utility for accurate recursive counting
  - Parser now skips placeholder "404" pages
  - Baseline level normalization for docs that don't start at H1
- Consolidated work logs into single comprehensive document
  - See: [v0.5.0-release-work.md](.agents/logs/v0.5.0-release-work.md)
- Pushed `gt/v0.5-release` branch to remote (ready for PR when needed)
- Restored metadata alias propagation for update/add flows; resolver now consults metadata aliases (branch `gt/feat/cli/restore-metadata-alias-propagation`)
- Fixed registry `create_source` cloning so cargo check/tests/install succeed; local CLI reinstalled after `cargo install --path crates/blz-cli`
- Removed backwards compatibility code for v0.4.x format
  - Removed 141-line custom `LlmsJson` deserializer
  - Added detection for old cache format with helpful error message
  - Created `blz clear` command with `--force` flag
  - Clears entire cache directory when old format detected
  - Error guides users to run `blz clear --force` then re-add sources
- Release prep: resolved bench API drift, removed unused CLI HTTP helper, restored
  expected JSON outputs for `blz get`/`search`, and re-ran cargo fmt/clippy/test
  (all green on 2025-10-01)

## 2025-09-30

- Completed alias terminology refactoring (commit 303ac00)
  - Renamed `LlmsJson.alias` → `LlmsJson.source`
  - Added `--aliases` flag to `blz add` command
  - Updated all CLI help text for clarity
  - 21 files modified, all 224 tests passing
- Implemented registry system features
  - Created `blz registry create-source` command
  - Added `blz add --dry-run` for source analysis
  - Seeded registry with 22 verified sources
  - Built registry infrastructure (TOML → JSON)
- Implemented upgrade command for llms.txt → llms-full.txt migration

## 2025-09-29

- Feature flag implementation (FORCE_PREFER_FULL)
- Updated documentation to remove flavor complexity
- Version bumped to 0.5.0 across all crates

## 2025-10-03

- Authored batch manifest design for `blz add` (`.agents/logs/20251003-blz-add-batch-manifest-spec.md`) covering format, metadata propagation, and update flow for remote/local sources.
- Began implementation: extended `blz_core::Source` with origin/descriptor metadata fields and updated CLI helpers/tests to compile with new structure.
- Added per-source descriptor persistence APIs in `Storage`, refactored `blz add` single-source flow to reuse shared finalization logic, and introduced manifest + local file ingestion skeleton (execute_manifest/add_local_source).
- Expanded `blz list` to support `--details` (descriptor-aware output) and richer JSON payloads; docs updated with manifest workflow, added template at `registry/templates/batch-manifest.example.toml`, documented `blz add` metadata flags/defaults, and introduced curl-install script (`install.sh`).
- Reworked v1.0.0-beta.1 improvement plan with enriched JSON as the first P0 deliverable, defaulted `--context` to 5 lines, documented `--block`/`--context` exclusivity, added pre-announcement easy wins, and captured follow-on notes for manifests/staleness updates.
- Finished enriched context/block implementation for search/get: introduced shared block finalization helpers, ensured heading-aware ranges (accurate `lines`/`lineNumbers`), trimmed trailing blank lines, and updated CLI tests + doctests to cover new metadata fields (all clippy/tests passing).
