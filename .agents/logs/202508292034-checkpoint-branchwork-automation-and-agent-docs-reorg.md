---
date: 2025-08-29 20:34 UTC
branch: 08-29-chore_.agents_get-date.sh_defaults_to_local_add_--utc_flag
slug: checkpoint-branchwork-automation-and-agent-docs-reorg
pr: # e.g. #48 / BLZ-42
agent: # e.g. claude, codex, cursor, etc.
---

# [CHECKPOINT] Branchwork automation and agent docs reorg

## Summary

Implemented a robust agent documentation and worklog workflow:

- Reorganized `.agent` → `.agents` with updated references and docs
- Added log utilities (`get-date.sh`, `new-log.sh`) and branchwork automation
- Introduced compact, deterministic PR stack context via `gt log`
- Added CI to archive branchwork docs on PR merge
- Standardized formatting with markdownlint auto-fix after each change

## Tasks
<!-- Use CLOSES: #123 / BLZ-45 for completed issues -->
<!-- Use ADDRESSES: #123 / BLZ-45 for partial work -->
<!-- Use RELATED: #123 / BLZ-45 for related but not closing -->

### Done

- Reorganize `.agent` → `.agents`; update references across repo
- Migrate memory/docs and logs; standardize filenames (`YYYYMMDDHHmm-[type]-desc.md`)
- Add `.agents/logs/AGENTS.md` and `CLAUDE.md`; update `.agents/README.md`
- Add `.agents/scripts/get-date.sh` (UTC default; `--local` flag)
- Add `.agents/scripts/new-log.sh` (templated logs + example stripping)
- Add `.agents/scripts/branchwork.sh` (create/update/log/refresh/archive)
- Always-compact PR Stack Context; trim trailing spaces; add blank line after fence
- Ignore `.agents/logs/CURRENT.md`; commit per-branch file only; symlink locally
- Add CI workflow to archive branchwork on merge
- Add `.agents/docs/use-branchwork.md` guide
- Auto-run `markdownlint-cli2 --fix` after create/update/refresh

### In Progress

- [ ] Submit branch via Graphite and sync PR number/title into branchwork

### Not Started

- [ ] Optional PR body sync from branchwork (gh)
- [ ] Optional Linear linkage for Issues (MCP Linear)

## Changes
<!-- Include files changed with brief descriptions -->

- `.agents/scripts/get-date.sh`: timestamp helper (UTC default; `--local`)
- `.agents/scripts/new-log.sh`: templated log generation + lint fix
- `.agents/scripts/branchwork.sh`: branchwork create/update/log/refresh/archive
- `.agents/logs/templates/BRANCHWORK.md`: cleaned template; example moved
- `.agents/logs/templates/*`: checkpoint/debug/review/refactor/feature/migration
- `.agents/logs/AGENTS.md` and `.agents/README.md`: updated conventions + usage
- `.agents/docs/use-branchwork.md`: new usage guide
- `.github/workflows/branchwork-archive.yml`: archive CURRENT-<branch>.md on merge
- `.gitignore`: ignore `.agents/logs/CURRENT.md`

## Decisions & Rationale
<!-- Key technical decisions and why they were made -->

- Keep CURRENT.md as a local symlink only to avoid stack conflicts
- Always compact PR Stack Context for determinism and readability
- Strip template examples on generation to keep logs concise
- Auto-run markdownlint after every change for consistent formatting

## Current State

- **Branch Status**: Working on current branch; PR not yet submitted
- **CI Status**: CI workflow added for branchwork archive; not exercised yet
- **PR Stack**: Compact tree embedded in branchwork; refresh reflects latest `gt log`
- **Open Items**: Submit via Graphite; optional PR body sync; optional Linear linkage

## Blockers
<!-- What's preventing progress, if anything -->

- None at this time

## Next Steps

1. Submit current branch via Graphite; refresh branchwork to capture PR title/number
2. Optionally add `branchwork sync-pr` (gh) to push key sections into PR body
3. Optionally add Linear linkage (`--linear BLZ-XX`) and MCP sync

## Follow-ups
<!-- Non-critical items discovered during work to circle back to -->

- Improve stack subtree rendering (parents/current/children only)
- Add Windows fallback for CURRENT symlink; document behavior
- Bash tests for section insertion and refresh idempotency

## Context/Notes

---
