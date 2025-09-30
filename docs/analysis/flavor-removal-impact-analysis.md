# Flavor Removal Impact Analysis

**Date:** 2025-09-29
**Purpose:** Comprehensive analysis for simplifying BLZ to always prefer llms-full.txt when available

## Executive Summary

Current dual-flavor system causes significant complexity:
- **Issue #232**: Text formatter flavor mismatch bugs
- **Issue #2**: Pagination loses flavor context
- **Issue #4**: Hidden/confusing flavor flags
- **Issue #5**: Poor default flavor strategy

**Proposed Solution:** Remove dual-flavor complexity entirely. Always prefer llms-full.txt when available, fallback to llms.txt only when full not available. Never store both.

## Impact Analysis

### Files Requiring Changes: 47 files total

#### **Critical (Core Implementation): 8 files**
1. `crates/blz-core/src/types.rs` - Remove Flavor enum, normalize_flavor_filters()
2. `crates/blz-core/src/storage.rs` - Simplify to single flavor paths
3. `crates/blz-core/src/index.rs` - Remove flavor filtering, simplify deletion
4. `crates/blz-core/src/optimized_index.rs` - Remove flavor fields from schema
5. `crates/blz-core/src/fetcher.rs` - Always select llms-full.txt first
6. `crates/blz-core/src/cache.rs` - Remove flavor from cache keys
7. `crates/blz-cli/src/utils/flavor.rs` - Simplify to discovery only
8. `crates/blz-cli/src/utils/settings.rs` - Remove BLZ_PREFER_LLMS_FULL

#### **High Priority (CLI Commands): 7 files**
9. `crates/blz-cli/src/cli.rs` - Remove --flavor flags
10. `crates/blz-cli/src/main.rs` - Remove flavor preprocessing
11. `crates/blz-cli/src/commands/mod.rs` - Remove FlavorMode export
12. `crates/blz-cli/src/commands/search.rs` - Remove FlavorMode parameter
13. `crates/blz-cli/src/commands/get.rs` - Remove --flavor flag
14. `crates/blz-cli/src/commands/update.rs` - Remove FlavorMode, add upgrade logic
15. `crates/blz-cli/src/commands/add.rs` - Always prefer llms-full.txt

#### **Medium Priority (Tests): 3 files to delete**
16. `crates/blz-cli/tests/add_multi_flavor.rs` - DELETE (138 lines)
17. `crates/blz-cli/tests/list_flavor_resolution.rs` - DELETE (234 lines)
18. `crates/blz-cli/tests/search_flavor.rs` - DELETE (156 lines)

#### **Low Priority (Minor Updates): 29 files**
- 2 test files with minor assertion updates
- 16 test files with no changes needed
- 2 output formatters (remove flavor field)
- 5 command files with incidental references
- 4 documentation files

### Code Volume Estimates

- **Lines to delete:** ~2,800 lines
- **Lines to modify:** ~800 lines
- **New lines (upgrade command):** ~200 lines
- **Net reduction:** ~2,400 lines

## Detailed Component Analysis

### 1. Core Library (blz-core)

#### types.rs
**Remove:**
- `Flavor` enum (44 lines)
- `normalize_flavor_filters()` function (27 lines)
- `SearchHit.flavor` field

**Impact:** All flavor-aware code downstream breaks until migration complete.

#### storage.rs
**Current:** 15 flavor-aware functions
**After:** 3 simplified functions

| Current Function | After Refactor |
|-----------------|----------------|
| `flavor_from_url()` | DELETE |
| `flavor_file_name()` | DELETE |
| `flavor_json_filename()` | DELETE |
| `flavor_metadata_filename()` | DELETE |
| `flavor_file_path()` | → `content_file_path()` |
| `flavor_json_path()` | → `json_path()` |
| `save_flavor_content()` | → `save_content()` |
| `save_flavor_json()` | → `save_json()` |
| `load_flavor_json()` | → `load_json()` |
| `save_source_metadata_for_flavor()` | → `save_source_metadata()` |
| `load_source_metadata_for_flavor()` | → `load_source_metadata()` |
| `metadata_path_for_flavor()` | → `metadata_path()` |
| `available_flavors()` | DELETE |
| `exists_any_flavor()` | → `exists()` |

