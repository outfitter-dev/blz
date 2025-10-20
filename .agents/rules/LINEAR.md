<!-- tldr ::: linear project management rules and configuration -->

# LINEAR.md

## Critical Rules

- Linear is the authoritative tracker. Log or locate an issue before meaningful work.
- Keep issues current: status, description, and comments should reflect reality.
- Use Linear IDs consistently across branches, commits, and pull requests.

## Workspace Defaults

- When this doc refers to `ID-###`, substitute your team’s prefix (e.g., `BLZ-123`).
- GitHub issues may sync to different numbers; the Linear ID remains canonical.
- Issues should enter `Backlog` or `Todo` (not `Triage`) unless triaging is the task.
- Assign a priority when possible. `Critical` is P0 and should be rare.

## Workflow Notes

- Status flow: `Backlog` → `Todo` → `In Progress` → `In Review` → `Ready to Merge` → `Done` (adjust per team norms).
- Attach artifacts (design docs, screenshots) directly to the Linear issue and reference them in commits/PRs when necessary.
- Keep relationships clear: add dependent/related issues in the body and via Linear’s relationship controls.

## Formatting Issues in Linear

- Link the first reference to another issue inline: `[ID-123](https://linear.app/<workspace>/issue/ID-<num>)`.
- Subsequent mentions can use `ID-123` with no link.
- Capture definition of done / acceptance criteria directly in the issue.
- Record dependencies by mentioning related IDs or using Linear’s relationship UI.

## Referencing Issues in GitHub

### Branches

- Include the issue ID and slug in the branch name: `id-123-issue-slug`.
- When possible, use Linear’s **Copy git branch name** helper (Cmd/Ctrl + Shift + .) to pull the canonical slug directly.

### Pull Requests

- PR titles follow the conventional-commit style with the ID suffix: `feat: improve pagination [ID-129]`.
- In PR descriptions, always include a magic word plus ID (`Fixes: ID-129`) if applicable.

### Commits

- Conventional commit summary, followed by a footer with Linear magic words:
  - `Fixes: ID-123` (closing magic word: moves issue to In Progress/Done when merged).
  - `Refs: ID-123`, `Related: ID-123`, `Part of: ID-123` (non-closing; keeps issue open).
- Closing magic words: `close`, `closes`, `closed`, `closing`, `fix`, `fixes`, `fixed`, `fixing`, `resolve`, `resolves`, `resolved`, `resolving`, `complete`, `completes`, `completed`, `completing`.
- Non-closing magic words: `ref`, `refs`, `references`, `part of`, `related to`, `contributes to`, `toward`, `towards`.
- Syntax example:

```
feat: add context auto expansion

Implements section-aware context expansion with depth limits.

Fixes: ID-135
Refs: ID-131
```

## Pull Request & Issue comments

- Linear recognizes closing/non-closing magic words in PR descriptions, commits, and comments just like it does in commit footers.
- Prefer one issue per PR; if a PR spans multiple issues, list each ID explicitly (e.g., `Fixes: ID-101, ID-202`).
