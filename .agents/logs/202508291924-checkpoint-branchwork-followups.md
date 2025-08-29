# Branchwork Automation Follow-ups

- Improve `branchwork refresh` PR stack rendering to show only current stack subtree with parent/child indicators.
- Add `branchwork update --subsection <H1/H2 path>` support for precise insertion positions.
- Support `--pros`/`--cons` convenience flags to append to standardized sections.
- Add `--link-linear BLZ-XX` to populate Issues & Tickets and synchronize via MCP Linear.
- Add `gh pr edit --body-file` integration to sync key sections into PR body.
- Pre-push suggestion: optional hook to prompt `branchwork update --log "..."` with last commit title.
- Tests: bash unit tests for section insertion and refresh idempotency.
- Make `CURRENT.md` a symlink always; guard update/refresh to operate on symlink target only (done for write paths).
- Edge cases: handle headings that don’t exist; create them deterministically in correct order.
- Windows compatibility: avoid symlink on Windows, fallback to copying (doc this).
