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

## gt/fix/release/semver-rust
- ✅ Addressed follow-up
- Notes: Refined `update_package_lock` to return the formatted JSON without rewriting on disk, so the caller performs the single write with context. `cargo test -p blz-release` passes after removing the unused `CARGO_LOCK` constant warning.

- ✅ Added package coverage
- Notes: Included `blz-release` in the default `--package` list for `update-lock` CLI invocations and confirmed `cargo run -p blz-release -- update-lock --version 0.3.1` succeeds without missing-package errors.

- ✅ Semver script parity
- Notes: `scripts/release/semver-bump.sh` now passes `--package blz-cli --package blz-core --package blz-release` so bumps affect every workspace crate tracked in `Cargo.lock`.

- ✅ Align dev-deps
- Notes: `crates/blz-release/Cargo.toml` now pulls `tempfile` from the workspace definition instead of pinning `"3"` locally.

- ✅ Workspace hygiene
- Notes: The release crate now inherits `version`, `edition`, `authors`, and `license` from `[workspace.package]` to stay in sync, and the `set` mode accepts idempotent bumps by comparing against the sanitized current version.

- ✅ Test cleanup & diagnostics
- Notes: Added contextual read errors for npm manifest checks and converted release-tool tests to return `anyhow::Result` instead of `unwrap`, following repository lint policy.

- ✅ Polish after review
- Notes: Marked `blz-release` as `publish = false`, cached `Version` string comparisons, preferred installed binaries in the semver script, and trimmed the unused `time` formatting feature per review nitpicks.

- ✅ Docstring coverage
- Notes: Added doc comments across the release helper CLI types and helpers to keep documentation coverage healthy.
