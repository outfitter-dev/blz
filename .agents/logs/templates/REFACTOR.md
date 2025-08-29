---
date: # e.g. 2025-08-29 21:00 UTC
scope: # e.g. storage-paths, parser-module
agent: # e.g. claude, codex, cursor, etc.
---

# Refactor - [Area/Component]

## Scope

## Motivation

## Approach

## Changes

### Structure Changes

### API Changes

### Behavior Changes

## Migration Required

## Risks

## Testing

## Rollout Plan

---

## Example

```markdown
---
date: 2025-08-29 21:00 UTC
scope: storage-paths
agent: claude
---

# Refactor - Storage Path Unification

## Scope

Unify all storage and configuration paths to use `~/.outfitter/blz` consistently across the codebase.

## Motivation

- Current paths inconsistent between docs and code
- Some components use `~/.cache/outfitter`, others use platform dirs
- User confusion about where data is stored
- Difficult to debug storage issues

## Approach

1. Audit all path references in codebase
2. Update to use consistent ProjectDirs configuration
3. Add migration for existing installations
4. Update documentation

## Changes

### Structure Changes

- `StorageConfig::base_dir()` now returns `~/.outfitter/blz`
- Removed hardcoded paths from individual components
- Centralized path resolution in `storage.rs`

### API Changes

- `Storage::new()` now takes optional base path override
- Added `Storage::migrate_from_legacy()` method
- Deprecated `Storage::cache_dir()` in favor of `Storage::data_dir()`

### Behavior Changes

- First run checks for legacy paths and offers migration
- All new installations use unified path
- Logs migration actions for debugging

## Migration Required

Users with existing installations will see:
```
Found existing data at ~/.cache/outfitter
Migrate to ~/.outfitter/blz? [Y/n]
```

## Risks

- Data loss if migration fails (mitigated by copying, not moving)
- Disk space during migration (need 2x space temporarily)
- Breaking change for scripts expecting old paths

## Testing

- Unit tests for path resolution
- Integration test for migration flow
- Manual testing on macOS, Linux, Windows
- Verify with fresh install and upgrade scenarios

## Rollout Plan

1. Release as v0.2.0 (breaking change)
2. Add prominent migration notes to release
3. Keep legacy path support for 2 versions
4. Remove legacy code in v0.4.0
```