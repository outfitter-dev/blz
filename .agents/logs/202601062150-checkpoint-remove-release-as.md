---
date: 2026-01-06 21:50 UTC
branch: chore/release/remove-release-as
slug: checkpoint-remove-release-as
pr: BLZ-304
agent: codex
---

# Checkpoint - remove release-as override

## Summary

Remove the temporary release-please `release-as` override now that the v1.5.0
release PR has been created.

## Tasks
<!-- Use CLOSES: #123 / BLZ-45 for completed issues -->
<!-- Use ADDRESSES: #123 / BLZ-45 for partial work -->
<!-- Use RELATED: #123 / BLZ-45 for related but not closing -->

### Done

- [x] CLOSES: BLZ-304 remove release-as override

### In Progress

- [ ] Create branch + submit PR once commit is ready

### Not Started

- [ ] …

## Changes
<!-- Include files changed with brief descriptions -->

- `.release-please-config.json` - remove release-as override

## Decisions & Rationale
<!-- Key technical decisions and why they were made -->

- Keep release-please back to normal version calculation after v1.5.0.

## Current State

- **Branch Status**: clean after commit
- **CI Status**: not run
- **PR Stack**: pending submit
- **Open Items**: none

## Blockers
<!-- What's preventing progress, if anything -->

## Next Steps

- Submit PR for removing release-as

## Follow-ups
<!-- Non-critical items discovered during work to circle back to -->

## Context/Notes

---
