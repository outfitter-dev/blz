# Worklogs

Keep track of your work and progress using timestamped log files in `.agents/logs/`.

Use the commands and templates below to create structured logs.

## Commands

### Creating New Logs

Use `.agents/scripts/new-log.sh` to create logs from templates:

```bash
# Basic usage - creates checkpoint log
./.agents/scripts/new-log.sh --type checkpoint --desc "session-summary"

# Create review log with PR and Linear ticket
./.agents/scripts/new-log.sh --type review --desc "pr-fixes" --pr 61 --linear BLZ-23

# Debug log with local timezone
./.agents/scripts/new-log.sh --type debug --desc "indexer-deadlock" --local

# Feature implementation log
./.agents/scripts/new-log.sh --type feature --desc "parallel-indexing"
```

**What it does:**

- Creates file: `YYYYMMDDHHmm-[type]-[slug].md` in `.agents/logs/`
- Auto-fills frontmatter: `date`, `branch`, `slug`, `pr`/`issue` (when present)
- Auto-detects PR number via `gh` or `gt` if not provided
- Strips example sections from templates
- Runs markdownlint to ensure proper formatting

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
./.agents/scripts/get-date.sh  # → 202508291830

# Current local timestamp
./.agents/scripts/get-date.sh --local  # → 202508291030

# From file metadata (creation → first commit → now)
./.agents/scripts/get-date.sh --file README.md
```

## Templates

- [`CHECKPOINT.md`](./.agents/logs/templates/CHECKPOINT.md): Session checkpoints with comprehensive state tracking
- [`RECAP.md`](./.agents/logs/templates/RECAP.md): Daily development recaps
- [`REVIEW.md`](./.agents/logs/templates/REVIEW.md): PR review notes and fixes
- [`DEBUG.md`](./.agents/logs/templates/DEBUG.md): Debugging sessions and investigations
- [`REFACTOR.md`](./.agents/logs/templates/REFACTOR.md): Major refactoring sessions
- [`FEATURE.md`](./.agents/logs/templates/FEATURE.md): Feature implementation logs
- [`MIGRATION.md`](./.agents/logs/templates/MIGRATION.md): Migrations or upgrades
