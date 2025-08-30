---
date: 2025-08-30 14:09 UTC
slug: branchwork-update-agent-rules
status: in-review
pr: 83
branch:
  name: chore/update-agent-rules
  base: main
  position: 1
  total: 1
reviewers: [coderabbitai]
dri: claude
scope: agents, documentation
risk: low
backout_plan: revert commit if any issues
last_updated: 2025-08-30 14:09 UTC
---

# PR #83: Update Agent Rules Files

## PR Stack Context

```text
main
└─● chore/update-agent-rules 👈 Current
```

## Issues

- Updates agent rules documentation based on project evolution

## Definition of Done

- [x] Agent rules files updated to latest standards
- [x] All markdown files properly formatted
- [ ] PR created and reviewed

## Merge Checklist

- [ ] All CI checks passing
- [ ] Code review feedback addressed
- [ ] Documentation reviewed
- [ ] Ready to merge

## CI Status

| Check | Status | Details |
|-------|--------|---------|
| Build | - | Pending |
| Tests | - | Pending |
| Lint | - | Pending |

## Decisions

- Keep agent rules aligned with current project structure
- Maintain consistency with other Outfitter submodules

## Notes

- Simple documentation update to keep agent rules current
- Low risk change affecting only development guidance

## Updates

### 2025-08-30 14:09: [@claude] Initial update

Updated agent rules files to reflect current project state

- Files changed (18 files, +706/-88):
  - `.agents/rules/GRAPHITE.md` - Added comprehensive Graphite workflow guide
  - `.agents/rules/WORKFLOW.md` - Updated agent workflow patterns
  - `.agents/rules/AGENT-RULES.md` - Added agent-specific rules
  - `.agents/rules/IMPORTANT.md` - Updated priority order
  - `.agents/rules/DEVELOPMENT.md` - Refined development practices
  - `.github/workflows/` - Updated CI/CD configurations
  - `CLAUDE.md` - Simplified important section
  - Various other agent documentation updates