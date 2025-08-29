# Branch Workflow Guide

A comprehensive guide to managing branch-specific documentation and workflows in the blz project.

## Overview

The branch workflow system provides persistent, branch-specific documentation that follows a branch throughout its entire lifecycle - from creation through PR reviews to final merge. This ensures that all context, decisions, and review feedback are preserved and accessible across sessions and between different agents working on the same branch.

## Core Concepts

### 1. Branchwork Document

A **branchwork** document is a living worklog that tracks all activity on a specific branch. It includes:

- Branch context and PR stack position
- Issues/tickets being addressed
- Definition of done / success criteria
- CI status tracking
- Decisions and rationale
- Chronological updates (code changes, reviews, responses)
- Follow-up items

### 2. CURRENT.md Symlink

The `.agents/logs/CURRENT.md` file is a symbolic link that always points to the active branch's branchwork document. This provides:

- A consistent location for all agents to read/write
- Automatic context switching when changing branches
- No need to remember branch-specific file paths

### 3. Lifecycle Management

The branchwork document follows this lifecycle:

1. **Creation**: When starting a new branch
2. **Active Development**: Continuously updated during work
3. **PR Review**: Captures all review comments and responses
4. **Archival**: Automatically archived when PR is merged

## File Structure

```
.agents/logs/
├── CURRENT.md -> branchwork/CURRENT-feat-new-feature.md  # Symlink
├── branchwork/
│   ├── CURRENT-feat-new-feature.md                       # Active branch
│   ├── CURRENT-fix-bug-123.md                           # Another active branch
│   └── 202508291630-feat-old-feature.md                 # Archived (merged)
└── templates/
    └── BRANCHWORK.md                                     # Template
```

## Workflow Commands

### Creating a Branchwork Document

When starting a new branch:

```bash
# Using just (recommended)
just branchwork create

# Or directly
./.agents/scripts/branchwork.sh create

# With initial status
just branchwork create --status draft
```

This command:
- Creates `.agents/logs/branchwork/CURRENT-<branch-slug>.md`
- Sets up the symlink `.agents/logs/CURRENT.md`
- Auto-fills metadata (date, branch, PR number if exists)
- Generates initial title (WIP or PR title)

### Updating the Document

#### Adding Log Entries

Record work as you go:

```bash
# Log a work update (adds timestamped entry to Updates section)
just branchwork log "Fixed memory leak in indexer"
just branchwork log "Addressed PR review feedback from @coderabbitai"
```

#### Adding Structured Content

Add items to specific sections:

```bash
# Add to checklist
just branchwork update --section "Merge Checklist" --item "Update CHANGELOG.md"

# Add to decisions
just branchwork update --section "Decisions" --item "Use Arc<Mutex> for thread safety"

# Add code block from file
just branchwork update --section "Notes" --code ./analysis.txt --lang markdown

# Add from stdin
echo "Important context" | just branchwork update --section "Notes" --code - --lang text
```

### Refreshing PR Context

Update the PR stack visualization and title:

```bash
just branchwork refresh
```

This command:
- Updates the PR Stack Context from `gt log`
- Refreshes the PR title if available
- Updates the last_updated timestamp
- Keeps the tree view deterministic (strips volatile info)

### Manual Archival

If needed, manually archive the current branchwork:

```bash
just branchwork archive
```

This moves the file to `branchwork/YYYYMMDDHHmm-<branch>.md` and removes the symlink.

## Automatic Features

### PR Detection

The scripts automatically detect PR information using:
1. GitHub CLI (`gh pr view`)
2. Graphite (`gt log`)

### Markdown Linting

After each update, the document is automatically formatted using `markdownlint-cli2` if available.

### CI/CD Integration

The `.github/workflows/branchwork-archive.yml` workflow automatically archives branchwork documents when PRs are merged.

### Integration with new-log.sh

When creating other log types (checkpoint, review, etc.), if a branchwork CURRENT exists, the creation is automatically logged:

```bash
# This command...
./.agents/scripts/new-log.sh --type checkpoint --desc "session-summary"

# ...automatically adds to CURRENT.md:
### 2025-08-29 20:39 UTC: [@agent] Created CHECKPOINT log: [202508292039-checkpoint-session-summary.md]
```

## The Branchwork Script

### Core Script: branchwork.sh

