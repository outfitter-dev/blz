# Agent Logs Guidelines (Source of Truth)

- File naming: `YYYYMMDDHHmm-[type]-description.md`
- Types:
  - `recap-`: daily development recaps
  - `checkpoint-`: session checkpoints (comprehensive, replaces `handoff-`)
  - `review-`: PR review notes and fixes
  - `debug-`: debugging sessions and investigations
  - `refactor-`: major refactoring sessions
  - `feature-`: feature implementation logs
  - `migration-`: migrations or upgrades
- Timestamp: use `./.agents/scripts/get-date.sh` (UTC default; `--local` supported)
- Archive: Move to `.agents/logs/.archive/` when:
  - Older than 30 days and not referenced
  - Superseded migrations/refactors
  - PR review notes for merged/closed PRs
  - Debug logs for resolved issues

## Examples

### Checkpoint Session
```bash
.agents/scripts/new-log.sh --type checkpoint --desc "coderabbit fixes implementation"
# Creates: 202508291345-checkpoint-coderabbit-fixes-implementation.md
```

### Daily Recap
```bash
.agents/scripts/new-log.sh --type recap --desc "daily development"
# Creates: 202508291800-recap-daily-development.md
```

## Templates

### Checkpoint Template
```markdown
# Checkpoint: [Brief Description]

## Session Summary
[2-3 sentences about what was accomplished]

## Files Changed
- `path/file.rs` - [Brief description]

## Key Decisions
1. [Decision and rationale]

## Current State
- [Status of main features/work]
- [Any blocking issues]

## Next Steps
1. [Priority task]

## Notes for Next Agent
[Important context or warnings]
```

### Recap Template
```markdown
# Daily Recap: [Date]

## Summary
[Overall progress]

## Completed
- [ ] Task 1
- [ ] Task 2

## In Progress
- [ ] Task with status

## Blockers
- [Any blocking issues]

## Tomorrow's Priorities
1. [Priority 1]
```

