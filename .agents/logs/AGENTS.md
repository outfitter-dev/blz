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
- Timestamp: `date -u +%Y%m%d%H%M`
- Archive: Move to `.agents/logs/.archive/` when:
  - Older than 30 days and not referenced
  - Superseded migrations/refactors
  - PR review notes for merged/closed PRs
  - Debug logs for resolved issues

## Examples
- `202508291015-recap-daily.md`
- `202508291020-checkpoint-session.md`
- `202508291030-review-pr-58-followups.md`
- `202508291040-debug-indexer-deadlock-investigation.md`
- `202508291050-refactor-storage-paths.md`
- `202508291100-feature-mcp-server-stub.md`
- `202508291110-migration-config-layout-v2.md`

## Templates

### Recap
```
# Daily Recap — YYYY-MM-DD
- Summary:
- Highlights:
- Blockers:
- Next:
```

### Checkpoint
```
# Session Checkpoint — YYYY-MM-DD HH:MM UTC
- Files changed:
- Decisions & rationale:
- Current project state:
- Open PRs + requested changes:
- PR stack maintenance:
- Next steps:
```

### Review
```
# PR Review — <org/repo>#<PR>
- Summary:
- Findings:
- Fixes applied:
- Follow-ups:
```

### Debug
```
# Debug Session — <area>
- Issue:
- Observations:
- Hypotheses:
- Experiments:
- Resolution:
```

### Refactor
```
# Refactor — <area>
- Scope:
- Motivation:
- Changes:
- Risks:
- Rollout:
```

### Feature
```
# Feature — <name>
- Goal:
- Design:
- Tasks:
- Validation:
```

### Migration
```
# Migration — <name>
- Context:
- Plan:
- Steps:
- Backout:
```
