# Flavor-Related Test Analysis for blz-cli

**Date:** 2025-09-30
**Purpose:** Document all tests related to dual-flavor support (llms.txt vs llms-full.txt) to understand impact of removing dual-flavor functionality.

---

## Executive Summary

**Total test files analyzed:** 22
**Files with flavor-specific tests:** 3 (dedicated)
**Files with flavor-related assertions:** 6 (incidental)
**Total test functions:** 41
**Flavor-specific test functions:** 6

### Critical Findings

1. **3 test files are entirely dedicated to multi-flavor behavior** and would need complete rewrites or removal
2. **1 test file** tests configuration management around `add.prefer_full` setting
3. **Several integration tests** make incidental references to "llms" but don't test dual-flavor behavior
4. Most tests use `BLZ_PREFER_LLMS_FULL=0` to ensure deterministic behavior, not to test flavor switching

---

## Category 1: Dedicated Flavor Test Files

### 🔴 CRITICAL: `/Users/mg/Developer/outfitter/blz/crates/blz-cli/tests/add_multi_flavor.rs`

**Purpose:** Tests that `blz add` fetches and indexes both llms.txt and llms-full.txt when available

**Test Functions:**
- `add_fetches_all_discovered_flavors()` (async)

**What it validates:**
- HEAD requests to both `/llms.txt` and `/llms-full.txt` endpoints
- GET requests fetch both flavors with different content
- Add command output shows summary for both flavors:
  - `"llms: "`
  - `"llms-full: "`
- File creation for both flavors:
  - `llms.txt`, `llms-full.txt`
  - `llms.json`, `llms-full.json`
  - `metadata.json`, `metadata-llms-full.json`
- Index directory `.index/` is populated
- `blz list --format json` includes both flavors in `flavors` array
- `searchFlavor` field resolves correctly based on `BLZ_PREFER_LLMS_FULL` env var

**Flavor-specific logic:**
- Tests dual-flavor discovery mechanism
- Validates storage structure for both flavors
- Tests flavor resolution in list output

**Impact if dual-flavor removed:**
- **ENTIRE FILE BECOMES OBSOLETE** - no dual-flavor to test
- Could be replaced with single-flavor equivalent testing basic add flow
- ~138 lines would need rewrite or removal

---

### 🔴 CRITICAL: `/Users/mg/Developer/outfitter/blz/crates/blz-cli/tests/list_flavor_resolution.rs`

**Purpose:** Tests flavor resolution logic in `blz list` command with preferences

**Test Functions:**
1. `list_resolves_flavor_with_preferences()` (async)
2. `list_handles_missing_flavors_gracefully()` (async)
3. `list_jsonl_format_includes_search_flavor()` (async)

**What each test validates:**

#### Test 1: `list_resolves_flavor_with_preferences()`
- Mocks both llms.txt (404B) and llms-full.txt (2048B)
- Adds source, fetches both flavors
- **Without `BLZ_PREFER_LLMS_FULL`:** `searchFlavor` = `"llms"`, `defaultFlavor` = `"llms"`
- **With `BLZ_PREFER_LLMS_FULL=1`:** `searchFlavor` = `"llms-full"`, `defaultFlavor` = `"llms-full"`
- Validates both flavors appear in `flavors` array
- Tests preference-based flavor selection

#### Test 2: `list_handles_missing_flavors_gracefully()`
- Mocks only llms.txt (llms-full.txt returns 404)
- Validates graceful fallback when llms-full unavailable
- `searchFlavor` = `"llms"` (only option)
- `flavors` array has single entry
- Tests resilience when dual-flavor unavailable

#### Test 3: `list_jsonl_format_includes_search_flavor()`
- Tests JSONL output format includes `searchFlavor` and `defaultFlavor`
- Validates consistency across output formats

**Flavor-specific logic:**
- Core flavor resolution testing
- Preference system (`BLZ_PREFER_LLMS_FULL`)
- Fallback behavior when flavors missing
- `searchFlavor` vs `defaultFlavor` field semantics

**Impact if dual-flavor removed:**
- **ENTIRE FILE BECOMES OBSOLETE**
- No flavor resolution to test with single flavor
- Could retain test 2 as "basic list JSON schema validation" but would need rewrite
- ~234 lines would need rewrite or removal

---

### 🔴 CRITICAL: `/Users/mg/Developer/outfitter/blz/crates/blz-cli/tests/search_flavor.rs`

**Purpose:** Tests search behavior with different flavor preferences and overrides

**Test Functions:**
1. `search_defaults_to_base_flavor_and_respects_override()` (async)

