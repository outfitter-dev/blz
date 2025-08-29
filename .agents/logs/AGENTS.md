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

- `202508291015-recap-daily.md`
- `202508291020-checkpoint-session.md`
- `202508291030-review-pr-58-followups.md`
- `202508291040-debug-indexer-deadlock-investigation.md`
- `202508291050-refactor-storage-paths.md`
- `202508291100-feature-mcp-server-stub.md`
- `202508291110-migration-config-layout-v2.md`

## Templates

Available templates in `.agents/logs/templates/`:

- [`CHECKPOINT.md`](templates/CHECKPOINT.md) - Session checkpoints with comprehensive state tracking
- [`RECAP.md`](templates/RECAP.md) - Daily development recaps
- [`REVIEW.md`](templates/REVIEW.md) - PR review notes and fixes
- [`DEBUG.md`](templates/DEBUG.md) - Debugging sessions and investigations
- [`REFACTOR.md`](templates/REFACTOR.md) - Major refactoring sessions
- [`FEATURE.md`](templates/FEATURE.md) - Feature implementation logs
- [`MIGRATION.md`](templates/MIGRATION.md) - Migrations or upgrades
- [`BRANCHWORK.md`](templates/BRANCHWORK.md) - Branch lifecycle tracking (PR creation through merge)

## Scripts & Automation

### Creating New Logs

Use `.agents/scripts/new-log.sh` to create logs from templates:

```bash
# Basic usage
./agents/scripts/new-log.sh --type checkpoint --desc "session-summary"

# With PR and Linear ticket
./agents/scripts/new-log.sh --type review --desc "pr-fixes" --pr 61 --linear BLZ-23

# With local timezone
./agents/scripts/new-log.sh --type debug --desc "indexer-deadlock" --local
```

**What it does:**

- Creates file: `YYYYMMDDHHmm-[type]-[slug].md` in `.agents/logs/`
- Auto-fills frontmatter: `date`, `branch`, `slug`, `pr`/`issue` (when present)
- Auto-detects PR number via `gh` or `gt` if not provided
- Strips example sections from templates

**Options:**

- `--type <type>` - One of: checkpoint, review, debug, feature, migration, recap, refactor
- `--desc "description"` - Short description for filename and slug
- `--pr <number>` - GitHub PR number (auto-detected if omitted)
- `--linear <id>` - Linear ticket ID (e.g., BLZ-23)
- `--local` - Use local timezone (default: UTC)

### Getting Timestamps

Use `.agents/scripts/get-date.sh` for consistent timestamps:

```bash
# Current UTC timestamp
./agents/scripts/get-date.sh  # → 202508291830

# Current local timestamp
./agents/scripts/get-date.sh --local  # → 202508291030

# From file metadata (creation → first commit → now)
./agents/scripts/get-date.sh --file README.md
```

- ### Branchwork (Per-Branch Worklog)

- Live file (not tracked): `.agents/logs/CURRENT.md` symlink → `branchwork/CURRENT-<branch>.md`
- Create: `.agents/scripts/branchwork.sh create` (sets date, branch, PR, stack pos)
- Update items/blocks/log entries:
  - `.agents/scripts/branchwork.sh update --section "Merge Checklist" --item "Squash commits"`
  - `.agents/scripts/branchwork.sh update --section "Decisions" --code ./notes.md --lang markdown`
  - `.agents/scripts/branchwork.sh update --log "Addressed review: fixed deadlock ordering"`
- Refresh PR stack context from Graphite (compact): `.agents/scripts/branchwork.sh refresh`
- Archive manually: `.agents/scripts/branchwork.sh archive`
- CI auto-archives on PR merge to `branchwork/YYYYMMDDHHmm-<branch>.md`
