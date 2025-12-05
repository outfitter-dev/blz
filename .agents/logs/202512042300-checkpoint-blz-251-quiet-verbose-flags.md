---
date: 2024-12-04T23:00:00Z
branch: blz-251-add-consistent-quiet-verbose-global-flags
slug: blz-251-quiet-verbose-flags
type: checkpoint
status: abandoned
linear: BLZ-251
---

# BLZ-251: Consistent Quiet/Verbose Global Flags

## Summary

This branch attempted to add consistent `--quiet` and `--verbose` global flags across all CLI commands. The work was abandoned because:

1. **Build is broken** - Missing dependencies in Cargo.toml that are still referenced in code
2. **Mixed concerns** - Combined feature work with unrelated large refactors (removing TOC, language filtering)
3. **Incomplete implementation** - Only ~40% complete

## What Was Being Implemented

### Core Feature: Global `--quiet`/`-q` and `--verbose`/`-v` Flags

**Changes to `cli.rs`:**
```rust
#[arg(short = 'v', long, global = true, conflicts_with = "quiet")]
pub verbose: bool,

/// Suppress informational messages (only show errors)
#[arg(short = 'q', long, global = true, conflicts_with = "verbose")]
pub quiet: bool,
```

### Key Implementation Pattern

The branch introduced `FormatArg::resolve_with_quiet()` in `cli_args.rs`:

```rust
/// Returns (OutputFormat, canonical_quiet_state)
pub fn resolve_with_quiet(quiet: bool) -> (OutputFormat, bool) {
    // Returns both format and final quiet state
    // Smart TTY detection for piped vs interactive output
    // Deprecation warning control with BLZ_SUPPRESS_DEPRECATIONS env var
}
```

### Commands Updated

- `search.rs`: Added `quiet` parameter to `SearchOptions` struct
- `update.rs` (now `refresh.rs`): Integrated quiet flag support
- `lib.rs`: Updated command handlers to use `resolve_with_quiet()`

## Why It Failed

1. **Removed dependencies from Cargo.toml still used in code:**
   - `html-escape = "0.2"`
   - `memchr = "2"`
   - `unicode-normalization = "0.1"`
   - These are referenced in `crates/blz-core/src/heading.rs`

2. **Large unrelated deletions mixed in:**
   - Removed entire TOC/anchors command (~2000+ lines)
   - Removed language filtering code
   - Removed config management
   - These should have been separate PRs

3. **Incomplete integration:**
   - Not all commands respect quiet flag
   - Verbose flag behavior not fully defined
   - No tests for new behavior

## Recommendation for Future Implementation

If revisiting this feature:

1. **Start fresh from current `main`**
2. **Focus ONLY on quiet/verbose flags** - no other changes
3. **Implement systematically:**
   - Add global flags to `Cli` struct
   - Create `resolve_with_quiet()` utility
   - Update each command one at a time
   - Add tests for each command's quiet/verbose behavior
4. **Target ~150-200 LOC** for a focused, reviewable PR

## Files That Would Need Changes

- `crates/blz-cli/src/cli.rs` - Add global flags
- `crates/blz-cli/src/utils/cli_args.rs` - Add `resolve_with_quiet()`
- `crates/blz-cli/src/lib.rs` - Plumb quiet flag to command handlers
- `crates/blz-cli/src/commands/*.rs` - Each command needs quiet support

## Commit Reference

```
01e94411 feat(cli): standardize quiet and verbose behavior
```

Branch abandoned: 2024-12-04
