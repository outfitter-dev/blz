# Consolidated Workflow Testing

This document demonstrates the functionality of the consolidated `publish.yml` workflow and its various modes.

## Test Cases

### Test Case 1: Full Release (Default)

```bash
gh workflow run publish.yml -f tag=v1.0.0
```

**Expected behavior:**
- ✅ Build all platform binaries
- ✅ Upload release assets to GitHub
- ✅ Publish to npm
- ✅ Publish to crates.io
- ✅ Publish to Homebrew
- ✅ Generate release notes
- ✅ Finalize GitHub release

### Test Case 2: Assets Only Mode

```bash
gh workflow run publish.yml -f tag=v1.0.0 -f mode=assets-only
```

**Expected behavior:**
- ✅ Build all platform binaries
- ✅ Upload release assets to GitHub
- ❌ Skip npm publishing
- ❌ Skip crates.io publishing
- ❌ Skip Homebrew publishing
- ✅ Generate release notes
- ✅ Finalize GitHub release (assets only)

### Test Case 3: Publish Only Mode

```bash
gh workflow run publish.yml -f tag=v1.0.0 -f mode=publish-only
```

**Expected behavior:**
- ❌ Skip building (assets must already exist)
- ❌ Skip asset upload
- ✅ Publish to npm (from existing release)
- ✅ Publish to crates.io
- ✅ Publish to Homebrew
- ✅ Update release notes

### Test Case 4: Selective Skip Flags

```bash
gh workflow run publish.yml -f tag=v1.0.0 -f skip_homebrew=true -f skip_npm=true
```

**Expected behavior:**
- ✅ Build all platform binaries
- ✅ Upload release assets to GitHub
- ❌ Skip npm publishing (explicit skip)
- ✅ Publish to crates.io
- ❌ Skip Homebrew publishing (explicit skip)
- ✅ Generate release notes
- ✅ Finalize GitHub release

### Test Case 5: Dry Run Mode

```bash
gh workflow run publish.yml -f tag=v1.0.0 -f dry_run=true
```

**Expected behavior:**
- ✅ Build all platform binaries
- ✅ Upload release assets to GitHub
- ❌ Skip npm publishing (dry run)
- ❌ Skip crates.io publishing (dry run)
- ❌ Skip Homebrew publishing (dry run)
- ✅ Generate release notes
- ✅ Finalize GitHub release (assets only)

### Test Case 6: Prerelease Version

```bash
gh workflow run publish.yml -f tag=v1.0.0-beta.1
```

**Expected behavior:**
- ✅ Build all platform binaries
- ✅ Upload release assets to GitHub
- ✅ Publish to npm (with `beta` dist-tag)
- ✅ Publish to crates.io
- ❌ Skip Homebrew publishing (prerelease auto-skip)
- ✅ Generate release notes
- ✅ Finalize GitHub release (marked as prerelease)

## Validation

To validate the workflow functionality, check:

1. **Parameters are parsed correctly** - Check the "Publish Parameters" section in workflow run summary
2. **Conditional job execution** - Verify skipped jobs show as "Skipped" in GitHub Actions UI
3. **Error handling** - Ensure graceful failures when assets are missing in publish-only mode
4. **Backwards compatibility** - Existing tag-triggered releases should work unchanged

## Migration Verification

### Before Consolidation

- `release.yml` - Manual workflow dispatch with basic parameters
- `release-simplified.yml` - Experimental simplified workflow
- `manual-publish.yml` - Publishing-only workflow

### After Consolidation

All functionality is available through `publish.yml` with appropriate parameters:

| Old Workflow | New Equivalent |
|-------------|----------------|
| `release.yml` | `publish.yml` (default mode=full) |
| `release-simplified.yml` | `publish.yml` (default mode=full) |
| `manual-publish.yml` | `publish.yml -f mode=publish-only` |
| Manual npm only | `publish.yml -f skip_crates=true -f skip_homebrew=true` |
| Manual crates only | `publish.yml -f skip_npm=true -f skip_homebrew=true` |

## Rollback Plan

If issues are discovered, the archived workflows can be restored:

```bash
# Restore individual workflow if needed
git mv .github/workflows/archive/manual-publish.yml.deprecated .github/workflows/manual-publish.yml
```

However, the consolidated approach provides all the same functionality with better maintainability.