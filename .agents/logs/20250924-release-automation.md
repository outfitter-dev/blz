# Release Automation Review — 2025-09-24

Reviewed sequentially via `gt up` starting from `main`.

## gt/feat_ci_homebrew_assets
- ✅ Looks good (no diff from `main`)
- Notes: Branch is empty; consider deleting if unused.

## gt/feat_release_homebrew_artifacts
- ✅ Looks good
- Notes: `publish.yml` now emits artifact SHAs that feed into `publish-homebrew.yml`, eliminating the draft-release dependency while preserving manual overrides.

## gt/ci_release_dotslash_trigger
- ✅ Looks good
- Notes: DotSlash workflow now listens to `release: published`, skips drafts, and verifies assets with retries before generating files.

## gt/test_cli_preprocess_coverage
- ✅ Looks good
- Notes: `is_known_subcommand` derives from Clap definitions (cached via `OnceLock`) with reserved fallbacks, and added tests lock in hidden subcommand behaviour.

## gt/ci_auto_publish_releases
- ✅ Looks good
- Notes: Adds `publish_release` job that undrafts the GitHub release before registry publishing; registry jobs wait on it and skip when in dry-run/skip modes. Release-summary messaging updated accordingly.

## gt/ci_homebrew_retry_logic
- ✅ Looks good
- Notes: Adds exponential backoff for both `gh release view` and `gh release download`, with step-summary logging and cleanup. Matches resilience goals for Homebrew publishing.

## gt/docs_update_workflow_readme
- ✅ Looks good
- Notes: Updated `generate-dotslash.yml` section to document the `release: published` trigger and asset readiness check.

## gt/ci_consolidate_workflows
- ✅ Looks good
- Notes: `publish_release` now runs when assets are skipped (publish-only mode) while still honoring dry-run and successful asset uploads in full mode.

## main (post-merge follow-up)
- ✅ Looks good
- Notes: Replaced the Node/Python helpers with the new `blz-release` Rust binary; `scripts/release/semver-bump.sh` now shells out to it for version math, npm sync, and Cargo.lock updates.
