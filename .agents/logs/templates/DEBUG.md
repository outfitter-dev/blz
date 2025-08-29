---
date: # e.g. 2025-08-29 20:00 UTC
issue: # GitHub and Linear (if applicable) e.g. #123 / BLZ-45
agent: # e.g. claude, codex, cursor, etc.
---

# Debug Session - [Area/Component]

## Issue Description

## Symptoms

- …

## Investigation

### Initial Observations

### Hypothesis

### Tests/Experiments

## Root Cause

## Resolution

## Verification

## Lessons Learned

---

## Example

```markdown
---
date: 2025-08-29 20:00 UTC
issue: #42
agent: claude
---

# Debug Session - Search Pagination

## Issue Description

Application panics when users pass `--limit 0` to search command.

## Symptoms

- Panic with "attempt to divide by zero" error
- Only occurs with `--limit 0` flag
- Affects all search operations
- First reported in production logs

## Investigation

### Initial Observations

- Stack trace points to `search.rs:234`
- Line contains: `let total_pages = total_results / limit`
- No guard clause for zero limit

### Hypothesis

Direct division by user-supplied limit without validation causes panic when limit is 0.

### Tests/Experiments

1. Reproduced locally with `blz search test --limit 0`
2. Added logging before division - confirmed limit=0
3. Tested with limit=1 - works fine
4. Checked other division operations - found 3 similar cases

## Root Cause

No validation of limit parameter before using in division operations. Users expected `--limit 0` to mean "unlimited" based on other CLI tools.

## Resolution

- Added early return when limit is 0
- Used saturating arithmetic for page calculations
- Added comprehensive tests for edge cases
- Updated CLI help to clarify limit behavior

## Verification

- Unit tests pass with limit=0
- Integration test covers CLI with --limit 0
- Property tests with random limits including 0
- Manual testing confirms no panic

## Lessons Learned

- Always validate user input before arithmetic operations
- Consider user expectations from similar tools
- Add property-based testing for user-facing parameters
```
