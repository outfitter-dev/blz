# PR Title Conventions

## Format

```text
<type>(<scope>): <subject>
```text

- **type**: feat, fix, docs, style, refactor, test, chore
- **scope**: Optional, the area affected (e.g., cli, core, mcp)
- **subject**: Imperative mood, lowercase, no period

## Rules

### Required

- Use imperative mood: "add" not "added" or "adds"
- Stay under 50 characters (72 absolute max)
- Match the primary commit if squash-merging
- Include ticket numbers: `fix(auth): resolve token expiry (#123)`

### Good PR Titles

```text
feat(cli): add progress indicator for indexing
fix(search): handle empty query strings gracefully
refactor(core): extract common parsing logic
docs: update API examples for v2.0
chore(deps): bump tantivy to 0.21
```text

### Bad PR Titles

```text
bug fix                     # Too vague
update code                 # What code? What update?
Fixed the thing that broke  # Not imperative, too casual
EMERGENCY FIX!!!           # Don't shout
misc changes               # Be specific
```text

## Stack-Specific (Graphite)

When working with stacked PRs:

- Each PR title describes only its changes
- Include stack position if helpful: `feat(api): [2/3] add validation`
- Keep titles independent - reviewers may see them out of order

## Why It Matters

Good PR titles:

- Populate changelog automatically
- Help reviewers prioritize
- Make git history searchable
- Document intent for future maintainers
