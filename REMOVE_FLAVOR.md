# Comprehensive Flavor Removal Plan

## Goal
Remove the confusing "flavor" concept entirely. Replace with simple model: **one URL per source, tags determine searchability**.

## Impact
- **~819 flavor references** across 31 files
- **~1,500-2,000 lines** of code to remove/refactor
- **4 test files** to delete, 1 to rename, several to update
- **Breaking change** for users with `--flavor` flags (tombstoned, so low risk)

## Implementation Phases

### Phase 1: Core Types & Storage (Foundation)
**Goal**: Simplify storage layer to single-file-per-source model

1. **Simplify Storage methods** (blz-core/src/storage.rs):
   - Replace `save_flavor_json()` calls with `save_json()`
   - Replace `load_flavor_json()` calls with `load_json()`
   - Replace `save_source_metadata_for_flavor()` with `save_metadata()`
   - Replace `load_source_metadata_for_flavor()` with `load_metadata()`
   - Remove: `available_flavors()`, `exists_any_flavor()`, `flavor_*` helper methods
   - Keep method signatures initially for backward compat, mark deprecated

2. **Remove Flavor enum** (blz-core/src/types.rs):
   - Delete `Flavor` enum (lines 70-111)
   - Delete `normalize_flavor_filters()` function
   - Update any code that imports `Flavor`

3. **Run blz-core tests** to catch immediate breakage

### Phase 2: Command Simplification (High Impact)
**Goal**: Remove FlavorMode from all commands, simplify logic

4. **Delete upgrade.rs entirely**:
   - `rm crates/blz-cli/src/commands/upgrade.rs`
   - Remove from `mod.rs` exports
   - Remove `Commands::Upgrade` from `cli.rs`
   - Remove route in `main.rs`

5. **Simplify add.rs**:
   - Remove multi-flavor fetching loop
   - Remove `discover_flavor_candidates()` calls
   - Single flow: fetch URL → parse → index → save to `llms.txt`
   - Always save to same filenames (no flavor variants)

6. **Simplify update.rs**:
   - Remove `FlavorMode` enum definition
   - Remove `FlavorPlan`, `FlavorSummary` structs
   - Remove flavor iteration loop
   - Single flow: check metadata → conditional fetch → reindex if changed

7. **Simplify search.rs**:
   - Remove `FlavorMode` parameter from `execute()`
   - Remove `select_flavor_for_search()` function
   - Always search what's in the index (already filters `is_index_only()` ✅)

8. **Simplify get.rs**:
   - Remove `FlavorMode` parameter
   - Always read from `llms.txt` (the only file)

9. **Simplify list.rs**:
   - Remove `searchFlavor` field from JSON output
   - Remove `flavors` array from output
   - Show: alias, url, tags, last_updated

10. **Simplify anchors.rs**:
    - Remove flavor resolution calls

### Phase 3: Utilities & CLI (Cleanup)
**Goal**: Remove flavor.rs and CLI flavor arguments

11. **Delete flavor.rs**:
    - `rm crates/blz-cli/src/utils/flavor.rs`
    - Remove `pub mod flavor;` from `utils/mod.rs`
    - Remove all `use crate::utils::flavor::*` imports
    - If any helpers are useful (like `build_llms_json`), move to appropriate module

12. **Remove CLI flavor arguments** (cli.rs):
    - Delete `flavor: Option<FlavorMode>` from Search, Get, Update commands
    - Remove related clap parsing attributes
    - Remove any flavor-related help text

13. **Update configuration** (settings.rs):
    - Remove `prefer_llms_full` field
    - Remove `BLZ_PREFER_LLMS_FULL` env var handling

### Phase 4: Tests (Validation)
**Goal**: Update/delete flavor-specific tests

14. **Delete flavor test files**:
    - `rm crates/blz-cli/tests/add_multi_flavor.rs`
    - `rm crates/blz-cli/tests/search_flavor.rs`
    - `rm crates/blz-cli/tests/list_flavor_resolution.rs`
    - `rm crates/blz-core/tests/integration_flavor_detection.rs`

15. **Rename & refactor force_prefer_full.rs**:
    - `git mv tests/force_prefer_full.rs tests/source_searchability.rs`
    - Remove all FORCE_PREFER_FULL references
    - Focus on testing: add → index → search (tag-based filtering)

16. **Update remaining tests**:
    - `config_command.rs` - remove flavor config tests
    - `list_status_json.rs` - remove `searchFlavor` field assertions
    - `search_pagination.rs` - remove flavor references

17. **Run full test suite** - ensure everything passes

### Phase 5: Fetcher Simplification
**Goal**: Remove multi-flavor discovery from fetcher

18. **Simplify Fetcher** (blz-core/src/fetcher.rs):
    - Delete `check_flavors()` method (~50 lines)
    - Delete `FlavorInfo` struct
    - Keep: `fetch()`, `fetch_with_cache()` (ETag support is good!)

### Phase 6: Documentation & Cleanup
**Goal**: Update docs to reflect new simplified model

19. **Delete flavor documentation**:
    - `rm docs/commands/upgrade.md`

20. **Update command docs**:
    - `docs/commands/add.md` - remove flavor mentions
    - `docs/commands/update.md` - remove flavor mentions
    - `docs/commands/search.md` - remove flavor mentions
    - `docs/commands/list.md` - update output format
    - `README.md` - remove flavor feature section

21. **Update benchmarks**:
    - `benches/performance_optimizations.rs`
    - `benches/search_performance.rs`

22. **Final verification**:
    - `cargo check --workspace`
    - `cargo test --workspace`
    - `cargo clippy --workspace`
    - Manual smoke tests: add, search, list, update

## Migration Notes for Users

### Breaking Changes
- **CLI**: `--flavor` flag removed (was already tombstoned)
- **Command**: `blz upgrade` removed entirely
- **Output**: `blz list` no longer shows `flavors` or `searchFlavor` fields
- **Config**: `prefer_llms_full` setting no longer recognized

### User Impact (Minimal)
- Existing sources continue to work (backward compatible storage read)
- On next `blz update`, sources will migrate to single-file storage
- Users using `--flavor` flags will get "unknown flag" errors (can remove from scripts)

### Benefits
- **Simpler mental model**: One URL, one file, no flavor confusion
- **Faster operations**: No multi-flavor checks or iterations
- **Clearer errors**: No "which flavor am I using?" questions
- **Smaller codebase**: ~2,000 lines removed

## Risk Assessment

**High Risk**:
- Storage layer changes (Phase 1) - most critical
- Command refactoring (Phase 2) - high complexity

**Medium Risk**:
- CLI argument removal (Phase 3) - user-facing
- Test updates (Phase 4) - must maintain coverage

**Low Risk**:
- Fetcher simplification (Phase 5)
- Documentation (Phase 6)

## Success Criteria
- ✅ All tests pass
- ✅ `cargo clippy` with zero warnings
- ✅ Can add, search, list, update sources without errors
- ✅ No references to "flavor" in user-facing output
- ✅ Codebase reduced by ~2,000 lines