**What it validates:**
- Mocks both flavors with different content:
  - Base: `"base-only insight"`
  - Full: `"full-only expansion"`
- **Test Scenario 1:** Search for `"full-only"` without preference
  - Expected: 0 results (searches base flavor by default)
- **Test Scenario 2:** Search with `--flavor full` flag
  - Expected: Results from llms-full.txt
- **Test Scenario 3:** Run `blz update docs --flavor full` to change preference
  - Expected: Future searches return full-flavor results
- **Test Scenario 4:** Force base flavor with `--flavor txt` despite full preference
  - Expected: 0 results (back to base flavor)

**Flavor-specific logic:**
- Default flavor selection
- `--flavor` flag override (values: `full`, `txt`)
- Persistent flavor preference via update
- Flag takes precedence over stored preference

**Impact if dual-flavor removed:**
- **ENTIRE FILE BECOMES OBSOLETE**
- No flavor switching or override to test
- ~156 lines would need removal

---

## Category 2: Configuration Management Test

### 🟡 MODERATE: `/Users/mg/Developer/outfitter/blz/crates/blz-cli/tests/config_command.rs`

**Purpose:** Tests configuration scoping (global/project/local) and `add.prefer_full` setting

**Test Functions:**
1. `config_command_manages_scopes()` (sync)
2. `add_respects_prefer_full_setting()` (async)

**What each test validates:**

#### Test 1: `config_command_manages_scopes()`
- Sets `add.prefer_full` to different values across scopes:
  - Global: `true`
  - Project: `false`
  - Local: `true`
- Validates `blz config get` shows all scopes and effective value
- Tests config precedence (local > project > global)

#### Test 2: `add_respects_prefer_full_setting()`
- Mocks both llms.txt and llms-full.txt
- **With `add.prefer_full=true`:** Add output contains both "llms-full" and "llms"
- **With `add.prefer_full=false`:** Add output contains "llms" only
- Validates storage files match preference:
  - `fullpref` source has `llms-full.json` with `llms-full.txt` path
  - `basepref` source has `llms.json` with `llms.txt` path

**Flavor-specific logic:**
- `add.prefer_full` configuration setting
- Controls which flavor is default during add operation

**Impact if dual-flavor removed:**
- **Test 1:** `add.prefer_full` setting becomes meaningless, test would need removal or replacement
- **Test 2:** Entire test becomes obsolete (nothing to prefer)
- Could replace with tests for other config settings if any remain
- ~98 lines affected (config_command specific tests)

---

## Category 3: Incidental Flavor References

These tests don't test multi-flavor behavior but reference "llms" in URLs, filenames, or environment variables for test setup.

### 🟢 LOW IMPACT: Tests with Incidental References

#### `/Users/mg/Developer/outfitter/blz/crates/blz-cli/tests/list_status_json.rs`
- **Test:** `list_status_json_includes_source_and_keys()`
- **Flavor references:**
  - Checks `flavors[0]["flavor"] == "llms"` in JSON output
  - Validates `searchFlavor == "llms"`
- **Impact:** Minimal - assertions would change from checking `flavors` array to simpler schema

#### `/Users/mg/Developer/outfitter/blz/crates/blz-cli/tests/search_pagination.rs`
- **Tests:** 7 pagination edge-case tests
- **Flavor references:**
  - Uses `BLZ_PREFER_LLMS_FULL=0` for determinism (not testing flavor)
  - Has assertion allowing: `"Flavor filtering requested (llms) but index schema has no flavor field"`
- **Impact:** Minimal - remove flavor-specific warning assertions, keep pagination logic

#### `/Users/mg/Developer/outfitter/blz/crates/blz-cli/tests/anchors_e2e.rs`
- **Test:** `add_update_generates_anchors_mapping()`
- **Flavor references:** Uses `/llms.txt` in mock URL setup
- **Impact:** None - just uses "llms.txt" as filename, not testing dual-flavor

#### `/Users/mg/Developer/outfitter/blz/crates/blz-cli/tests/pipe_detection.rs`
- **Tests:** 4 tests for auto-JSON when piped
- **Flavor references:** Uses `/llms.txt` URLs in mocks
- **Impact:** None - filename is incidental

#### `/Users/mg/Developer/outfitter/blz/crates/blz-cli/tests/preflight_more.rs`
- **Tests:** 2 tests for HEAD request preflight validation
- **Flavor references:** Uses `/llms.txt` URLs
- **Impact:** None

#### `/Users/mg/Developer/outfitter/blz/crates/blz-cli/tests/score_display.rs`
- **Tests:** 5 tests for score display formatting
- **Flavor references:** Uses `/llms.txt` URLs, helper function `serve_llms()`
- **Impact:** None

