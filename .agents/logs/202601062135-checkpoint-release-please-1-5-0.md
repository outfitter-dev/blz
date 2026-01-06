---
date: 2026-01-06 21:35 UTC
branch: chore/ci/npm-trusted-publishing
slug: checkpoint-release-please-1-5-0
pr: #392 / BLZ-303
agent: codex
---

# Checkpoint - force release-please 1.5.0

## Summary

Set release-please to cut v1.5.0 and ensure `package.json` is versioned
alongside Cargo workspace releases.

## Tasks
<!-- Use CLOSES: #123 / BLZ-45 for completed issues -->
<!-- Use ADDRESSES: #123 / BLZ-45 for partial work -->
<!-- Use RELATED: #123 / BLZ-45 for related but not closing -->

### Done

- [x] ADDRESSES: BLZ-303 set release-as 1.5.0 and include package.json

### In Progress

- [ ] Create PR + submit stack once branch is created

### Not Started

- [ ] …

## Changes
<!-- Include files changed with brief descriptions -->

- `.release-please-config.json` - add release-as and package.json extra-file

## Decisions & Rationale
<!-- Key technical decisions and why they were made -->

- Force v1.5.0 so release-please doesn't default to 1.4.1.
- Track package.json version for npm publish validation.

## Current State

- **Branch Status**: clean after commit
- **CI Status**: not run
- **PR Stack**: stacked above trusted publishing branch (pending submit)
- **Open Items**: none

## Blockers
<!-- What's preventing progress, if anything -->

## Next Steps

- Submit PR for release-please 1.5.0 override
- Remove release-as in follow-up after release PR merges

## Follow-ups
<!-- Non-critical items discovered during work to circle back to -->

## Context/Notes

---
