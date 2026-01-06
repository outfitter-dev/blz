---
date: 2026-01-06 22:34 UTC
branch: chore/release/reset-release-baseline
slug: checkpoint-reset-release-baseline
pr: BLZ-307
agent: codex
---

# Checkpoint - reset release baseline

## Summary

Reset release-please baseline versions to the last tagged release (v1.3.0) so
release-please can cut v1.5.0 cleanly.

## Tasks
<!-- Use CLOSES: #123 / BLZ-45 for completed issues -->
<!-- Use ADDRESSES: #123 / BLZ-45 for partial work -->
<!-- Use RELATED: #123 / BLZ-45 for related but not closing -->

### Done

- [x] ADDRESSES: BLZ-307 reset manifest + Cargo workspace versions to v1.3.0

### In Progress

- [ ] Create branch + submit PR once commit is ready

### Not Started

- [ ] …

## Changes
<!-- Include files changed with brief descriptions -->

- `Cargo.toml` - reset workspace + internal crate versions to 1.3.0
- `.release-please-manifest.json` - reset baseline to 1.3.0

## Decisions & Rationale
<!-- Key technical decisions and why they were made -->

- release-please requires the manifest version to map to an existing tag.

## Current State

- **Branch Status**: clean after commit
- **CI Status**: not run
- **PR Stack**: pending submit
- **Open Items**: none

## Blockers
<!-- What's preventing progress, if anything -->

## Next Steps

- Submit PR for baseline reset

## Follow-ups
<!-- Non-critical items discovered during work to circle back to -->

## Context/Notes

---
