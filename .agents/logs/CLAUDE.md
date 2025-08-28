# Agent Log Format Guidelines

## File Naming Convention

All log files must follow the format: `YYYYMMDDHHMM-[type]-description.md`

### Getting the Timestamp
```bash
date +%Y%m%d%H%M
```

### Type Prefixes

| Prefix | Usage | Example |
|--------|-------|---------|
| `recap-` | Daily development summaries | `202508281500-recap-daily-development.md` |
| `checkpoint-` | Session summaries, handoffs, PR stack status | `202508281600-checkpoint-pr-stack-update.md` |
| `review-` | PR review notes and fixes | `202508281700-review-coderabbit-fixes.md` |
| `debug-` | Debugging sessions | `202508281800-debug-search-performance-issue.md` |
| `refactor-` | Major refactoring work | `202508281900-refactor-index-module.md` |
| `feature-` | Feature implementation | `202508282000-feature-etag-support.md` |
| `migration-` | Migrations and upgrades | `202508282100-migration-tantivy-upgrade.md` |

## Log Templates

### Checkpoint Template
```markdown
# Session Checkpoint: [Brief Description]

## Session Summary
[2-3 sentences about what was accomplished]

## Files Changed
- `path/to/file1.rs` - [Brief description of changes]
- `path/to/file2.rs` - [Brief description of changes]

## Key Decisions
1. [Decision 1 and rationale]
2. [Decision 2 and rationale]

## Current Project State
- [Current status of main features/work]
- [Any blocking issues]

## PR Stack Status
- PR #XX: [Status and any review comments]
- PR #YY: [Status and any review comments]

## Next Steps
1. [Priority task 1]
2. [Priority task 2]

## Notes for Next Agent
[Any important context or warnings]
```

### Recap Template
```markdown
# Daily Recap: [Date]

## Summary
[Overall progress summary]

## Completed Tasks
- [ ] Task 1
- [ ] Task 2

## In Progress
- [ ] Task with status

## Blockers
- [Any blocking issues]

## Tomorrow's Priorities
1. [Priority 1]
2. [Priority 2]
```

### Review Template
```markdown
# PR Review: [PR Number and Title]

## Review Source
[CodeRabbit/Human reviewer/etc.]

## Issues to Address
1. [ ] Issue 1 description
2. [ ] Issue 2 description

## Changes Made
- `file.rs`: [Change description]

## Testing
- [ ] Tests added/updated
- [ ] All tests passing

## Status
[Ready for re-review/Merged/etc.]
```

### Debug Template
```markdown
# Debug Session: [Issue Description]

## Problem
[Clear description of the issue]

## Investigation
1. [Step 1 and findings]
2. [Step 2 and findings]

## Root Cause
[Identified root cause]

## Solution
[Applied solution]

## Testing
[How the fix was verified]

## Prevention
[Steps to prevent recurrence]
```

## Archival Guidelines

Move logs to `.agents/logs/.archive/` when:

1. **Age-based**: Logs older than 30 days that aren't referenced by current docs
2. **Status-based**: 
   - PR review notes for merged/closed PRs
   - Debug logs for resolved issues
   - Superseded migration logs
3. **Relevance-based**: Logs no longer relevant to current development

## Best Practices

1. **Be Concise**: Focus on actionable information
2. **Use Timestamps**: Always use the standardized format
3. **Link Issues/PRs**: Reference GitHub issues and PRs where relevant
4. **Document Decisions**: Explain why, not just what
5. **Update Regularly**: Create checkpoints at session end
6. **Archive Promptly**: Keep logs directory clean and relevant

## Synchronization

This file should be kept in sync with `CLAUDE.md` in the same directory. Use `.agents/scripts/sync-agents-md.sh` to maintain consistency.