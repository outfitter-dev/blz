---
date: # e.g. 2025-08-29 19:30 UTC
pr: # GitHub and Linear (if applicable) e.g. #123 / BLZ-45
reviewer: # e.g. coderabbitai, human username
agent: # e.g. claude, codex, cursor, etc.
---

# PR Review - #[PR]

## Summary

## Review Comments

### Critical
<!-- Must fix before merge -->

### Suggestions
<!-- Nice to have improvements -->

### Questions
<!-- Clarifications needed -->

## Fixes Applied

- …

## Not Fixed (with rationale)

- …

## Follow-ups
<!-- To address in future PRs -->

---

## Example

```markdown
---
date: 2025-08-29 19:30 UTC
pr: #48
reviewer: coderabbitai
agent: claude
---

# PR Review - #48

## Summary

CodeRabbit identified 5 issues in the pagination fix PR: 1 critical bug, 3 suggestions, and 1 question about approach.

## Review Comments

### Critical

- **Line 360**: Snippet context length can exceed max_len parameter
  - Fixed by deriving context from max_len, splitting budget evenly

### Suggestions

- **Line 145**: Consider using `NonZeroU32` for limit parameter
  - More type-safe, prevents divide-by-zero at compile time
- **Line 223**: URL parsing could use `Url::parse()` instead of string manipulation
  - Safer for edge cases with query parameters
- **Benchmark file**: Registry creation discards entries, returns empty
  - Fixed by adding proper population logic

### Questions

- Why use saturating arithmetic instead of checked arithmetic?
  - Answered: Performance in hot path, aligns with existing patterns

## Fixes Applied

- [x] Fixed snippet context length calculation
- [x] Fixed benchmark registry population
- [x] Improved URL parsing safety
- [x] Added rationale comment for arithmetic choice

## Not Fixed (with rationale)

- NonZeroU32 suggestion: Would be breaking API change, defer to v0.2

## Follow-ups

- Consider NonZeroU32 for v0.2 API redesign
- Add more comprehensive URL parsing tests
- Document arithmetic overflow behavior
```
