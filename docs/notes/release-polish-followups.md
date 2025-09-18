# Release Tweaks Follow-ups

_Tracking doc for the CLI polish changes landed on `gt-v0.2/feat/release-polish`._

## Context
- Default brief text layout now emits rank/score headers, path summaries, and arrow-style footer.
- `--format` flag replaced `--output`; `--show` now only controls `url`/`lines`.
- `blz instruct` trimmed to curated guidance + docs pointer.

## Follow-up Items

1. **Formatter cleanup**
   - Consider re-introducing hashed heading line once compact path display is revisited.
   - Evaluate configurable snippet length vs full-span printing (depending on user feedback).

2. **Score presentation**
   - Allow users to configure score precision (`--score-precision`?) or switch to raw Tantivy score in JSON only.

3. **Path truncation**
   - Replace current `first > ... > penultimate > last` heuristic with a reusable helper aware of CLI width.

4. **History & pagination**
   - Persist history toggles (e.g., remember `--show url`) and expose `blz history` command.

5. **Docs & examples**
   - Regenerate screenshots / asciicasts to reflect the new output.

_Add new items as we receive feedback, and reference this doc from inline TODOs when deferring work._
