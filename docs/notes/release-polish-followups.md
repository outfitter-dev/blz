# Release Tweaks Follow-ups

_Tracking doc for the CLI polish changes landed on `gt-v0.3/mega`._

## Context
- Default brief text layout now emits rank/score headers, path summaries, and arrow-style footer.
- `--format` flag replaced `--output`; `--show` now only controls `url`/`lines`.
- `blz instruct` trimmed to curated guidance + docs pointer.

## Follow-up Items

1. **Formatter cleanup**
   - ✅ Hashed heading line available via `--show anchor`; snippet length now configurable (`--snippet-lines` / `BLZ_SNIPPET_LINES`).

2. **Score presentation**
   - ✅ Users can set score precision with `--score-precision` / `BLZ_SCORE_PRECISION` (JSON remains raw).

3. **Path truncation**
   - ✅ Width-aware helper landed in `utils::formatting::format_heading_path`.

4. **History & pagination**
 - ✅ CLI persists `--show`, score precision, snippet length defaults and ships a `blz history` command (text/json/jsonl).
  - ✅ Added a parent-process watchdog so orphaned `blz search …` processes terminate automatically (GitHub issue #188).

5. **Docs & examples**
   - Regenerate screenshots / asciicasts to reflect the new output.

6. **Release automation**
   - ✅ `release.yml` builds and uploads artifacts for macOS (arm64/x64), Linux (x64/arm64), and Windows x64, and references `CARGO_REGISTRY_TOKEN` for crates.io publishing.

_Add new items as we receive feedback, and reference this doc from inline TODOs when deferring work._
