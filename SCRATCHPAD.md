# Scratchpad

Quick notes and links to detailed work logs.

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

- [v0.5.0-release-work.md](.agents/logs/v0.5.0-release-work.md) - Comprehensive v0.5.0 work log (current branch)
- [alias-terminology-audit.md](.agents/logs/alias-terminology-audit.md) - Analysis of alias terminology issues
- [flavor-removal-impact-analysis.md](.agents/logs/flavor-removal-impact-analysis.md) - Impact analysis for flavor simplification
- [202509301145-checkpoint-v0.5.0-release-prep.md](.agents/logs/202509301145-checkpoint-v0.5.0-release-prep.md) - Earlier checkpoint

## Branch Status

- **Current**: `gt/v0.5-release`
- **Remote**: Synced with `origin/gt/v0.5-release`
- **Tests**: ✅ 224/224 passing
- **Status**: Ready for release / PR creation

## Next Steps

- Create PR for v0.5.0 when ready
- Consider removing old `gt/fix-normalize-heading-counts-and-filter-placeholder-pages` branch (fixes cherry-picked)
