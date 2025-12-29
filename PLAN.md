# BLZ Release Flow Migration Plan

**Migration from label-based releases to release-please automation**

## Executive Summary

This plan migrates BLZ from a custom label-driven release system to [release-please](https://github.com/googleapis/release-please), Google's automated release management tool. This addresses the current problems of multiple release entrypoints, tool drift, and reliability edge cases while achieving the goal of fully automated releases.

### Key Benefits
- ✅ **Single release flow** - eliminates multiple "official" paths
- ✅ **Fully automated** - no manual version bumping or label management  
- ✅ **Battle-tested** - used by Google and thousands of open source projects
- ✅ **Conventional commits ready** - leverages existing `commitlint` enforcement
- ✅ **Multi-package coordination** - handles Rust workspace + npm package simultaneously
- ✅ **Audit trail** - explicit release PRs with generated changelogs

## Current State Analysis

### Existing Release Infrastructure

**Files that will be deprecated/replaced:**
- `.github/workflows/auto-release.yml` (label-driven automation) → **REMOVE**
- `scripts/release/semver-bump.sh` (custom semver logic) → **REMOVE** 
- `crates/blz-release` (custom Rust tooling) → **REMOVE**
- `justfile` `release-prep` (cargo-release path) → **REMOVE**
- `docs/development/workflow.md` release sections → **UPDATE**

**Files that will be preserved:**
- `.github/workflows/publish.yml` (build + publish pipeline) → **INTEGRATE**
- All publish sub-workflows (`publish-npm.yml`, `publish-crates.yml`, etc.) → **KEEP**

### Current Problems Solved
1. **Multiple tools conflict** (`cargo-release` vs `blz-release`) → Single tool
2. **Label reliability issues** → Commit-driven automation  
3. **Documentation drift** → Self-documenting config
4. **Manual recovery scenarios** → Standardized error handling

## Migration Strategy

### Phase 1: Setup and Configuration (Days 1-2)

**Goal:** Install release-please alongside existing system without disruption

#### Step 1.1: Create release-please configuration

**File: `.release-please-config.json`**
```json
{
  "bootstrap-sha": "8e63a6d",
  "release-type": "node", 
  "packages": {
    ".": {
      "release-type": "node",
      "package-name": "@outfitter/blz",
      "extra-files": [
        "Cargo.toml",
        "Cargo.lock"
      ]
    },
    "crates/blz-core": {
      "release-type": "rust",
      "package-name": "blz-core"
    },
    "crates/blz-cli": {
      "release-type": "rust", 
      "package-name": "blz-cli"
    },
    "crates/blz-mcp": {
      "release-type": "rust",
      "package-name": "blz-mcp"
    },
    "crates/blz-release": {
      "release-type": "rust",
      "package-name": "blz-release"
    }
  },
  "group-pull-request-title-pattern": "chore: release ${version}",
  "separate-pull-requests": false,
  "pull-request-title-pattern": "chore: release ${component} ${version}",
  "pull-request-header": "🤖 Release-please has created this PR to release new versions of the following packages:",
  "changelog-sections": [
    {"type": "feat", "section": "Features"},
    {"type": "fix", "section": "Bug Fixes"},
    {"type": "chore", "section": "Miscellaneous", "hidden": true},
    {"type": "docs", "section": "Documentation"},
    {"type": "style", "section": "Styles", "hidden": true},
    {"type": "refactor", "section": "Code Refactoring"},
    {"type": "perf", "section": "Performance Improvements"},
    {"type": "test", "section": "Tests", "hidden": true},
    {"type": "build", "section": "Build System", "hidden": true},
    {"type": "ci", "section": "Continuous Integration", "hidden": true}
  ]
}
```

#### Step 1.2: Initialize version manifest

**File: `.release-please-manifest.json`**
```json
{
  ".": "1.3.0",
  "crates/blz-core": "1.3.0", 
  "crates/blz-cli": "1.3.0",
  "crates/blz-mcp": "1.3.0",
  "crates/blz-release": "1.3.0"
}
```

**Notes:**
- Use current version (1.3.0) as starting point
- release-please will manage this file going forward
- All packages start synchronized

#### Step 1.3: Create release-please workflow

**File: `.github/workflows/release-please.yml`**
```yaml
name: Release Please

on:
  push:
    branches:
      - main
  workflow_dispatch:

permissions:
  contents: write
  pull-requests: write

jobs:
  release-please:
    runs-on: ubuntu-latest
    outputs:
      release_created: ${{ steps.release.outputs.release_created }}
      tag_name: ${{ steps.release.outputs.tag_name }}
      version: ${{ steps.release.outputs.version }}
      # Package-specific outputs for conditional publishing
      blz-core--release_created: ${{ steps.release.outputs['crates/blz-core--release_created'] }}
      blz-cli--release_created: ${{ steps.release.outputs['crates/blz-cli--release_created'] }}
      blz-mcp--release_created: ${{ steps.release.outputs['crates/blz-mcp--release_created'] }}
      npm--release_created: ${{ steps.release.outputs['--release_created'] }}
    steps:
      - name: Release Please
        uses: google-github-actions/release-please-action@v4
        id: release
        with:
          config-file: .release-please-config.json
          manifest-file: .release-please-manifest.json

  # Call existing publish workflow when release is created
  publish:
    needs: release-please
    if: ${{ needs.release-please.outputs.release_created }}
    uses: ./.github/workflows/publish.yml
    with:
      tag: ${{ needs.release-please.outputs.tag_name }}
      version: ${{ needs.release-please.outputs.version }}
      mode: 'full'
      # Skip individual packages if they weren't updated
      skip_crates: ${{ !needs.release-please.outputs.blz-core--release_created && !needs.release-please.outputs.blz-cli--release_created }}
      skip_npm: ${{ !needs.release-please.outputs.npm--release_created }}
    secrets: inherit
```

#### Step 1.4: Test in parallel

**Testing approach:**
1. Create test commits with conventional commit messages
2. Verify release-please creates proper PRs (but don't merge yet)
3. Validate version calculations and changelog generation
4. Test dry-run mode of publish workflow

**Commands for testing:**
```bash
# Create test branch for release-please testing
git checkout -b test/release-please-migration

# Make test commits
git commit -m "feat(cli): add test feature for release-please validation" --allow-empty
git commit -m "fix(core): resolve test issue for release-please validation" --allow-empty

# Push and observe release-please behavior
git push origin test/release-please-migration

# Manually trigger release-please workflow on test branch
gh workflow run release-please.yml --ref test/release-please-migration
```

### Phase 2: Parallel Operation (Days 3-5)

**Goal:** Run both systems simultaneously to validate release-please behavior

#### Step 2.1: Modify auto-release.yml

Add bypass condition to prevent conflicts:

```yaml
# Add to .github/workflows/auto-release.yml detect job
- name: Check for release-please override
  id: override
  run: |
    if [[ -f ".release-please-config.json" && "${{ github.event_name }}" == "push" ]]; then
      echo "skip=true" >> "$GITHUB_OUTPUT"
      echo "reason=release-please-active" >> "$GITHUB_OUTPUT"
    fi

# Update bump job condition
bump:
  needs: detect
  if: ${{ github.event_name == 'push' && needs.detect.outputs.skip == 'false' && needs.detect.outputs.override != 'true' }}
```

#### Step 2.2: Create monitoring workflow

**File: `.github/workflows/release-monitoring.yml`**
```yaml
name: Release System Monitor

on:
  schedule:
    - cron: '0 12 * * *'  # Daily at noon UTC
  workflow_dispatch:

jobs:
  monitor:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Check release system health
        run: |
          echo "## Release System Status" >> $GITHUB_STEP_SUMMARY
          echo "Date: $(date)" >> $GITHUB_STEP_SUMMARY
          
          # Check for pending release-please PRs
          if gh pr list --label "autorelease: pending" --json number,title; then
            echo "✅ Release-please PRs found" >> $GITHUB_STEP_SUMMARY
          else
            echo "ℹ️ No pending release PRs" >> $GITHUB_STEP_SUMMARY
          fi
          
          # Check version consistency
          CARGO_VERSION=$(awk -F '"' '/^version =/ {print $2; exit}' Cargo.toml)
          NPM_VERSION=$(jq -r .version package.json)
          
          if [[ "$CARGO_VERSION" == "$NPM_VERSION" ]]; then
            echo "✅ Versions synchronized: $CARGO_VERSION" >> $GITHUB_STEP_SUMMARY
          else
            echo "❌ Version mismatch: Cargo=$CARGO_VERSION, npm=$NPM_VERSION" >> $GITHUB_STEP_SUMMARY
          fi
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

#### Step 2.3: Validation testing

**Release-please validation:**
1. Create several test commits with different conventional commit types
2. Verify release PRs are created correctly  
3. Test merge of release PR triggers publish workflow
4. Validate all artifacts are built and published correctly

**Current system validation:**
1. Verify label-driven releases still work (but with bypass active)
2. Test edge cases like missing labels, unassociated PRs
3. Confirm no interference between systems

### Phase 3: Migration Cutover (Day 6)

**Goal:** Switch to release-please as the primary system

#### Step 3.1: Disable old system

**Actions:**
1. Remove/rename `auto-release.yml` to `auto-release.yml.disabled`
2. Add deprecation notice to justfile `release-prep` target
3. Update `.gitignore` to ignore `.semver-meta.json`

#### Step 3.2: Update documentation

**File: `docs/development/workflow.md`**
- Remove manual release sections
- Replace with release-please workflow documentation
- Add conventional commit guidelines
- Update troubleshooting section

**Key sections to add:**
```markdown
## Release Process

Releases are fully automated via [release-please](https://github.com/googleapis/release-please):

1. **Commit with conventional commit format:**
   ```bash
   git commit -m "feat(cli): add new search command"
   git commit -m "fix(core): resolve memory leak in indexer"
   ```

2. **Release-please analyzes commits and creates release PR**
3. **Merge release PR to trigger automated publishing**

### Version Bumps
- `feat:` → minor version bump
- `fix:` → patch version bump  
- `feat!:` or `BREAKING CHANGE:` → major version bump

### Manual Override
If needed, you can manually trigger releases:
```bash
gh workflow run release-please.yml
```
```

#### Step 3.3: Update contributing guidelines

Add conventional commit enforcement to contribution docs:

```markdown
## Commit Message Format

We use [Conventional Commits](https://www.conventionalcommits.org/) for automated releases:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Types:**
- `feat:` A new feature
- `fix:` A bug fix
- `docs:` Documentation changes
- `style:` Code style changes (formatting, etc.)
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `test:` Adding or modifying tests
- `chore:` Other changes (dependencies, etc.)

**Breaking changes:**
Use `feat!:` or `fix!:` or add `BREAKING CHANGE:` in footer.
```

### Phase 4: Cleanup (Day 7)

**Goal:** Remove deprecated code and finalize migration

#### Step 4.1: Remove deprecated files

```bash
# Remove old release automation
rm .github/workflows/auto-release.yml
rm scripts/release/semver-bump.sh
rm -rf crates/blz-release

# Update Cargo.toml workspace members
# Remove blz-release from workspace

# Clean up justfile
# Remove or update release-prep target
```

#### Step 4.2: Update dependencies

**File: `Cargo.toml`**
```toml
# Remove blz-release from workspace members
[workspace]
members = [
    "crates/blz-core",
    "crates/blz-cli", 
    "crates/blz-mcp",
    # Remove: "crates/blz-release"
]
```

#### Step 4.3: Final validation

**Test complete flow:**
1. Make commits with conventional commit messages
2. Verify release PR creation
3. Merge release PR  
4. Confirm all publish workflows complete successfully
5. Validate artifacts are published to all registries

## Implementation Timeline

| Day | Phase | Activities | Success Criteria |
|-----|-------|------------|------------------|
| 1 | Setup | Create configs, workflows | release-please workflow runs |
| 2 | Setup | Test parallel operation | Release PR created correctly |
| 3-4 | Parallel | Monitor both systems | Both systems work independently |
| 5 | Parallel | Validate integration | Publish workflow triggered by release-please |
| 6 | Cutover | Switch primary system | release-please is only active system |
| 7 | Cleanup | Remove deprecated code | Clean repository, docs updated |

## Risk Mitigation

### Risk 1: Release-please misconfiguration

**Likelihood:** Medium  
**Impact:** High (broken releases)

**Mitigation:**
- Extensive testing in parallel phase
- Validate configuration with release-please CLI tool
- Manual review of first several release PRs
- Keep old system available as backup during parallel phase

### Risk 2: Conventional commit adoption

**Likelihood:** Low  
**Impact:** Medium (reduced automation)

**Mitigation:**
- Already enforcing conventional commits via `commitlint`
- Update contribution guidelines and docs
- GitHub commit template with conventional format

### Risk 3: Multi-package version drift  

**Likelihood:** Low  
**Impact:** Medium (inconsistent versions)

**Mitigation:**
- Synchronized versioning in configuration
- Monitoring workflow to detect version mismatches
- Automated version synchronization in extra-files config

### Risk 4: Integration with existing publish workflow

**Likelihood:** Medium  
**Impact:** High (publish failures)

**Mitigation:**
- Minimal changes to existing publish.yml
- Test integration thoroughly in parallel phase
- Preserve manual workflow_dispatch triggers as fallback

## Rollback Plan

If critical issues are discovered:

### Immediate Rollback (< 1 hour)
1. Rename `auto-release.yml.disabled` back to `auto-release.yml`
2. Disable release-please workflow (add `if: false` condition)
3. Restore previous system operation

### Full Rollback (< 4 hours)  
1. Restore deleted files from git history:
   ```bash
   git checkout HEAD~n -- .github/workflows/auto-release.yml
   git checkout HEAD~n -- scripts/release/semver-bump.sh
   git checkout HEAD~n -- crates/blz-release/
   ```
2. Update Cargo.toml workspace members
3. Restore documentation
4. Remove release-please files

### Data Recovery
- Version information preserved in git tags
- Release history intact in GitHub Releases
- No data loss risk due to configuration-only changes

## Success Metrics

### Technical Metrics
- ✅ Single release workflow (1 active system)
- ✅ Zero manual version bumping
- ✅ 100% automated changelog generation  
- ✅ < 5 minute release cycle time (commit → published)
- ✅ Version synchronization across all packages

### Process Metrics
- ✅ Zero "why didn't it release?" issues
- ✅ Reduced release documentation maintenance
- ✅ Standard GitHub integration (no custom tools)
- ✅ Clear audit trail (explicit release PRs)

## Post-Migration Optimization

### Phase 5: Enhanced Automation (Optional)

After successful migration, consider these enhancements:

1. **Release notes enhancement:**
   - Custom release note templates
   - Automatic breaking change detection
   - Integration with Linear/GitHub issues

2. **Pre-release automation:**
   - Canary releases on feature branches
   - Beta releases for release candidates

3. **Quality gates:**
   - Block releases if tests fail
   - Require security audit passing
   - Performance regression detection

## Appendix A: Configuration Reference

### Conventional Commit Types
| Type | Version Bump | Description |
|------|--------------|-------------|
| `feat` | minor | New feature |
| `fix` | patch | Bug fix |
| `perf` | patch | Performance improvement |
| `refactor` | patch | Code refactoring |
| `docs` | none | Documentation only |
| `style` | none | Code style changes |
| `test` | none | Test changes |
| `chore` | none | Maintenance |
| `feat!` | major | Breaking feature |
| `fix!` | major | Breaking fix |

### Release-please Outputs
Available in workflow outputs for conditional logic:

```yaml
# Main release outputs
release_created: "true"/"false"  
tag_name: "v1.4.0"
version: "1.4.0"
major: "1"
minor: "4"  
patch: "0"
sha: "abc123..."
upload_url: "https://..."

# Package-specific outputs
"crates/blz-core--release_created": "true"/"false"
"crates/blz-cli--release_created": "true"/"false"  
"--release_created": "true"/"false"  # npm package (root)
```

## Appendix B: Troubleshooting

### Common Issues

**Issue:** Release-please doesn't create PR
**Solution:** Check conventional commit format and ensure commits since last release

**Issue:** Wrong version bump calculated  
**Solution:** Verify conventional commit type matches intended change scope

**Issue:** Multi-package version sync issues
**Solution:** Check extra-files configuration in .release-please-config.json

**Issue:** Publish workflow not triggered
**Solution:** Verify workflow_call inputs match release-please outputs

### Debug Commands

```bash
# Validate release-please config
npx release-please bootstrap --config-file .release-please-config.json

# Check what release-please would do
npx release-please --dry-run --config-file .release-please-config.json

# View current manifest state
cat .release-please-manifest.json

# Check conventional commits since last release
git log --oneline $(git describe --tags --abbrev=0)..HEAD
```

---

**This plan provides a comprehensive, step-by-step migration from the current label-based system to release-please automation, addressing all identified problems while maintaining the reliability and publish targets of the existing system.**