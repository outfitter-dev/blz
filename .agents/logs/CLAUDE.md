# Agent Log Formatting Instructions

This document is the **source of truth** for agent log formatting. The CLAUDE.md file is a mirror that should be kept in sync using the sync script.

## File Naming Convention

All log files must follow this format: `YYYYMMDDHHmm-[type]-description.md`

### Getting the Timestamp

```bash
date +%Y%m%d%H%M
```

Example output: `202508281030` (August 28, 2025, 10:30 AM)

### Type Prefixes

| Type | When to Use | Example Filename |
|------|-------------|------------------|
| `recap-` | Daily development summaries | `202508261735-recap-daily-development.md` |
| `checkpoint-` | Session summaries with project state | `202508261916-checkpoint-session-summary.md` |
| `review-` | PR review notes and fixes | `202508240848-review-pr-fixes.md` |
| `debug-` | Debugging sessions | `202508281000-debug-search-latency.md` |
| `refactor-` | Major refactoring work | `202508250000-refactor-rules-audit.md` |
| `feature-` | Feature implementation | `202508271400-feature-cache-warming.md` |
| `migration-` | Migration or upgrade work | `202501231200-migration-rules-adaptation.md` |

## Log Templates

### Recap Template (`recap-`)

```markdown
# Daily Development Recap - [Date]

## Summary
Brief overview of the day's work and accomplishments.

## Completed Tasks
- [ ] Task 1 with outcome
- [ ] Task 2 with outcome

## Code Changes
- **Files Modified**: List of key files
- **Lines Changed**: Approximate count
- **Key Improvements**: Bullet points

## Challenges & Solutions
- **Challenge**: Description
  **Solution**: How it was resolved

## Tomorrow's Priorities
1. Priority task 1
2. Priority task 2

## Notes
Any additional context or observations.
```

### Checkpoint Template (`checkpoint-`)

```markdown
# Session Checkpoint - [Date]

## Session Overview
**Duration**: [Start time - End time]
**Focus**: Main objective of this session
**Status**: [Completed | In Progress | Blocked]

## Files Changed
- `path/to/file1.rs` - Description of changes
- `path/to/file2.md` - Description of changes

## Decisions Made
1. **Decision**: Rationale behind it
2. **Decision**: Rationale behind it

## Current Project State
- **Branch**: Current working branch
- **Tests**: [Passing | Failing - details]
- **Build**: [Success | Failed - details]

## Open Pull Requests
- **PR #XX**: Title - Status
  - Requested changes: Brief summary
  - Next actions: What needs to be done

## PR Stack Maintenance
- **Stack Status**: Overview of stacked PRs
- **Rebase Needed**: Yes/No
- **Conflicts**: Any merge conflicts

## Next Session
- Immediate priorities
- Blocked items needing attention
```

### Review Template (`review-`)

```markdown
# PR Review - [PR Number/Title]

## Review Source
**Reviewer**: [@coderabbitai | @human-username]
**Date**: [Date of review]
**PR Link**: [URL]

## Requested Changes
1. [ ] Change description
   - **File**: `path/to/file`
   - **Line**: Line number(s)
   - **Fix Applied**: Description or "Pending"

2. [ ] Change description
   - **File**: `path/to/file`
   - **Line**: Line number(s)
   - **Fix Applied**: Description or "Pending"

## Improvements Made
- Summary of fixes applied
- Any additional improvements beyond requested

## Testing
- Tests run after changes
- Results

## Status
- [ ] All requested changes addressed
- [ ] Tests passing
- [ ] Ready for re-review
```

### Debug Template (`debug-`)

```markdown
# Debug Session - [Issue Description]

## Issue
**Symptoms**: What was observed
**Expected**: What should happen
**Impact**: Severity and affected areas

## Investigation
1. **Hypothesis**: Initial theory
   **Test**: How it was tested
   **Result**: What was found

2. **Hypothesis**: Next theory
   **Test**: How it was tested
   **Result**: What was found

## Root Cause
Detailed explanation of the actual problem.

## Solution
- **Fix Applied**: Code changes made
- **Files Modified**: List of files
- **Verification**: How fix was verified

## Prevention
Recommendations to prevent similar issues.
```

### Refactor Template (`refactor-`)

```markdown
# Refactoring Session - [Component/Module]

## Refactoring Goals
- [ ] Goal 1
- [ ] Goal 2

## Changes Made
### Structural Changes
- Description of architectural changes

### Code Improvements
- **Before**: Problem description
- **After**: Solution description

### Files Affected
- `path/to/file` - Type of changes

## Testing
- Tests updated: Yes/No
- New tests added: List
- Coverage: Before vs After

## Performance Impact
- Benchmarks run
- Results comparison

## Migration Notes
Any steps needed for existing code to work with refactored version.
```

### Feature Template (`feature-`)

```markdown
# Feature Implementation - [Feature Name]

## Feature Description
Brief description of what was implemented.

## Requirements Addressed
- [ ] Requirement 1
- [ ] Requirement 2

## Implementation Details
### Architecture
- Design decisions
- Component structure

### New Files
- `path/to/new/file` - Purpose

### Modified Files
- `path/to/modified/file` - Changes made

## Testing
### Unit Tests
- `test_file` - What it tests

### Integration Tests
- `test_file` - What it tests

## Documentation
- [ ] API docs updated
- [ ] README updated
- [ ] Examples added

## Next Steps
- Follow-up tasks
- Known limitations
```

### Migration Template (`migration-`)

```markdown
# Migration - [What Was Migrated]

## Migration Overview
**From**: Previous state/version
**To**: New state/version
**Reason**: Why migration was necessary

## Pre-Migration State
- Configuration/setup before

## Migration Steps
1. Step taken
2. Step taken
3. Step taken

## Post-Migration State
- New configuration/setup

## Verification
- [ ] Tests passing
- [ ] Functionality verified
- [ ] Performance checked

## Rollback Plan
Steps to revert if needed.

## Notes
Any gotchas or important observations.
```

## Archiving Guidelines

### When to Archive

Move logs to `.agents/logs/.archive/` when:

1. **Age**: Older than 30 days and not actively referenced
2. **PR Status**: Review notes for merged or closed PRs
3. **Issue Status**: Debug logs for resolved issues
4. **Superseded**: Migration or refactor logs that have been superseded
5. **Relevance**: No longer relevant to current development

### Archive Command

```bash
git mv .agents/logs/[filename] .agents/logs/.archive/
```

## Best Practices

1. **Timestamp Accuracy**: Always use the actual time when creating the log
2. **Descriptive Names**: Make the description part meaningful
3. **Template Usage**: Start with the appropriate template
4. **Regular Archiving**: Review and archive weekly
5. **Cross-References**: Link to related PRs, issues, or other logs
6. **Conciseness**: Be thorough but avoid unnecessary verbosity

## Sync Process

When this file is updated, run the sync script to update CLAUDE.md:

```bash
.agents/scripts/sync-agents-md.sh
```

This ensures both files remain synchronized.