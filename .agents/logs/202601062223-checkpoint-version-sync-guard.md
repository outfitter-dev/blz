---
date: 2026-01-06 22:23 UTC
branch: chore/ci/version-sync-guard
slug: checkpoint-version-sync-guard
pr: BLZ-306
agent: codex
---

# Checkpoint - add version sync guard

## Summary

Add a lightweight CI check to keep package.json, Cargo.toml, and the release
manifest versions aligned.

## Tasks
<!-- Use CLOSES: #123 / BLZ-45 for completed issues -->
<!-- Use ADDRESSES: #123 / BLZ-45 for partial work -->
<!-- Use RELATED: #123 / BLZ-45 for related but not closing -->

### Done

- [x] ADDRESSES: BLZ-306 add version sync guard workflow and script

### In Progress

- [ ] Create branch + submit PR once commit is ready

### Not Started

- [ ] …

## Changes
<!-- Include files changed with brief descriptions -->

- `scripts/check-version-sync.py` - new version alignment check
- `.github/workflows/version-sync.yml` - run version sync on relevant changes

## Decisions & Rationale
<!-- Key technical decisions and why they were made -->

- Use Python tomllib/json for a dependency-free check on GitHub runners.

## Current State

- **Branch Status**: clean after commit
- **CI Status**: not run
- **PR Stack**: pending submit
- **Open Items**: none

## Blockers
<!-- What's preventing progress, if anything -->

## Next Steps

- Submit PR for version sync guard

## Follow-ups
<!-- Non-critical items discovered during work to circle back to -->

## Context/Notes

---
