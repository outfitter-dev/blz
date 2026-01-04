# CI/CD Pipeline

Complete guide to BLZ's continuous integration and deployment setup using GitHub Actions.

## Table of Contents

- [Overview](#overview)
- [Release Automation](#release-automation)
- [Publish Workflow](#publish-workflow)
- [Local Testing with Act](#local-testing-with-act)
- [Test Cases](#test-cases)
- [Troubleshooting](#troubleshooting)

## Overview

BLZ uses GitHub Actions for continuous integration and deployment with the following workflows:

- **`release-please.yml`** - Opens/updates the release PR on `main`
- **`release-please-canary.yml`** - Opens/updates prerelease PRs on `release-canary`
- **`publish.yml`** - Main release publisher (assets + registries)
- **`ci.yml`** - Continuous integration checks on pull requests

All workflows are optimized for Graphite stacked PRs and support both automatic and manual triggering.

## Release Automation

Release-please drives versioning and changelog updates via conventional commits.
Legacy label-based automation (`auto-release.yml`) and release-drafter are
archived under `.github/workflows/archive/` and no longer run.

### Mainline releases (`main`)

1. Merge conventional commits into `main`.
2. release-please opens or updates the release PR.
3. Merge the release PR to create the tag and draft release.
4. The tag triggers `publish.yml` to upload assets and publish to registries.

### Canary releases (`release-canary`)

1. Merge conventional commits into `release-canary`.
2. release-please opens or updates a prerelease PR.
3. Merge the PR to create a `-canary` tag and draft release.
4. The tag triggers `publish.yml` and publishes with the canary dist-tag.

### Token requirement

Release-please must use a PAT (e.g., `RELEASE_PLEASE_TOKEN`) so tag creation
triggers downstream workflows like `publish.yml`.

## Publish Workflow

The `publish.yml` workflow publishes assets and registries. Release notes are
owned by release-please; set `generate_release_notes=true` only when you need
to overwrite the release body from PR history.

### Modes

#### Full Release (Default)

Complete release with all publishing steps:

```bash
gh workflow run publish.yml -f tag=v1.0.0
```

**Actions:**

- ✅ Build all platform binaries
- ✅ Upload release assets to GitHub
- ✅ Publish to npm
- ✅ Publish to crates.io
- ✅ Publish to Homebrew
- ✅ Finalize GitHub release

#### Assets Only Mode

Build and upload binaries without publishing to registries:

```bash
gh workflow run publish.yml -f tag=v1.0.0 -f mode=assets-only
```

**Actions:**

- ✅ Build all platform binaries
- ✅ Upload release assets to GitHub
- ❌ Skip npm publishing
- ❌ Skip crates.io publishing
- ❌ Skip Homebrew publishing
- ✅ Finalize GitHub release (assets only)

#### Publish Only Mode

Publish to registries using existing release assets:

```bash
gh workflow run publish.yml -f tag=v1.0.0 -f mode=publish-only
```

**Actions:**

- ❌ Skip building (assets must already exist)
- ❌ Skip asset upload
- ✅ Publish to npm (from existing release)
- ✅ Publish to crates.io
- ✅ Publish to Homebrew
- ✅ Update release notes (only if `generate_release_notes=true`)

**Use case:** When you need to re-publish to a specific registry without rebuilding binaries.

### Selective Skip Flags

Skip specific publishing steps:

```bash
gh workflow run publish.yml -f tag=v1.0.0 -f skip_homebrew=true -f skip_npm=true
```

**Actions:**

- ✅ Build all platform binaries
- ✅ Upload release assets to GitHub
- ❌ Skip npm publishing (explicit skip)
- ✅ Publish to crates.io
- ❌ Skip Homebrew publishing (explicit skip)
- ✅ Generate release notes
- ✅ Finalize GitHub release

### Dry Run Mode

Test the workflow without publishing:

```bash
gh workflow run publish.yml -f tag=v1.0.0 -f dry_run=true
```

**Actions:**

- ✅ Build all platform binaries
- ✅ Upload release assets to GitHub
- ❌ Skip npm publishing (dry run)
- ❌ Skip crates.io publishing (dry run)
- ❌ Skip Homebrew publishing (dry run)
- ✅ Generate release notes
- ✅ Finalize GitHub release (assets only)

### Prerelease Handling

Prereleases are automatically detected:

```bash
gh workflow run publish.yml -f tag=v1.0.0-beta.1
```

**Actions:**

- ✅ Build all platform binaries
- ✅ Upload release assets to GitHub
- ✅ Publish to npm (with `beta` dist-tag)
- ✅ Publish to crates.io
- ❌ Skip Homebrew publishing (prerelease auto-skip)
- ✅ Generate release notes
- ✅ Finalize GitHub release (marked as prerelease)

### Workflow Consolidation

The current `publish.yml` consolidates several previous workflows:

| Old Workflow | New Equivalent |
|-------------|----------------|
| `release.yml` | `publish.yml` (default mode=full) |
| `release-simplified.yml` | `publish.yml` (default mode=full) |
| `manual-publish.yml` | `publish.yml -f mode=publish-only` |
| Manual npm only | `publish.yml -f skip_crates=true -f skip_homebrew=true` |
| Manual crates only | `publish.yml -f skip_npm=true -f skip_homebrew=true` |

## Local Testing with Act

Use [act](https://github.com/nektos/act) to rehearse release automation locally before pushing tags.

### Prerequisites

- Docker installed and running
- `act` installed: `brew install act` or see the [act README](https://github.com/nektos/act)
- Personal access token with `repo` scope saved as `~/.config/act/secrets` or passed via `-s GITHUB_TOKEN=...`

### Quick Start

#### Test Release-Please Workflow

Dry-run the release-please workflow (note: GitHub API interactions are limited under `act`):

```bash
act workflow_dispatch -W .github/workflows/release-please.yml
```

#### Simulate Build Pipeline

Test the build and upload steps (publishing skipped when `ACT=true`):

```bash
ACT=1 act workflow_dispatch \
  -W .github/workflows/publish.yml \
  -j upload_release_assets \
  --input tag=v0.0.0 --input dist_tag=latest
```

### Tips

**Cache Compilation Artifacts**

Mount a cache volume to reuse Rust compilation artifacts:

```bash
act workflow_dispatch --reuse -v cargo-cache:/github/home/.cargo
```

**Limit Scope**

Only trigger build-focused jobs when rehearsing locally:

```bash
# Test only the build step
act workflow_dispatch -W .github/workflows/publish.yml -j upload_release_assets
```

**Publishing Jobs**

Publishing jobs require network access and real secrets. Omit them during local testing to avoid accidental publishes.

### Local Hooks + Nextest

For faster local development, use Lefthook with nextest:

```bash
# Bootstrap the development environment (installs nextest, sets up hooks)
just bootstrap-fast

# After setup, pre-commit hooks will run nextest automatically
git commit -am "your message"
```

See `docs/development/testing.md` for more details on the testing setup.

## Test Cases

### Validation Checklist

To validate workflow functionality, check:

1. **Parameters parsed correctly** - Review "Publish Parameters" in workflow run summary
2. **Conditional job execution** - Verify skipped jobs show as "Skipped" in GitHub Actions UI
3. **Error handling** - Ensure graceful failures when assets are missing in publish-only mode
4. **Backwards compatibility** - Existing tag-triggered releases should work unchanged

### Example Test Scenarios

#### Scenario 1: Emergency Hotfix

You need to publish a critical patch immediately:

```bash
# 1. Merge hotfix PR to main (conventional commit)
# 2. Merge the release-please PR
# 3. Automation creates tag and runs publish workflow
gh run list --workflow=publish.yml
```

#### Scenario 2: Failed npm Publish

npm publish failed but other steps succeeded:

```bash
# Re-publish only to npm without rebuilding
gh workflow run publish.yml \
  -f tag=v1.0.0 \
  -f mode=publish-only \
  -f skip_crates=true \
  -f skip_homebrew=true
```

#### Scenario 3: Beta Testing

Release a beta for testing before stable:

```bash
# 1. Merge changes into release-canary
# 2. Merge the release-please canary PR
# 3. Published with canary dist-tag
npm install @outfitter/blz@canary
```

## Troubleshooting

### Common Issues

#### Workflow Fails with "Assets Not Found"

**Problem:** Publish-only mode can't find release assets.

**Solution:**

```bash
# Run assets-only mode first
gh workflow run publish.yml -f tag=v1.0.0 -f mode=assets-only

# Then run publish-only
gh workflow run publish.yml -f tag=v1.0.0 -f mode=publish-only
```

#### Publishing to Homebrew Fails

**Problem:** Homebrew tap update fails.

**Solution:**

1. Check that the tag exists on GitHub
2. Verify release assets are uploaded
3. Ensure `HOMEBREW_TOKEN` secret is set
4. Review Homebrew tap PR for issues

#### Dry Run Publishes Anyway

**Problem:** Dry run mode still publishes to registries.

**Cause:** The `dry_run` flag only works in full release mode.

**Solution:**

```bash
# Use dry_run=true in default mode
gh workflow run publish.yml -f tag=v1.0.0 -f dry_run=true
```

#### Act Fails Locally

**Problem:** Act fails with authentication errors.

**Solution:**

```bash
# Ensure GITHUB_TOKEN is set
cat ~/.config/act/secrets
# Should contain: GITHUB_TOKEN=ghp_...

# Or pass inline
act workflow_dispatch -s GITHUB_TOKEN=ghp_...
```

### Debugging Workflows

#### Enable Debug Logging

Add debug output to workflow runs:

```bash
# Set repository secret
gh secret set ACTIONS_STEP_DEBUG -b true
gh secret set ACTIONS_RUNNER_DEBUG -b true
```

#### View Workflow Logs

```bash
# List recent runs
gh run list --workflow=publish.yml

# View specific run
gh run view <run-id> --log

# Download logs
gh run download <run-id>
```

#### Check Workflow Status

```bash
# Watch workflow in real-time
gh run watch

# List all workflows
gh workflow list

# View workflow file
gh workflow view publish.yml
```

## Rollback Plan

If issues are discovered with the consolidated workflow, archived workflows can be restored:

```bash
# Restore individual workflow if needed
git mv .github/workflows/archive/manual-publish.yml.deprecated \
  .github/workflows/manual-publish.yml
```

However, the consolidated approach provides all the same functionality with better maintainability.

## See Also

- [Testing Guide](testing.md) - Testing strategies and tools
- [Contributing](./contributing.md) - How to contribute
- [Development Workflow](./workflow.md) - Development process