**New filename strategy:**
- Always try `llms-full.txt` first
- Fallback to `llms.txt` if full not available
- Store as single `llms.txt` internally (normalized name)

#### index.rs & optimized_index.rs
**Remove:**
- `flavor_field: Option<Field>` from schema
- `alias_flavor_field: Option<Field>` from schema
- Flavor filtering logic in `search()` (35 lines)
- Flavor-aware deletion in `index_blocks()` (30 lines)

**Simplify:**
- Delete by alias only (no flavor dimension)
- SearchHit no longer needs flavor field
- Cache keys no longer need flavor component

#### fetcher.rs
**Keep:** `check_flavors()` function but always return first available in priority order:
1. llms-full.txt
2. llms.txt
3. Other variants

**Remove:** FlavorInfo sorting complexity (caller just takes first)

### 2. CLI Layer

#### FlavorMode Enum
**Location:** `commands/update.rs:22-32`
**Status:** DELETE entirely

**Affected commands:**
- search (remove --flavor flag)
- get (remove --flavor flag)
- update (replace with simpler logic)

#### Constants
**Keep:** `BASE_FLAVOR` and `FULL_FLAVOR` constants during migration for backward compat reading old storage

**After migration:** Remove constants entirely

#### Settings
**Remove:**
- `BLZ_PREFER_LLMS_FULL` environment variable
- `prefer_llms_full` config setting
- `effective_prefer_llms_full()` function
- All per-source flavor override logic in blz.json

### 3. New Upgrade Command

**Signature:**
```rust
pub async fn execute_upgrade(
    alias: Option<String>,
    all: bool,
    yes: bool,
) -> Result<()>
```

**Behavior:**
1. List sources that have llms.txt but llms-full.txt is available upstream
2. Prompt user (unless --yes) to upgrade each
3. Fetch llms-full.txt
4. Re-index with new content
5. Delete old llms.txt data
6. Report success/failure per source

**CLI:**
```bash
blz upgrade              # Interactive, check all sources
blz upgrade react        # Upgrade specific source
blz upgrade --all --yes  # Non-interactive bulk upgrade
```

## Migration Strategy

### Phase 1: Core Storage Simplification (PR #1)
**Goal:** Make storage always prefer full, maintain backward compat for reads

**Changes:**
1. Update `fetcher.rs` to always return llms-full.txt first
2. Add `storage.rs` helper `resolve_best_available_file()`:
   - Check for `llms-full.txt` first
   - Fallback to `llms.txt`
   - Error if neither exists
3. Keep old flavor-aware read functions but mark deprecated
4. Add new simplified write functions

**Tests:** Existing tests should still pass (reading old data works)

### Phase 2: CLI Simplification (PR #2)
**Goal:** Remove all --flavor flags and FlavorMode

**Changes:**
1. Remove `--flavor` from search, get, update commands
2. Delete FlavorMode enum
3. Simplify all command implementations to use new storage API
4. Remove flavor preprocessing in main.rs

**Tests:** Delete flavor-specific tests, update remaining

### Phase 3: Index Schema Migration (PR #3)
**Goal:** Remove flavor fields from search index

**Changes:**
1. Bump index version to trigger rebuilds
2. Remove flavor_field and alias_flavor_field from schema
3. Simplify deletion logic (alias only)
4. Remove flavor filtering from search

**Tests:** Ensure searches still work, just without flavor dimension

### Phase 4: Add Upgrade Command (PR #4)
**Goal:** Provide migration path for existing users

**Changes:**
1. Implement `commands/upgrade.rs`
2. Add CLI command definition
3. Add interactive prompts
4. Add documentation

**Tests:** Integration tests for upgrade flows

### Phase 5: Cleanup & Documentation (PR #5)
**Goal:** Remove all deprecated code, update docs

**Changes:**
1. Remove deprecated storage functions
2. Remove BASE_FLAVOR/FULL_FLAVOR constants
3. Update all documentation
4. Update agent instructions
5. Remove BLZ_PREFER_LLMS_FULL from env docs

## Risk Areas

### 1. **Backward Compatibility**
**Risk:** Users with existing llms.txt data can't search

**Mitigation:**
- Phase 1 keeps read compatibility
- Upgrade command provides migration path
- Clear upgrade instructions in release notes

