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
- Timestamp: use `./.agents/scripts/get-date.sh` (UTC by default; `--local` supported)
- Archive rules: see `./.agents/logs/AGENTS.md`

## Workflows
- Create a new log (recommended):
  - `.agents/scripts/new-log.sh --type checkpoint --desc "agent docs reorg" --pr 61 --linear BLZ-23`
  - Types: `checkpoint`, `review`, `debug`, `feature`, `migration`, `recap`, `refactor`
  - Strips example content automatically; fills date, branch, slug, PR/issue.
- Get a timestamp for manual tasks:
  - UTC: `.agents/scripts/get-date.sh`
  - Local: `.agents/scripts/get-date.sh --local`
- Daily recap: use `recap-`
- Session summary: use `checkpoint-` (replaces `handoff-`)
- Archive old/obsolete logs to `logs/.archive/` (keep `.gitkeep`)

## Branchwork (Per-Branch Worklog)
- Create or update the branch worklog that all agents use on a feature branch.
- Live file (not tracked): `.agents/logs/CURRENT.md` symlink → `.agents/logs/branchwork/CURRENT-<branch>.md`
- On merge, CI archives to `.agents/logs/branchwork/YYYYMMDDHHmm-<branch>.md` automatically.

Commands:
- Create: `just branchwork create` (or `.agents/scripts/branchwork.sh create`)
- Update items: `just branchwork update --section "Merge Checklist" --item "Squash commits"`
- Add code block: `just branchwork update --section "Decisions" --code ./notes.md --lang markdown`
- Log an update: `just branchwork log "Addressed review: fixed deadlock ordering"`
- Refresh PR stack: `just branchwork refresh`
- Archive (manual): `just branchwork archive`

## Git Tracking
- `.agents` is tracked
- Archived files ignored via `.gitignore`, but `.archive/.gitkeep` kept

## References
- Rules: `./.agents/rules/`
- Detailed logs guide: `./.agents/logs/AGENTS.md`