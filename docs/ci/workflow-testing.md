# Workflow Testing Guide

Use [act](https://github.com/nektos/act) to rehearse the release automation locally before pushing tags.

## Prerequisites
- Docker installed and running
- `act` installed (`brew install act` or see the act README)
- A personal access token with `repo` scope saved as `~/.config/act/secrets` or passed via `-s GITHUB_TOKEN=...`

## Quickstart
```bash
# Dry-run the release detection logic on the current branch
act pull_request -W .github/workflows/auto-release.yml -j detect

# Simulate the publish pipeline (builds only, publishing steps skip when ACT=true)
ACT=1 act workflow_dispatch \
  -W .github/workflows/publish.yml \
  -j upload_release_assets \
  --input tag=v0.0.0 --input dist_tag=latest
```

## Notes
- Only trigger the build-focused jobs (e.g., `upload_release_assets`) when rehearsing locally so no external publishing runs.
- You can mount a cache volume to reuse compilation artifacts:
  `act workflow_dispatch --reuse -v cargo-cache:/github/home/.cargo`
- Publishing jobs still require network access and real secrets—omit them while testing.

Refer back to this document whenever you need to validate changes to the release automation without burning real tags.