```bash
#!/usr/bin/env bash
# branchwork.sh — Manage per-branch worklog

Commands:
  create   Create CURRENT symlink and branchwork doc from template
  update   Append items/blocks or log an update entry
  refresh  Refresh PR Stack Context from `gt log`
  archive  Move CURRENT to timestamped archive in branchwork/
  log      Shorthand for update --log

Options for create:
  --local                Use local time instead of UTC
  --status <status>      Set initial status (draft|in-review|changes-requested|approved)

Options for update:
  --section <Heading>    Required for --item/--subitem/--code
  --item "text"          Append list item under section
  --subitem "text"       Append nested list item under last list
  --code <path|->        Append fenced code block under section
  --lang <id>            Language id for code fence
  --log "summary"        Add timestamped entry to Updates section

Examples:
  branchwork create --status draft
  branchwork update --section "Decisions" --item "Use async/await"
  branchwork update --section "Notes" --code ./analysis.md --lang markdown
  branchwork log "Fixed CI failures"
  branchwork refresh
```

## Template Structure

The BRANCHWORK.md template includes these key sections:

### Front Matter
```yaml
date: 2025-08-29 18:55 UTC
slug: branchwork-<branch>
status: draft  # draft | in-review | changes-requested | approved | merged
pr: 47  # Auto-detected
branch:
  name: feat/feature-name
  base: main
  position: 2  # Position in stack
  total: 3     # Total in stack
```

### PR Stack Context
Visual representation using Unicode tree characters:
```text
◯ 08-28-feat_12_add_changelog.md_to_track_project_changes
◯ 08-28-fix_42_prevent_divide-by-zero_panic_in_search_pagination
│  ◯ 08-29-ci_set_up_graphite_ci_optimization_pipeline
│  ◉ 08-29-chore_.agents_get-date.sh_defaults_to_local_add_--utc_flag (current)
├──┘
◯ main
```

### Key Sections
- **Issues & Tickets**: Links to GitHub/Linear issues
- **Definition of Done**: Success criteria checklist
- **Merge Checklist**: Pre-merge requirements
- **CI Status**: Table of check results
- **Decisions**: Important technical decisions
- **Notes**: General observations
- **Updates**: Chronological log of all changes
- **Follow-up Work**: Items for future PRs

## Best Practices

### 1. Create Early
Run `branchwork create` immediately after creating a new branch with `gt create`.

### 2. Log Frequently
Use `branchwork log` to capture work as you complete it:
- After implementing a feature
- After fixing a bug
- After addressing review feedback
- When making important decisions

### 3. Preserve Review Comments
Copy PR review comments verbatim into the Updates section using code blocks:
```markdown
### 2025-08-29 21:15 - PR Review from @coderabbitai
````markdown
**Critical**: Potential deadlock in mutex ordering
- File: src/indexer.rs, Line 145
````
```

### 4. Track CI Status
Keep the CI Status table updated to track test results and build status.

### 5. Use for Context
When returning to a branch after time away, read CURRENT.md first to restore context.

## Integration with Development Tools

### Just Commands

The `justfile` provides convenient aliases:

```makefile
# Branchwork management
branchwork *ARGS:
    ./.agents/scripts/branchwork.sh {{ARGS}}
```

### Git Hooks

Consider adding a post-checkout hook to display CURRENT.md when switching branches.

### Editor Integration

Configure your editor to recognize `.agents/logs/CURRENT.md` as a project notes file.

## Troubleshooting

### CURRENT.md Not Found
If `CURRENT.md` doesn't exist, run:
```bash
just branchwork create
```

### Symlink Broken
If the symlink is broken, recreate it:
```bash
just branchwork create
```

### Updates Not Appearing
Ensure you're using the correct section name (case-sensitive):
```bash
just branchwork update --section "Updates" --log "Your message"
```

### Markdown Formatting Issues
The script automatically runs `markdownlint-cli2 --fix` if available. Install it:
```bash
npm install -g markdownlint-cli2
```

## Advanced Usage

### Custom Sections
Add new sections as needed by using them with update:
```bash
just branchwork update --section "Performance Metrics" --item "Search: 6ms P95"
```

### Bulk Updates
Pipe content from other commands:
```bash
git diff --stat | just branchwork update --section "Changes" --code - --lang diff
```

### Integration with Other Tools
The branchwork system integrates with:
- **Graphite (gt)**: For PR stack visualization
- **GitHub CLI (gh)**: For PR information
- **Linear**: Via ticket IDs in frontmatter
- **CI/CD**: Via GitHub Actions for auto-archival

## Summary

The branch workflow system provides a robust way to maintain context and documentation throughout a branch's lifecycle. By using branchwork documents, teams can:

- Preserve all decisions and context
- Track review feedback systematically
- Maintain continuity across sessions
- Enable seamless handoffs between agents
- Create an audit trail of development work

The combination of automated tooling and consistent structure ensures that no important information is lost, making development more efficient and collaborative.