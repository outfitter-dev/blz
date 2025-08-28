# .agents Directory

## Overview

This directory contains agent-generated documentation, logs, and rules for the blz project. It serves as a centralized location for all agent-related files, providing both current documentation and historical context for development decisions.

## Directory Structure

```
.agents/
├── docs/                  # Current, undated documentation
│   ├── PRD.initial.md    # Product requirements document
│   ├── branding.md       # Branding guidelines
│   ├── performance-benchmarks.md
│   └── blz-release-stack-plan.md
├── logs/                  # Dated logs and session records
│   ├── YYYYMMDDHHMM-[type]-description.md
│   └── .archive/         # Obsolete logs (>30 days or superseded)
│       └── .gitkeep
├── rules/                 # Agent behavior rules and conventions
│   ├── ARCHITECTURE.md   # System architecture guidelines
│   ├── CORE.md          # Core engineering principles
│   ├── DEVELOPMENT.md   # Development practices
│   └── conventions/     # Language and tool-specific conventions
├── scripts/              # Utility scripts
│   └── sync-agents-md.sh # Sync AGENTS.md to CLAUDE.md
├── AGENTS.md            # Source of truth for agent instructions
└── CLAUDE.md            # Generated from AGENTS.md (do not edit directly)
```

## File Naming Conventions

### Log Files (`logs/` directory)

All files in `.agents/logs` must follow the format: `YYYYMMDDHHMM-[type]-description.md`

#### Type Prefixes

- **`recap-`**: Daily development recaps summarizing work completed
- **`checkpoint-`**: Comprehensive session summaries including:
  - Files changed during the session
  - Decisions made and rationale
  - Current project state
  - Open PRs with requested changes
  - PR stack maintenance notes
- **`review-`**: PR review notes, fixes, and feedback implementation
- **`debug-`**: Debugging sessions and issue investigations
- **`refactor-`**: Major refactoring sessions
- **`feature-`**: Feature implementation logs
- **`migration-`**: Migration or upgrade logs

#### Date Format

To generate the correct timestamp format, run:
```bash
date +%Y%m%d%H%M
```

Example: `202508281735-recap-daily-development.md`

### Documentation Files (`docs/` directory)

Files in `docs/` should not have date prefixes as they represent current documentation. Historical versions are tracked through git history.

## Archive Policy

Files are moved to `.agents/logs/.archive/` when:

- They are older than 30 days and not referenced by current documentation
- They contain superseded migration or refactoring logs
- They are PR review notes for merged or closed PRs
- They are debug logs for resolved issues

## Git Configuration

- The `.agents` directory is tracked in git
- `.agents/logs/.archive/*` files are ignored (except `.gitkeep`)
- All rules and current documentation are version controlled

## Common Workflows

### Creating a New Log

1. Determine the appropriate type prefix for your log
2. Generate timestamp: `date +%Y%m%d%H%M`
3. Create file: `.agents/logs/YYYYMMDDHHMM-[type]-description.md`
4. Follow the template for that log type

### Archiving Old Logs

1. Identify logs older than 30 days or that meet archive criteria
2. Move to `.agents/logs/.archive/` directory
3. Ensure `.gitkeep` remains in the archive directory

### Updating Agent Instructions

1. Edit `.agents/AGENTS.md` (source of truth)
2. Run `.agents/scripts/sync-agents-md.sh` to update CLAUDE.md
3. Never edit CLAUDE.md directly

## Templates

### Checkpoint Log Template

```markdown
# Checkpoint: [Session Description]

## Date
[YYYY-MM-DD HH:MM]

## Session Summary
[Brief overview of what was accomplished]

## Files Changed
- [ ] file1.rs - [description of changes]
- [ ] file2.toml - [description of changes]

## Decisions Made
1. [Decision]: [Rationale]
2. [Decision]: [Rationale]

## Current Project State
- [Component]: [Status]
- [Component]: [Status]

## Open PRs
- PR #[number]: [title] - [status/requested changes]
- PR #[number]: [title] - [status/requested changes]

## Next Steps
1. [Task]
2. [Task]
```

### Recap Log Template

```markdown
# Daily Recap: [Date]

## Summary
[High-level overview of the day's work]

## Completed Tasks
- [x] [Task description]
- [x] [Task description]

## In Progress
- [ ] [Task description]
- [ ] [Task description]

## Blockers
- [Blocker description and potential solutions]

## Key Decisions
- [Decision and reasoning]

## Metrics
- Lines of code: [added/removed]
- Tests: [added/passed/failed]
- Performance: [relevant metrics]
```

## Notes

- This structure is designed to maintain clarity while preserving historical context
- The separation of `docs/` and `logs/` helps distinguish between living documentation and point-in-time records
- The archive system prevents clutter while keeping important historical logs accessible
- All agent rules are maintained in the `rules/` directory for consistency