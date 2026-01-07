---
date: 2026-01-07 18:33 UTC
branch: chore/release/release-as-1-5-0
slug: checkpoint-release-as-reset
pr: BLZ-308
agent: codex
---

# Checkpoint - reset baseline and release-as

## Summary

Align package.json back to the baseline release version and re-enable the
release-as override so release-please can cut v1.5.0.

## Tasks
<!-- Use CLOSES: #123 / BLZ-45 for completed issues -->
<!-- Use ADDRESSES: #123 / BLZ-45 for partial work -->
<!-- Use RELATED: #123 / BLZ-45 for related but not closing -->

### Done

- [x] ADDRESSES: BLZ-308 reset package.json to 1.3.0 and re-add release-as 1.5.0

### In Progress

- [ ] Create branch + submit PR once commit is ready

### Not Started

- [ ] …

## Changes
<!-- Include files changed with brief descriptions -->

- `package.json` - reset version to 1.3.0
- `.release-please-config.json` - re-add release-as 1.5.0

## Decisions & Rationale
<!-- Key technical decisions and why they were made -->

- release-please requires extra-files to match the last tagged release.

## Current State

- **Branch Status**: clean after commit
- **CI Status**: not run
- **PR Stack**: pending submit
- **Open Items**: none

## Blockers
<!-- What's preventing progress, if anything -->

## Next Steps

- Submit PR for baseline alignment + release-as override

## Follow-ups
<!-- Non-critical items discovered during work to circle back to -->

## Context/Notes

---
