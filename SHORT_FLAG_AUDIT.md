# Short Flag Consistency Audit (BLZ-113)

## Summary

This document audits all short flags across the blz CLI to identify inconsistencies and propose fixes.

## Current Short Flag Usage

### Global Flags (Available in all commands)
- `-v, --verbose` - Enable verbose output
- `-q, --quiet` - Suppress informational messages

### FormatArg (Shared across many commands)
- `-f, --format` - Output format (json, text, jsonl, raw)
- `--json` - Convenience flag for `--format json`
- `-o, --output` - **DEPRECATED** Hidden alias for `--format`

### Command-Specific Flags

#### Search
- `-s, --source` - Filter by source(s)
- `-n, --limit` - Maximum number of results (conflicts with `--all`)
- No short flag for `--context` (inconsistent with Get command which has `-c`)

#### Get
- `-l, --lines` - Line range to retrieve
- `-c, --context` - Context lines or 'all' for full section

#### Update/Remove/Clear
- `-y, --yes` - Skip confirmation prompt (Update, Remove)
- `-f, --force` - Skip confirmation prompt (Clear)

#### Anchors (deprecated command, line 250-266)
- `-o, --output` - Output format ← **CONFLICTS with FormatArg's deprecated `-o`**

#### Anchor Subcommands
- `anchor list`:
  - `-o, --output` - Output format ← **CONFLICTS with FormatArg's deprecated `-o`**
- `anchor get`:
  - `-c, --context` - Context lines
  - `-o, --output` - Output format ← **CONFLICTS with FormatArg's deprecated `-o`**

#### Registry create-source
- `-y, --yes` - Skip confirmation prompts

#### Add
- `-y, --yes` - Apply without prompting

## Issues Identified

### 1. Output Format Flag Conflicts
**Problem**: The `-o` short flag is used in multiple places:
- `FormatArg` uses `-o` as a deprecated alias for `--format`
- `Anchors` command (line 256) uses `-o` for `--output`
- `anchor list` subcommand (line 716) uses `-o` for `--output`
- `anchor get` subcommand (line 738) uses `-o` for `--output`

**Impact**: If `Anchors` or `anchor` subcommands migrate to use `FormatArg`, there will be conflicts.

**Root Cause**: The `Anchors` command and `anchor` subcommands predate the introduction of `FormatArg` and use their own format handling.

### 2. Context Flag Inconsistency
**Problem**: The `-c` short flag for `--context` is:
- Available in `Get` command (line 436)
- Available in `anchor get` subcommand (line 734)
- **NOT available** in `Search` command (only `--context` long form)

**Impact**: Inconsistent UX - users expect `-c` to work everywhere context is supported.

### 3. Yes/Force Flag Inconsistency
**Problem**: Confirmation skipping uses different flags:
- `-y, --yes` in Update, Remove, Add, and registry create-source
- `-f, --force` in Clear command

**Impact**: Inconsistent UX - users must remember which flag works with which command.

## Proposed Fixes

### Fix 1: Migrate Anchors commands to FormatArg
**Change**: Update `Anchors` command and `anchor` subcommands to use `FormatArg` instead of custom `-o` flag.

**Benefits**:
- Eliminates `-o` conflict
- Provides consistent format handling across all commands
- Gives access to `--json` convenience flag
- Respects `BLZ_OUTPUT_FORMAT` environment variable

**Implementation**:
```rust
// Before (Anchors command, line 251-262)
Anchors {
    alias: String,
    #[arg(short = 'o', long, value_enum, default_value = "text", env = "BLZ_OUTPUT_FORMAT")]
    output: OutputFormat,
    #[arg(long)]
    mappings: bool,
}

// After
Anchors {
    alias: String,
    #[command(flatten)]
    format: FormatArg,
    #[arg(long)]
    mappings: bool,
}
```

Apply similar change to `anchor list` and `anchor get` subcommands.

### Fix 2: Add `-c` short flag to Search command
**Change**: Add `-c` as a short flag for `--context` in Search command (line 377).

**Benefits**:
- Consistent with Get command behavior
- Matches grep-style conventions (`-c` for context)
- Improves UX for frequent users

**Implementation**:
```rust
// Before (line 377)
#[arg(
    long = "context",
    value_name = "LINES|all",
    num_args = 0..=1,
    default_missing_value = "5",
    conflicts_with = "block"
)]
context: Option<ContextMode>,

// After
#[arg(
    short = 'c',
    long = "context",
    value_name = "LINES|all",
    num_args = 0..=1,
    default_missing_value = "5",
    conflicts_with = "block"
)]
context: Option<ContextMode>,
```

### Fix 3: Standardize confirmation skipping (OPTIONAL)
**Option A**: Keep as-is (different semantics justify different flags)
- `-y, --yes` for commands that modify sources (Update, Remove, Add)
- `-f, --force` for destructive command (Clear)

**Option B**: Standardize on `-y, --yes` everywhere
- Change Clear to use `-y, --yes` instead of `-f, --force`
- More consistent but loses semantic distinction

**Recommendation**: Keep Fix 3 out of scope for BLZ-113. The semantic distinction between "yes" (confirmation) and "force" (destructive operation) is valuable.

## Implementation Plan

1. **Fix 1**: Migrate TOC commands to FormatArg
   - Update `Toc` command struct
   - Update `AnchorCommands::List` struct
   - Update `AnchorCommands::Get` struct
   - Update corresponding handler code in `commands/toc.rs`
   - Update tests

2. **Fix 2**: Add `-c` to Search context flag
   - Add `short = 'c'` to Search command context arg
   - Update integration tests
   - Update documentation

3. **Testing**:
   - Run full test suite
   - Test all affected commands manually
   - Verify backward compatibility (deprecated `-o` still works with warning)

4. **Documentation**:
   - Update help text where needed
   - Add changelog entry

## Acceptance Criteria

- [ ] No conflicting short flags (especially `-o`)
- [ ] Consistent `-c` for context across Search and Get commands
- [ ] All commands using output format leverage FormatArg
- [ ] All tests pass (unit + integration)
- [ ] Clippy clean with no warnings
- [ ] Manual testing confirms expected behavior