#### Other files with minimal references:
- `alias_add_rm.rs` - uses "llms" in mock URLs (no flavor logic)
- `alias_resolver_update_remove.rs` - same
- `anchor_get.rs` - same
- `batch_get.rs` - same
- `history.rs` - same
- `search_json.rs` - same
- `search_next_flag.rs` - same
- `search_phrase.rs` - same

---

## Mock Data and Test Fixtures

### Flavor-Specific Mock Patterns

Most flavor tests use this pattern:

```rust
// Mock both flavors with different content
let base_doc = "# Docs\n\nBase content";
let full_doc = "# Docs\n\nFull content with more";

// HEAD for discovery
Mock::given(method("HEAD"))
    .and(path("/llms.txt"))
    .respond_with(ResponseTemplate::new(200))
    .mount(&server).await;

Mock::given(method("HEAD"))
    .and(path("/llms-full.txt"))
    .respond_with(ResponseTemplate::new(200))
    .mount(&server).await;

// GET for content
Mock::given(method("GET"))
    .and(path("/llms.txt"))
    .respond_with(ResponseTemplate::new(200).set_body_string(base_doc))
    .mount(&server).await;

Mock::given(method("GET"))
    .and(path("/llms-full.txt"))
    .respond_with(ResponseTemplate::new(200).set_body_string(full_doc))
    .mount(&server).await;
```

**Impact if dual-flavor removed:**
- All dual-mock patterns simplify to single `/llms.txt` endpoint
- No need for HEAD discovery of multiple flavors
- Single GET per source

---

## Summary Table: Test Impact Analysis

| Test File | Test Functions | Flavor Testing | Impact | Lines Affected | Action Required |
|-----------|----------------|----------------|--------|----------------|-----------------|
| `add_multi_flavor.rs` | 1 | ✅ Core | 🔴 **Critical** | ~138 | **Remove or full rewrite** |
| `list_flavor_resolution.rs` | 3 | ✅ Core | 🔴 **Critical** | ~234 | **Remove or full rewrite** |
| `search_flavor.rs` | 1 | ✅ Core | 🔴 **Critical** | ~156 | **Remove entirely** |
| `config_command.rs` | 2 | ✅ Partial | 🟡 **Moderate** | ~98 | **Remove prefer_full tests** |
| `list_status_json.rs` | 1 | ❌ Incidental | 🟢 **Low** | ~5 | Update assertions |
| `search_pagination.rs` | 7 | ❌ Incidental | 🟢 **Low** | ~3 | Remove warning checks |
| Other 16 files | 26 | ❌ None | 🟢 **Minimal** | 0 | No changes (URLs only) |

**Totals:**
- **528+ lines** of flavor-specific test code to remove/rewrite
- **10 test functions** entirely dedicated to flavor behavior (would be deleted)
- **16 test files** unaffected (incidental references only)

---

## Test Execution Strategy for Removal

### Phase 1: Identify Breakage
Run test suite after flavor removal to identify failures:
```bash
cargo test -p blz-cli 2>&1 | grep -i "flavor\|prefer\|llms-full"
```

### Phase 2: Remove Dedicated Tests
1. **Delete entirely:**
   - `tests/add_multi_flavor.rs`
   - `tests/list_flavor_resolution.rs`
   - `tests/search_flavor.rs`

2. **Remove from `config_command.rs`:**
   - `config_command_manages_scopes()` - remove `add.prefer_full` test
   - `add_respects_prefer_full_setting()` - delete entire function

### Phase 3: Update Incidental References
1. **`list_status_json.rs`:**
   - Remove `flavors` array assertions
   - Remove `searchFlavor` field assertions
   - Update JSON schema expectations

2. **`search_pagination.rs`:**
   - Remove `"Flavor filtering requested (llms)"` from acceptable stderr patterns
   - Keep all pagination logic unchanged

### Phase 4: Simplify Mock Patterns
- Replace dual-flavor mock setup with single `/llms.txt` endpoint
- Remove HEAD discovery patterns for flavor detection
- Simplify storage assertions (no `llms-full.json`, `metadata-llms-full.json`)

### Phase 5: Update Environment Variables
- Remove all `BLZ_PREFER_LLMS_FULL` environment variable usage
- Tests currently use it for determinism, would no longer be needed

---

## Environment Variables Used in Tests

### `BLZ_PREFER_LLMS_FULL`
- **Used in:** 3 files (add_multi_flavor, list_flavor_resolution, search_flavor)
- **Purpose:** Control flavor preference for deterministic testing
- **Impact:** Variable becomes meaningless, remove all usages

