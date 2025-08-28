# PR Review Feedback - Implementation Plan

## Overview
This document tracks all CodeRabbit PR review feedback and our systematic plan to address them.

## PR Stack Structure
1. PR #6: feat/comprehensive-overhaul-and-rename (base)
2. PR #13: 08-23-fix_address_code_review_feedback_and_issues
3. PR #14: 08-23-docs_standardize_terminology_and_fix_examples_consistency_7_
4. PR #15: 08-23-fix_quality_code_quality_improvements_and_maintenance_issue_11_
5. PR #16: 08-23-fix_error-handling_improve_error_handling_and_unicode_safety_issue_9_ (top)

## Feedback by PR

### PR #16: Error Handling & Unicode Safety
**Branch:** 08-23-fix_error-handling_improve_error_handling_and_unicode_safety_issue_9_

#### Issue 1: Snippet Context Length
- **Location:** crates/blz-core/src/index.rs:360-381
- **Problem:** Fixed 50-char context can exceed max_len parameter
- **Fix:** Derive context from max_len, split budget evenly around match
- **Status:** PENDING

### PR #15: Code Quality Improvements  
**Branch:** 08-23-fix_quality_code_quality_improvements_and_maintenance_issue_11_
- **No review comments found**

### PR #14: Documentation Standardization
**Branch:** 08-23-docs_standardize_terminology_and_fix_examples_consistency_7_

#### Issue 1: Benchmark Registry Bug
- **Location:** crates/blz-core/benches/registry_search_performance.rs:20-26
- **Problem:** create_large_registry() discards synthetic entries, returns empty Registry::new()
- **Fix:** Add Registry::from_entries() or insert() method to populate registry
- **Status:** COMPLETED ✅

#### Issue 2: Compilation Error - Borrowed Temporary String
- **Location:** crates/blz-core/benches/registry_search_performance.rs:357-368
- **Problem:** &"javascript ".repeat(20) creates reference to temporary
- **Fix:** Remove & to own the String
- **Status:** COMPLETED ✅

#### Issue 3: Duplicate Dev Dependencies
- **Location:** crates/blz-core/Cargo.toml:53
- **Problem:** Both mockito and wiremock present, mockito unused
- **Fix:** Remove mockito dependency
- **Status:** COMPLETED ✅

#### Issue 4: URL Query/Fragment Handling
- **Location:** crates/blz-core/src/fetcher.rs:149-156
- **Problem:** Extension check fails for URLs with query/fragment
- **Fix:** Strip query/fragment before extension check
- **Status:** COMPLETED ✅

#### Issue 5: URL Base Extraction Fragility
- **Location:** crates/blz-core/src/fetcher.rs:222-234
- **Problem:** Unsafe string slicing for scheme detection
- **Fix:** Use safer approach with saturating_sub or URL parser
- **Status:** COMPLETED ✅

#### Issue 6: Incomplete Terminology Update
- **Location:** docs/mcp.md (multiple locations)
- **Problem:** Still using "cached" instead of "indexed" in many places
- **Fix:** Replace all instances of "cached" with "indexed"
- **Status:** COMPLETED ✅

### PR #13: Comprehensive Code Improvements
**Branch:** 08-23-fix_address_code_review_feedback_and_issues
- **No review comments found**

### PR #6: Comprehensive Overhaul
**Branch:** feat/comprehensive-overhaul-and-rename
- **Too many comments to process** - will need to check manually if needed

## Implementation Order

Based on the dependency stack, we need to fix from bottom to top to avoid merge conflicts:

1. **PR #14** (docs branch) - Fix all 6 issues
2. **PR #15** (quality branch) - No fixes needed
3. **PR #16** (error-handling branch) - Fix 1 issue

## Execution Plan

### Phase 1: PR #14 Fixes
1. Checkout branch: `08-23-docs_standardize_terminology_and_fix_examples_consistency_7_`
2. Fix benchmark registry population issue
3. Fix borrowed temporary string compilation error
4. Remove unused mockito dependency
5. Fix URL query/fragment handling
6. Fix URL base extraction
7. Update documentation terminology
8. Commit and push
9. Comment on PR with changes

### Phase 2: PR #15 Check
1. Checkout branch: `08-23-fix_quality_code_quality_improvements_and_maintenance_issue_11_`
2. Verify no changes needed
3. Move to next PR

### Phase 3: PR #16 Fixes
1. Checkout branch: `08-23-fix_error-handling_improve_error_handling_and_unicode_safety_issue_9_`
2. Fix snippet context length issue
3. Commit and push
4. Comment on PR with changes

## Notes for Subagents

When working on fixes:
- Ensure you're in the correct branch before making changes
- Test changes locally before committing
- Use descriptive commit messages
- Update this document with completion status
- Leave detailed PR comments explaining the fixes