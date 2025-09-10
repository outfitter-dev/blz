---
name: Release checklist
about: Prepare and ship a new release
title: "Release vX.Y.Z"
labels: [release]
---

## Pre-flight

- [ ] Confirm `main` is green and releasable
- [ ] Set required secrets in repo:
  - [ ] `CARGO_REGISTRY_TOKEN` (crates.io)
  - [ ] `HOMEBREW_TAP_TOKEN` (write access to outfitter-dev/homebrew-tap)
  - [ ] `NPM_TOKEN` (already set)

## Versioning

- [ ] Pick version `vX.Y.Z`
- [ ] Ensure crate versions are correct in `Cargo.toml`

## Tag + Release

Run the tag script (creates and pushes annotated tag):

```bash
scripts/tag-release.sh vX.Y.Z
```

Then:

- [ ] Create GitHub Release for the new tag (attach notes/binaries if applicable)

## Post-release automations

- [ ] Homebrew tap bump PR opened on `outfitter-dev/homebrew-tap` and merged
- [ ] crates.io publish completed (if publishing this release)

## Triage

Use the helper to review open issues/PRs:

```bash
scripts/triage-issues.sh
```

## Verification

- [ ] `brew install outfitter-dev/tap/blz` works
- [ ] `cargo install --git https://github.com/outfitter-dev/blz --branch main blz-cli` works
- [ ] Smoke test basic commands: `blz --version`, `blz list`
