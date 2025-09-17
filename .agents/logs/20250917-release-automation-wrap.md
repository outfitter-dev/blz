# 2025-09-17 — Release Automation Wrap-Up

## Context
- Finalized CI release hardening work from PR #179
- Scoped npm publish permissions and added auth checks
- Ran local `act` dry run (npm path) to verify manual publish workflow
- Follow-up adjustments after merge to enforce Windows build parity and wire actionlint tooling

## Notes
- `manual-publish.yml` now limits `id-token` scope to npm jobs
- Increased crates propagation wait loop to 24 attempts (10s interval)
- Added `npm whoami` preflight for stable & canary paths
- Local dry-run used dockerized `act` with `publish_crates=false` to avoid index mismatch on unpublished versions; npm dry-run completed successfully
- `publish.yml` now requires Windows builds to pass (removed `continue-on-error`) and exposes `workflow_call` for reuse by `release.yml`
- `lefthook` pre-commit runs `actionlint` when workflow files change to catch structural errors early
- Post-merge polish tightened reusable publish permissions, expanded tag validation, and fixed artifact flattening to handle nested `target/release` paths
- Updated `publish.yml` to place downloaded artifacts in deterministic per-target directories and harden the flatten helper against missing Windows zips (with deeper search fallback + duplicate-safe moves)
- Ran `scripts/release/semver-bump.sh patch` to advance the workspace to v0.2.2 ahead of the next tag cut (syncing Cargo manifests, lockfile, npm metadata, and CHANGELOG)

## Follow-ups
- None; monitor next real publish run for parity with dry-run results
- If crates dry-run coverage required later, pre-seed registry or adjust test payload to skip until publish events
