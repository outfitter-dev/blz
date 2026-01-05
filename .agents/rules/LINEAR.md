<!-- tldr ::: linear project management rules and configuration -->

# LINEAR.md

## Project

- Team: BLZ
- Key: `BLZ`
- Workspace: Outfitter

## Streamlinear MCP

This project uses the `streamlinear` MCP server for Linear integration. All actions use a single tool: `mcp__linear__linear`.

### Default Team Filter

Always filter by team `BLZ` when searching:

```json
{ "action": "search", "query": { "team": "BLZ" } }
```

### Common Actions

| Action | Example |
|--------|---------|
| Search team issues | `{ "action": "search", "query": { "team": "BLZ" } }` |
| Search in progress | `{ "action": "search", "query": { "team": "BLZ", "state": "In Progress" } }` |
| Get issue | `{ "action": "get", "id": "BLZ-123" }` |
| Update status | `{ "action": "update", "id": "BLZ-123", "state": "Done" }` |
| Add comment | `{ "action": "comment", "id": "BLZ-123", "body": "Comment text" }` |
| Create issue | `{ "action": "create", "title": "Issue title", "team": "BLZ" }` |
| GraphQL query | `{ "action": "graphql", "graphql": "query { ... }" }` |

## Critical Rules

- Linear is the authoritative tracker. Log or locate an issue before meaningful work.
- Always use team filter `BLZ` when searching to scope results to this project.
- Keep issues current: status, description, and comments should reflect reality.
- Use Linear IDs consistently across branches, commits, and pull requests.

## Workspace Defaults

- GitHub issues may sync to different numbers; the Linear ID remains canonical.
- Issues should enter `Backlog` or `Todo` (not `Triage`) unless triaging is the task.
- Assign a priority when possible. `Critical` is P0 and should be rare.

## Workflow Notes

- Status flow: `Backlog` -> `Todo` -> `In Progress` -> `In Review` -> `Ready to Merge` -> `Done`.
- Attach artifacts (design docs, screenshots) directly to the Linear issue and reference them in commits/PRs when necessary.
- Keep relationships clear: add dependent/related issues in the body and via Linear's relationship controls.

## Formatting Issues in Linear

- Link the first reference to another issue inline: `[BLZ-123](https://linear.app/outfitter/issue/BLZ-123)`.
- Subsequent mentions can use `BLZ-123` with no link.
- Capture definition of done / acceptance criteria directly in the issue.
- Record dependencies by mentioning related IDs or using Linear's relationship UI.

## Referencing Issues in GitHub

### Branches

- Include the issue ID and slug in the branch name: `blz-123-issue-slug`.
- When possible, use Linear's **Copy git branch name** helper (Cmd/Ctrl + Shift + .) to pull the canonical slug directly.

### Pull Requests

- PR titles follow the conventional-commit style with the ID suffix: `feat: improve pagination [BLZ-129]`.
- In PR descriptions, always include a magic word plus ID (`Fixes: BLZ-129`) if applicable.

### Commits

- Conventional commit summary, followed by a footer with Linear magic words:
  - `Fixes: BLZ-123` (closing magic word: moves issue to In Progress/Done when merged).
  - `Refs: BLZ-123`, `Related: BLZ-123`, `Part of: BLZ-123` (non-closing; keeps issue open).
- Closing magic words: `close`, `closes`, `closed`, `closing`, `fix`, `fixes`, `fixed`, `fixing`, `resolve`, `resolves`, `resolved`, `resolving`, `complete`, `completes`, `completed`, `completing`.
- Non-closing magic words: `ref`, `refs`, `references`, `part of`, `related to`, `contributes to`, `toward`, `towards`.
- Syntax example:

```
feat: add context auto expansion

Implements section-aware context expansion with depth limits.

Fixes: BLZ-135
Refs: BLZ-131
```

## Pull Request & Issue Comments

- Linear recognizes closing/non-closing magic words in PR descriptions, commits, and comments just like it does in commit footers.
- Prefer one issue per PR; if a PR spans multiple issues, list each ID explicitly (e.g., `Fixes: BLZ-101, BLZ-202`).
