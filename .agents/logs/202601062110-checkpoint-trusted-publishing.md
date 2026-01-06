---
date: 2026-01-06 21:10 UTC
branch: chore/ci/npm-trusted-publishing
slug: checkpoint-trusted-publishing
pr: BLZ-302
agent: codex
---

# Checkpoint - enable npm trusted publishing

## Summary

Make npm publishing rely on GitHub OIDC when `NPM_TOKEN` is absent and require
npm 11.5.1+ for trusted publishing.

## Tasks
<!-- Use CLOSES: #123 / BLZ-45 for completed issues -->
<!-- Use ADDRESSES: #123 / BLZ-45 for partial work -->
<!-- Use RELATED: #123 / BLZ-45 for related but not closing -->

### Done

- [x] ADDRESSES: BLZ-302 enable npm trusted publishing (OIDC) in publish workflow

### In Progress

- [ ] Create PR + submit stack once branch is created

### Not Started

- [ ] …

## Changes
<!-- Include files changed with brief descriptions -->

- `.github/workflows/publish-npm.yml` - make `NPM_TOKEN` optional and ensure npm 11.5.1+

## Decisions & Rationale
<!-- Key technical decisions and why they were made -->

- Keep token fallback while preferring OIDC; npm CLI uses OIDC before tokens.
- Require npm 11.5.1+ per npm trusted publishing requirements.

## Current State

- **Branch Status**: clean after commit
- **CI Status**: not run
- **PR Stack**: pending submit
- **Open Items**: none

## Blockers
<!-- What's preventing progress, if anything -->

## Next Steps

- Submit PR for trusted publishing tweak
- Proceed to release-please 1.5.0 slice

## Follow-ups
<!-- Non-critical items discovered during work to circle back to -->

## Context/Notes

---
