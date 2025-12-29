# BLZ release flow migration plan (optimized)

Last updated: 2025-12-29

## Executive summary

This plan replaces BLZ's label-based release automation and manual changelog
maintenance with a single, automated release flow driven by conventional commits.
We will use release-please to generate a release PR, update `CHANGELOG.md`, and
create the tag/release, then keep the existing `publish.yml` workflow as the
publisher of assets and registries. A prerelease branch (`release-canary`) will
support canary releases. Lockfiles stay up to date via dependency automation,
not in the release PR.

## Research summary (current state)

### Release entrypoints in the repo today
- `.github/workflows/auto-release.yml` uses PR labels (`release:*`) to bump and tag versions.
- `.github/workflows/publish.yml` publishes releases on tag push or manual dispatch.
- `.github/workflows/release-drafter.yml` creates draft release notes based on labels.
- `scripts/release/semver-bump.sh` plus `crates/blz-release` compute and sync versions.
- `justfile` includes `release-prep` using cargo-release.

### Version sources and packaging
- `package.json` is the npm package source (`@outfitter/blz`).
- `Cargo.toml` uses a workspace version (`[workspace.package] version = "1.3.0"`).
- `blz-cli` and `blz-core` use `version.workspace = true`.
- Workspace dependencies pin `blz-core`/`blz-mcp` versions inside `Cargo.toml`.

### Release history drift
- Tags show gaps (e.g., `v1.2.0` is missing), while `CHANGELOG.md` lists a 1.2.0 release.
- GitHub releases show duplicate drafts for `v1.3.1` without a tag.
- Prior release logs mention manual fixes (e.g., asset gaps and manual semver bumps).

### Changelog and release notes
- `CHANGELOG.md` is manual and contains a large `Unreleased` section.
- `publish.yml` generates release notes by walking PRs between tags (not from the changelog).
- `release-drafter` also generates release notes from labels, creating overlap.

### Summary of pain points
- Multiple release entrypoints and overlapping release notes.
- Version drift between tags, releases, and the changelog.
- Manual steps for versioning and changelog maintenance.
- Release labels add friction and are easy to miss or misapply.

## Goals

- Single, reliable release path with minimal manual steps.
- Automated version bumps and changelog updates.
- Consistent GitHub releases, tags, and registries.
- Clear canary/prerelease story without extra tooling.
- Fewer release-specific tools and scripts to maintain.

## Decision: release-please (selected)

### Why release-please
- GitHub-native and well-suited to a repo that already uses GitHub Actions.
- Supports conventional commits, which we already enforce.
- Handles multi-package/version synchronization and changelog updates.
- Creates release PRs that are easy to review and audit.

### Why not semantic-release (for now)
- Node-first toolchain adds extra complexity in a Rust-first repo.
- Would require custom exec plugins to update Cargo workspace versioning.
- Higher maintenance for cross-language release logic.

## Target release flow (future state)

1. PRs merge to `main` with conventional commits.
2. release-please opens or updates a single release PR.
3. Merging the release PR bumps versions and updates `CHANGELOG.md`.
4. release-please creates the tag and draft GitHub release (notes from changelog).
5. `publish.yml` uploads assets and publishes to npm, crates.io, and Homebrew.
6. `publish.yml` publishes the draft release without overwriting release-please notes.

Single entrypoint: release-please is the only system that decides when a release is created.

## Changelog strategy (fix the mess)

- `CHANGELOG.md` is managed by release-please going forward.
- The former `Unreleased` section is archived in `docs/release/next-release-notes.md`.
- GitHub release notes should always come from the release-please changelog body.

## Implementation plan

### Phase 0: Preflight and cleanup (1 day)

- Reconcile release state:
  - Confirm latest tag and release (currently `v1.3.0`).
  - Close or merge duplicate draft releases (e.g., duplicate `v1.3.1` drafts).
  - Decide how to handle the missing `v1.2.0` tag (leave as historical drift or backfill if needed).
- Identify versioned files:
  - `package.json`
  - `Cargo.toml` (workspace version + workspace dependency pins)
- Freeze label-based automation and release-drafter to avoid conflicts during setup.

### Phase 1: Introduce release-please in PR-only mode (1-2 days)

- Add `.release-please-config.json` and `.release-please-manifest.json`.
- Use a single root package config and update:
  - `package.json`
  - `Cargo.toml` workspace version and pinned dependency versions via `extra-files`.
- Do **not** update lockfiles in release PRs.
- Configure changelog sections to align with our conventional commit types.
- Enable `draft: true` so the draft release is ready for publish.yml to attach assets.
- Run release-please via `workflow_dispatch` only.

Validation checklist:
- Release PR content (version bumps, changelog entries).
- Tag format (`vX.Y.Z`) and release draft creation.
- No asset publishing during dry runs.

### Phase 2: Canary release branch (1 day)

- Create a separate workflow for the `release-canary` branch.
- Use a dedicated config file (e.g., `.release-please-canary.json`) with:
  - `prerelease: true`
  - `prerelease-type: canary`
  - `draft: true`
- The canary branch produces prerelease tags and draft releases, published via `publish.yml`.

### Phase 3: Align publish workflow with release-please (1 day)

- Ensure `publish.yml` does not overwrite release-please release notes.
- Prefer release-please release body; only update notes if explicitly requested.
- Keep manual `workflow_dispatch` support for one-off re-publish.

### Phase 4: Cutover (same day)

- Enable release-please on `main` and `release-canary`.
- Disable/remove:
  - `.github/workflows/auto-release.yml`
  - `.github/workflows/release-drafter.yml`
  - `scripts/release/semver-bump.sh`
  - `crates/blz-release`
  - `justfile` `release-prep` target

### Phase 5: Documentation updates and guardrails (1 day)

- Update:
  - `docs/development/ci_cd.md`
  - `docs/development/workflow.md`
- Add a short release playbook focused on:
  - "merge release PR" as the only release action
  - how to run manual publish if needed
  - how to cut a canary from `release-canary`

### Phase 6: Lockfile upkeep (same day)

- Fix `.github/dependabot.yml` to keep:
  - `Cargo.lock` via the `cargo` ecosystem
  - `package-lock.json` via the `npm` ecosystem
- Schedule weekly updates; lockfiles stay current outside release PRs.

## Risks and mitigations

- Misconfigured release-please updates the wrong files.
  - Mitigate with dry runs and explicit file lists.
- Release notes drift between changelog and GitHub releases.
  - Mitigate by using release-please body as the single source of truth.
- Lockfile drift.
  - Mitigate with Dependabot and periodic dependency updates.
- Tag creation fails to trigger publish workflows.
  - Mitigate by using a PAT for release-please (GITHUB_TOKEN does not trigger downstream workflows).

## Rollback plan

- Re-enable `.github/workflows/auto-release.yml`.
- Restore `scripts/release/semver-bump.sh` and `crates/blz-release`.
- Revert release-please config and workflow files.
- Keep `publish.yml` unchanged so tag-based release continues to work.

## Success criteria

- A single release PR is created per release cycle.
- Version bumps are automated across npm and Rust workspace files.
- `CHANGELOG.md` is updated automatically and matches GitHub release notes.
- Tag creation consistently triggers the publish workflow.
- No manual label application required to ship releases.
