# Scratchpad

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

## Related Logs

- **[202510021153-feature-flavor-elimination-and-url-intelligence-v1-0.md](.agents/logs/202510021153-feature-flavor-elimination-and-url-intelligence-v1-0.md)** - v1.0.0-beta.1 implementation plan (active)
- [v0.5.0-release-work.md](.agents/logs/v0.5.0-release-work.md) - Comprehensive v0.5.0 work log (superseded)
- [alias-terminology-audit.md](.agents/logs/alias-terminology-audit.md) - Analysis of alias terminology issues
- [flavor-removal-impact-analysis.md](.agents/logs/flavor-removal-impact-analysis.md) - Impact analysis for flavor simplification
- [202509301145-checkpoint-v0.5.0-release-prep.md](.agents/logs/202509301145-checkpoint-v0.5.0-release-prep.md) - Earlier checkpoint

## Branch Status

- **Current**: `gt/v0.5-release` (will become v1.0.0-beta.1)
- **Remote**: Synced with `origin/gt/v0.5-release`
- **Tests**: ✅ 224/224 passing
- **Status**: Active development - major refactor in progress

## Next Steps

- ✅ Research and planning complete for v1.0.0-beta.1
- 🚧 Phase 0: Implement URL intelligence (llms-full.txt preference)
- 🚧 Phases 1-8: Execute flavor elimination cleanup sequentially
- Create PR for v1.0.0-beta.1 after implementation complete
- Consider removing old `gt/fix-normalize-heading-counts-and-filter-placeholder-pages` branch (fixes cherry-picked)

## 2025-10-03

- Authored batch manifest design for `blz add` (`.agents/logs/20251003-blz-add-batch-manifest-spec.md`) covering format, metadata propagation, and update flow for remote/local sources.
- Began implementation: extended `blz_core::Source` with origin/descriptor metadata fields and updated CLI helpers/tests to compile with new structure.
- Added per-source descriptor persistence APIs in `Storage`, refactored `blz add` single-source flow to reuse shared finalization logic, and introduced manifest + local file ingestion skeleton (execute_manifest/add_local_source).
- Expanded `blz list` to support `--details` (descriptor-aware output) and richer JSON payloads; docs updated with manifest workflow, added template at `registry/templates/batch-manifest.example.toml`, documented `blz add` metadata flags/defaults, and introduced curl-install script (`install.sh`).
- Reworked v1.0.0-beta.1 improvement plan with enriched JSON as the first P0 deliverable, defaulted `--context` to 5 lines, documented `--block`/`--context` exclusivity, added pre-announcement easy wins, and captured follow-on notes for manifests/staleness updates.
- Finished enriched context/block implementation for search/get: introduced shared block finalization helpers, ensured heading-aware ranges (accurate `lines`/`lineNumbers`), trimmed trailing blank lines, and updated CLI tests + doctests to cover new metadata fields (all clippy/tests passing).
