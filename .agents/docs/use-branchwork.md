# Using Branchwork

Branchwork is a per-branch worklog that keeps context, status, and decisions in sync across agents.

## What it creates
- A tracked file per branch: `.agents/logs/branchwork/CURRENT-<branch>.md`
- A local symlink (not tracked): `.agents/logs/CURRENT.md` → the file above
- CI archives the file on merge to: `.agents/logs/branchwork/YYYYMMDDHHmm-<branch>.md`

## Quick start
- Create for current branch:
  - `just branchwork create` (or `.agents/scripts/branchwork.sh create`)
  - Optional: `--status draft|in-review|changes-requested|approved`
- Refresh PR stack context (compact tree):
  - `just branchwork refresh`
- Log a session update:
  - `just branchwork log "Short summary of work"`
- Add items to sections:
  - `just branchwork update --section "Merge Checklist" --item "Squash commits"`
  - `just branchwork update --section "Decisions" --code ./notes.md --lang markdown`

## Behavior
- Title:
  - No PR: `# [WIP] \`exact/branch-name\``
  - With PR: `# PR #<num>: <title>` (auto-updated on refresh)
- PR Stack Context: compact, deterministic tree from `gt log`
- Linting: runs `markdownlint-cli2 --fix` after each change if available

## Tips
- Run `just branchwork create` right after `gt create` when starting a new branch.
- Use `just branchwork log` whenever you complete a meaningful chunk to build the Updates timeline.
- On merge, the GitHub Action archives the file automatically.
