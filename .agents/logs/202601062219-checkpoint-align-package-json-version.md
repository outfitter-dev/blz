---
date: 2026-01-06 22:19 UTC
branch: chore/release/align-package-json-version
slug: checkpoint-align-package-json-version
pr: BLZ-305
agent: codex
---

# Checkpoint - align package.json version

## Summary

Align package.json with the manifest baseline so release-please can compute the
next release cleanly.

## Tasks
<!-- Use CLOSES: #123 / BLZ-45 for completed issues -->
<!-- Use ADDRESSES: #123 / BLZ-45 for partial work -->
<!-- Use RELATED: #123 / BLZ-45 for related but not closing -->

### Done

- [x] CLOSES: BLZ-305 bump package.json to match v1.4.0 baseline

### In Progress

- [ ] Create branch + submit PR once commit is ready

### Not Started

- [ ] …

## Changes
<!-- Include files changed with brief descriptions -->

- `package.json` - bump version to 1.4.0

## Decisions & Rationale
<!-- Key technical decisions and why they were made -->

- release-please requires extra-files to match the last tagged version.

## Current State

- **Branch Status**: clean after commit
- **CI Status**: not run
- **PR Stack**: pending submit
- **Open Items**: none

## Blockers
<!-- What's preventing progress, if anything -->

## Next Steps

- Submit PR for package.json version alignment

## Follow-ups
<!-- Non-critical items discovered during work to circle back to -->

## Context/Notes

---
