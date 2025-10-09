# .agents Directory

## Purpose

The `.agents` directory contains all agent-generated documentation, logs, and rules for the blz project. This directory provides a structured approach to maintaining agent context, development history, and operational guidelines.

## Directory Structure

```text
.agents/
├── docs/                 # Current, undated documentation
│   ├── PRD.initial.md
│   ├── branding.md
│   ├── performance-benchmarks.md
│   └── 20250825-blz-release-stack-plan.md
├── logs/                 # Timestamped agent logs and notes
│   ├── AGENTS.md        # Source of truth for formatting instructions
│   ├── CLAUDE.md        # Mirror of AGENTS.md for agent reference
│   ├── YYYYMMDDHHmm-[type]-*.md  # Active logs
│   └── .archive/        # Obsolete logs (>30 days or closed PRs)
│       └── .gitkeep
├── rules/               # Engineering and development rules
│   ├── IMPORTANT.md     # Priority rules and guidelines
│   ├── CORE.md          # Core engineering principles
│   ├── ARCHITECTURE.md  # System design patterns
│   ├── DEVELOPMENT.md   # Development practices
│   ├── STYLEGUIDE.md    # Writing tone and documentation standards
│   └── conventions/     # Language and tool-specific conventions
└── scripts/             # Helper scripts for maintenance
    └── sync-agents-md.sh
```text

## Subdirectory Purposes

### `/docs`

Contains current, undated documentation that represents the latest state of project documentation, requirements, and design decisions. These files are actively maintained and referenced.

### `/logs`

Timestamped logs of agent sessions, organized by type:

- **Active logs**: Recent development activity, PR reviews, debugging sessions
- **Archive**: Historical logs older than 30 days or for completed/closed work

### `/rules`

Engineering principles, architectural patterns, and development guidelines that agents should follow. These are actively used during development.

### `/scripts`

Utility scripts for maintaining agent documentation consistency.

## Log Type Prefixes

All files in `.agents/logs/` follow the format: `YYYYMMDDHHmm-[type]-description.md`

| Prefix | Purpose | Example |
|--------|---------|---------|
| `recap-` | Daily development recaps | `202508261735-recap-daily-development.md` |
| `checkpoint-` | Comprehensive session summaries | `202508261916-checkpoint-session-summary.md` |
| `review-` | PR review notes and fixes | `202508240848-review-pr-fixes.md` |
| `debug-` | Debugging sessions and investigations | `202508281000-debug-search-latency.md` |
| `refactor-` | Major refactoring sessions | `202508250000-refactor-rules-audit.md` |
| `feature-` | Feature implementation logs | `202508271400-feature-cache-warming.md` |
| `migration-` | Migration or upgrade logs | `202501231200-migration-rules-adaptation.md` |

## Git Tracking

### Tracked Files

- All files in `.agents/` except those in `.archive/`
- The `.gitkeep` file in `.archive/` is tracked

### Ignored Files

- Contents of `.agents/logs/.archive/*` (except `.gitkeep`)

## Common Workflows

### Creating a New Log

1. Generate timestamp: `date +%Y%m%d%H%M`
2. Choose appropriate type prefix
3. Create file: `.agents/logs/YYYYMMDDHHmm-[type]-description.md`
4. Follow templates in `AGENTS.md`

### Archiving Old Logs

Move logs to archive when:

- Older than 30 days and not referenced by current docs
- PR review notes for merged/closed PRs
- Debug logs for resolved issues
- Superseded migration or refactoring logs

```bash
git mv .agents/logs/old-file.md .agents/logs/.archive/
```text

### Finding Recent Activity

```bash
# List recent logs (last 7 days)
find .agents/logs -name "*.md" -mtime -7 | grep -v archive

# Find specific type
ls .agents/logs/*-review-*.md
```text

### Updating Rules

1. Edit source file in `.agents/rules/`
2. If updating AGENTS.md, run: `.agents/scripts/sync-agents-md.sh`
3. This ensures CLAUDE.md stays in sync

## Links to Detailed Documentation

- **Formatting Instructions**: See [logs/AGENTS.md](logs/AGENTS.md)
- **Engineering Principles**: See [rules/CORE.md](rules/CORE.md)
- **Development Workflow**: See [rules/WORKFLOW.md](rules/WORKFLOW.md)
- **Architecture Patterns**: See [rules/ARCHITECTURE.md](rules/ARCHITECTURE.md)

## Maintenance Notes

- Keep active logs clean: regularly archive old content
- Maintain clear, descriptive filenames
- Use consistent formatting per type templates
- Update this README when structure changes
