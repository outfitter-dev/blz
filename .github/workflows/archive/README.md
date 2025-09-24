# Archived Workflows

This directory contains deprecated workflow files that have been consolidated into a single parameterized workflow.

## Archived Files

- **`release.yml.deprecated`** - Legacy release workflow with matrix builds
- **`release-simplified.yml.deprecated`** - Experimental simplified release workflow
- **`manual-publish.yml.deprecated`** - Manual publishing workflow for crates/npm

## Replacement

These workflows have been consolidated into **`publish.yml`** which now supports:

- Multiple release modes (`full`, `assets-only`, `publish-only`)
- Individual skip flags (`skip_npm`, `skip_crates`, `skip_homebrew`)
- Dry run mode for validation
- All functionality from the original workflows

## Usage

Instead of the archived workflows, use:

```bash
# Full release (default behavior)
gh workflow run publish.yml -f tag=v1.0.0

# Build and upload assets only
gh workflow run publish.yml -f tag=v1.0.0 -f mode=assets-only

# Publish to registries only (from existing release)
gh workflow run publish.yml -f tag=v1.0.0 -f mode=publish-only

# Skip specific registries
gh workflow run publish.yml -f tag=v1.0.0 -f skip_homebrew=true

# Dry run for validation
gh workflow run publish.yml -f tag=v1.0.0 -f dry_run=true
```

## Migration Date

These workflows were consolidated on 2024-09-24 as part of issue #208.