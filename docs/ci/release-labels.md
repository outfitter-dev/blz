# Release Label Guide

Use these labels on pull requests that target `main` to signal release automation.

- `release:patch` – publish a new patch version (0.0.x) once merged.
- `release:minor` – publish a new minor version (0.x.0) once merged.
- `release:major` – publish a new major version (x.0.0) once merged.
- `release:canary` – publish a pre-release canary build tagged with the canary dist-tag.
- `release:hold` – pause automation for the PR until the label is removed.

If no `release:*` label is present, the release workflow will skip tagging. See `.github/workflows/auto-release.yml` for automation logic.