### `BLZ_DATA_DIR`
- **Used in:** All test files
- **Purpose:** Isolate test storage
- **Impact:** None (unrelated to flavor)

### `BLZ_CONFIG_DIR`
- **Used in:** Many test files
- **Purpose:** Isolate test configuration
- **Impact:** None (unrelated to flavor)

---

## Storage File Assertions Affected

### Current Dual-Flavor Storage
Tests verify these files exist after `blz add`:
```
<alias>/
  llms.txt
  llms-full.txt
  llms.json
  llms-full.json
  metadata.json
  metadata-llms-full.json
  .index/
```

### Single-Flavor Storage (after removal)
```
<alias>/
  llms.txt
  llms.json
  metadata.json
  .index/
```

**Tests affected:**
- `add_multi_flavor.rs` - checks for 6 files, would check for 3
- `config_command.rs::add_respects_prefer_full_setting()` - loads `llms-full.json`, would only load `llms.json`

---

## JSON Schema Changes in Tests

### Current List JSON Schema (with flavors)
```json
{
  "alias": "source-name",
  "url": "https://example.com/llms.txt",
  "searchFlavor": "llms",
  "defaultFlavor": "llms",
  "flavors": [
    {
      "flavor": "llms",
      "filename": "llms.txt",
      "lines": 150,
      "size": 4096
    },
    {
      "flavor": "llms-full",
      "filename": "llms-full.txt",
      "lines": 500,
      "size": 12288
    }
  ]
}
```

### Future List JSON Schema (single flavor)
```json
{
  "alias": "source-name",
  "url": "https://example.com/llms.txt",
  "lines": 150,
  "size": 4096,
  "filename": "llms.txt"
}
```

**Tests affected:**
- All tests parsing list JSON with `flavors` array
- All tests checking `searchFlavor` or `defaultFlavor` fields

---

## Recommendations

### Immediate Actions
1. **Delete 3 dedicated flavor test files** (~528 lines) - no salvage value
2. **Remove flavor-related config tests** from `config_command.rs`
3. **Simplify list JSON assertions** in `list_status_json.rs`
4. **Clean up pagination warning checks** in `search_pagination.rs`

### Optional Follow-up
1. **Add single-flavor coverage tests** to replace some deleted tests:
   - Test basic add/list/search flow without flavor complexity
   - Validate simplified storage structure
   - Test missing llms.txt graceful failure

2. **Consolidate mock patterns:**
   - Create shared helper: `serve_single_llms_txt(content: &str)`
   - Reduces duplication in remaining tests

### Testing Strategy
After removal:
```bash
# Verify no broken tests
cargo test -p blz-cli

# Verify no references remain
rg -i "llms-full|prefer_full|searchFlavor|defaultFlavor" crates/blz-cli/tests/

# Check for orphaned env vars
rg "BLZ_PREFER_LLMS_FULL" crates/blz-cli/tests/
```

---

## Appendix: Complete Test Function Inventory

### Flavor-Specific Test Functions (10 total)

1. `add_multi_flavor::add_fetches_all_discovered_flavors()` - 🔴 Delete
2. `list_flavor_resolution::list_resolves_flavor_with_preferences()` - 🔴 Delete
3. `list_flavor_resolution::list_handles_missing_flavors_gracefully()` - 🔴 Delete
4. `list_flavor_resolution::list_jsonl_format_includes_search_flavor()` - 🔴 Delete
5. `search_flavor::search_defaults_to_base_flavor_and_respects_override()` - 🔴 Delete
6. `config_command::config_command_manages_scopes()` - 🟡 Modify (remove prefer_full)
7. `config_command::add_respects_prefer_full_setting()` - 🔴 Delete
8. `list_status_json::list_status_json_includes_source_and_keys()` - 🟢 Update assertions
9. `search_pagination::*` (7 functions) - 🟢 Remove warning checks

### Unaffected Test Functions (26 total)
All other test functions only reference "llms" in URLs/filenames and require no changes beyond mock simplification.

---

## Conclusion

Removing dual-flavor support will impact **4 test files significantly** and require deletion of **~528 lines of test code**. The impact is well-contained to flavor-specific tests, with most integration tests unaffected beyond minor assertion updates.

The test suite will actually become **simpler and more maintainable** after removal, eliminating complexity around:
- Flavor discovery logic
- Preference resolution
- Dual-index management
- Configuration scoping for flavor selection

**Estimated effort:** 2-3 hours to remove tests, update assertions, and verify full test suite passes.