### 2. **Sites Without llms-full.txt**
**Risk:** Some sites only have llms.txt

**Mitigation:**
- Fetcher still supports llms.txt as fallback
- No user-visible change for these sources

### 3. **Breaking Changes**
**Risk:** Major version bump required

**Mitigation:**
- Clear in changelog this is breaking change
- Version bump to 0.5.0 or 1.0.0
- Migration guide in docs

### 4. **Index Rebuilds**
**Risk:** Large indexes need full rebuild

**Mitigation:**
- Automatic rebuild on schema version mismatch
- Progress indicators during rebuild
- Keep old index until new one ready

## Testing Strategy

### Unit Tests
- Storage path resolution (prefer full, fallback to base)
- Fetcher priority ordering
- Upgrade command logic

### Integration Tests
- Add sources (always gets best available)
- Update sources (auto-upgrades to full if available)
- Search across sources (no flavor dimension)
- Upgrade command (migration flows)

### Regression Tests
- Backward compat: reading old llms.txt data
- Graceful fallback when full unavailable
- Error handling for missing sources

### Manual Testing
- Fresh install workflow
- Upgrade from 0.4.x workflow
- Sources with only llms.txt
- Sources with both flavors
- Sources with neither flavor

## Benefits

### For Users
- **Simpler mental model**: No flavor concept to understand
- **No configuration needed**: Always get best docs available
- **Fewer errors**: No flavor mismatch bugs
- **Faster searches**: No flavor filtering overhead

### For Maintainers
- **Less code**: ~2,400 lines removed
- **Simpler logic**: Single code path for storage/indexing
- **Fewer bugs**: Remove entire class of flavor-related issues
- **Easier testing**: Fewer permutations to test

### For Performance
- **Smaller indexes**: No flavor field overhead
- **Simpler queries**: No flavor filtering
- **Reduced storage**: Never store both flavors
- **Faster cache lookups**: Simpler cache keys

## Open Questions

1. **Should we support custom flavor names?** (e.g., llms-preview.txt)
   - Proposal: No, only llms-full.txt and llms.txt

2. **What about sites that add llms-full.txt later?**
   - Proposal: `blz update` auto-detects and upgrades

3. **Should upgrade be automatic or opt-in?**
   - Proposal: Automatic on `blz update`, can skip with --no-upgrade flag

4. **Archive old llms.txt files?**
   - Proposal: Yes, use existing archive mechanism

5. **Provide rollback capability?**
   - Proposal: `blz downgrade` command using archive

## Implementation Checklist

### Before Starting
- [ ] File GitHub issue with this analysis
- [ ] Get approval on migration strategy
- [ ] Create milestone for v0.5.0 or v1.0.0

### PR #1: Storage Simplification
- [ ] Update fetcher to prefer llms-full.txt
- [ ] Add resolve_best_available_file() helper
- [ ] Mark old functions deprecated
- [ ] Verify backward compat
- [ ] Update unit tests

### PR #2: CLI Simplification
- [ ] Remove --flavor flags from all commands
- [ ] Delete FlavorMode enum
- [ ] Update command implementations
- [ ] Delete flavor-specific tests
- [ ] Update integration tests

### PR #3: Index Schema Migration
- [ ] Bump index version
- [ ] Remove flavor fields
- [ ] Simplify deletion logic
- [ ] Remove flavor filtering
- [ ] Test index rebuilds

### PR #4: Upgrade Command
- [ ] Implement upgrade command
- [ ] Add CLI definition
- [ ] Add interactive prompts
- [ ] Write integration tests
- [ ] Write documentation

### PR #5: Cleanup & Docs
- [ ] Remove deprecated code
- [ ] Remove constants
- [ ] Update all documentation
- [ ] Update agent instructions
- [ ] Write migration guide
- [ ] Update changelog

## Timeline Estimate

- **PR #1**: 1-2 days
- **PR #2**: 2-3 days
- **PR #3**: 1-2 days
- **PR #4**: 2-3 days
- **PR #5**: 1 day
- **Total**: 7-11 days

---

**Next Steps:**
1. Review this analysis with team
2. File GitHub issue with proposal
3. Get sign-off on breaking changes
4. Begin Phase 1 implementation