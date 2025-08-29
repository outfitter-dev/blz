---
date: 2025-08-29 22:20 UTC
slug: branchwork-claude-issue-67-20250829-2220
status: draft
pr: # to be filled when PR is created
branch:
  name: claude/issue-67-20250829-2220
  base: main
  position: 1
  total: 1
reviewers: [galligan]
dri: claude
scope: agent-tooling, docs, development-experience
risk: low
backout_plan: Remove new files, revert symlinks
last_updated: 2025-08-29 22:20 UTC
---

# [WIP] `claude/issue-67-20250829-2220`

## Issues & Tickets

- [ ] #67 / BLZ-25: Improve Rust Development Reliability for AI Agents

## Definition of Done

- [ ] Create .agents/rules/ASYNC-PATTERNS.md with async/await guidance for agents
- [ ] Create .agents/rules/COMPILER-LOOP.md with JSON diagnostics workflow
- [ ] Create crate-specific AGENTS.md files for blz-core, blz-cli, blz-mcp
- [ ] Create CLAUDE.md symlinks pointing to AGENTS.md files
- [ ] Create scripts/agent-check.sh for compiler-in-the-loop workflow
- [ ] Update root AGENTS.md with agent quick start section
- [ ] All files follow project conventions and pass linting

## Merge Checklist

- [ ] All CI checks passing
- [ ] Code review feedback addressed
- [ ] Documentation follows project style guide
- [ ] Files are properly organized in directory structure
- [ ] Symlinks work correctly across platforms

## Decisions

- Use AGENTS.md as primary files with CLAUDE.md as symlinks (per user request)
- Focus on concrete pain points: async patterns, compiler errors, macro debugging
- Provide ready-to-use templates and anti-patterns
- Create directory-specific guidance for different crates

## Notes

- Building on the comprehensive analysis from issue #67
- Focusing on practical developer experience improvements
- Templates should be copy-pastable for agents

## Updates

### 2025-08-29 22:20 UTC: [@claude] Starting implementation

Setting up branch work documentation and beginning implementation of agent-friendly Rust development guides.