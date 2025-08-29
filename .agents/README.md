# .agents Directory

Purpose: Centralized, maintainable structure for agent-generated documentation and logs.

## Structure
```
.agents/
  README.md               # Overview and workflows
  rules/                  # Agent rules and standards
  docs/                   # Current undated documentation
  logs/
    AGENTS.md            # Logs formatting (source of truth)
    CLAUDE.md            # Mirror of AGENTS.md
    .archive/            # Obsolete logs (kept with .gitkeep)
```

## Logs Conventions
- Naming: `YYYYMMDDHHmm-[type]-description.md`
- Types: recap-, checkpoint-, review-, debug-, refactor-, feature-, migration-
- Timestamp: `date -u +%Y%m%d%H%M`
- Archive rules: see `./.agents/logs/AGENTS.md`

## Workflows
- Create a new log: `date -u +%Y%m%d%H%M` then choose type and description
- Daily recap: use `recap-`
- Session summary: use `checkpoint-` (replaces `handoff-`)
- Archive old/obsolete logs to `logs/.archive/` (keep `.gitkeep`)

## Git Tracking
- `.agents` is tracked
- Archived files ignored via `.gitignore`, but `.archive/.gitkeep` kept

## References
- Rules: `./.agents/rules/`
- Detailed logs guide: `./.agents/logs/AGENTS.md`
```
