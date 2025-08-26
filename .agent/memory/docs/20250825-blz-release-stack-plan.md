# blz v0.1 Public Release Stack Plan (Graphite)

This document defines a detailed, end-to-end implementation plan for the first public release of blz. It lays out a stacked-PR sequence (Graphite) with clear scope, success criteria, test plans, and risk management.

## Objectives

- Ship a reliable, fast MVP suitable for public use.
- Ensure search quality (correct snippets, line ranges, and relevance) and a truthful CLI surface (no stubbed commands advertised).
- Align on a consistent storage/config path that matches documentation and user expectations.
- Maintain performance discipline (no major regressions; basic optimizations in place).

## Release Scope (v0.1)

P0 (must):
- Parser correctness for heading-block extraction (exact, non-duplicated slices with accurate line ranges).
- Implement `update` MVP (ETag/Last-Modified; archive + reindex when modified).
- Unify storage/config location to match docs (prefer `~/.outfitter/blz`).
- Reduce search over-fetching and add basic parallelism across sources.
- Tighten lints (remove broad allow list) and hide unfinished commands (e.g., `diff`) from help.

P1 (should):
- Map common HTTP errors to better error kinds (404 → NotFound) for nicer CLI UX.
- Documentation updates (CLI, storage paths, known limitations, MCP output shape stability).

P2 (later follow-ups):
- Feature-gate “optimized” subsystems; incremental indexing; field boosts; richer agent JSON modes.

## Success Criteria

- Parser: unit/integration tests demonstrate that heading blocks contain exactly the content between headings, no duplication; line ranges match real lines.
- CLI: `add`, `search`, `get`, `list`, `update` work as documented; `diff` hidden or marked experimental; docs reflect actual behavior.
- Config/Storage: path on disk matches documentation; legacy path (if any) is migrated or a clear message is printed.
- Search: no over-fetch (large default caps removed); parallel search reduces latency versus sequential.
- Lints: no broad crate-level `#![allow(...)]` in core; CI linting passes with warnings treated seriously.

## Stack Overview (Graphite)

Parent branch: `main`

Proposed stack (each PR is small, focused, and reviewable):

1. `fix/parser/heading-blocks-slicing` — P0: Correct heading-block extraction
2. `fix/storage/paths-unify` — P0: Unify storage/config path; docs + migration
3. `feat/cli/update-mvp` — P0: Implement update with ETag/Last-Modified; archive+reindex
4. `perf/search/limit-and-concurrency` — P0: Reduce over-fetching; parallelize across sources
5. `chore/lints-and-cli-polish` — P0: Tighten lints; hide `diff`; friendlier errors
6. `docs/release-notes-and-known-limitations` — P1: Docs polish + MCP shape notes

Upstack (post-release or optional):

7. `feat/core/feature-flag-optimized` — Gate optimized subsystems behind a feature
8. `perf/index/boost-heading-path` — Field boosts for `heading_path` in Tantivy
9. `tests/e2e/cli-integration` — Integration tests for `add→search→get→list→update`

## Detailed PR Breakdown

### 1) fix/parser/heading-blocks-slicing

Title: P0: Correct heading-block extraction; exact line slices

Primary files:
- `crates/blz-core/src/parser.rs` — Replace node-by-node concatenation with explicit slicing by byte ranges between headings. Compute start/end lines correctly.
- Tests in `crates/blz-core/src/parser.rs` (existing module): add sentinel-based tests verifying ranges and that no duplicated content appears; keep property tests intact.

Key changes:
- Collect headings as tuples: `(level, byte_start, line_start)` while walking the tree.
- Build blocks by slicing: `text[prev.byte_start .. curr.byte_start]` and computing `end_line` from `line_count` or precomputed line offsets. Final block ends at EOF.
- Maintain `toc.lines` and `heading_blocks.{start_line,end_line}` as 1-based, inclusive.

Acceptance criteria:
- All parser tests pass; new tests confirm no duplication and correct range accounting under mixed headings and Unicode.
- No unsafe code introduced; memory usage and parse times remain within targets (<150 ms/MB typical).

Risks & mitigations:
- Off-by-one and byte/char boundary issues → extensive tests with Unicode and grapheme clusters.

### 2) fix/storage/paths-unify

Title: P0: Unify storage/config paths to `~/.outfitter/blz` + migration

Primary files:
- `crates/blz-core/src/storage.rs` — Switch to `ProjectDirs::from("dev", "outfitter", "blz")` (data dir) and ensure directories reflect documentation.
- `crates/blz-core/src/config.rs` — Same alignment for config path (config dir under the same app name), and update defaults.
- Docs: `docs/cli.md`, `docs/architecture.md` — Align on the final path.

Key changes:
- Align root data dir to `~/.outfitter/blz` (macOS: `~/Library/Application Support/outfitter.blz` via directories crate; ensure docs mention platform mapping).
- Optional migration: if we detect old `outfitter/cache` layout with `llms.json`, log a warning with simple migration guidance or copy-on-first-access (best-effort).

Acceptance criteria:
- Fresh installs use the documented path; existing installs see a clear message or automatic migration on first run.

Risks & mitigations:
- Breaking existing users → print actionable guidance and keep a fallback one-time copy where feasible.

### 3) feat/cli/update-mvp

Title: P0: Implement `update` command (ETag/Last-Modified) + archive & reindex

Primary files:
- `crates/blz-cli/src/commands/update.rs` — Implement logic for `update <alias>` and `update --all`.
- `crates/blz-core/src/fetcher.rs` — Ensure `fetch_with_cache` returns `NotModified` vs `Modified { content, etag, last_modified, sha256 }` robustly.
- `crates/blz-core/src/storage.rs` — Reuse `archive()` before overwriting content; write `llms.txt`, regenerate `llms.json`, rebuild index.
- Docs: `docs/cli.md` — Update `update` section with behavior + examples.

Flow:
1) Load `llms.json` to get last `etag`/`last_modified`/`sha256`.
2) Call `fetch_with_cache`.
3) If `NotModified`: print "Up-to-date".
4) If `Modified`: `archive(alias)`, write new `llms.txt`, parse, write new `llms.json`, rebuild Tantivy index.
5) Print summary (lines, diagnostics count, updated at).

Acceptance criteria:
- Wiremock-based tests: 304 path (no update) and 200 path (reindex + archive).
- CLI works for both `update --all` and per-alias.

Risks & mitigations:
- Large reindexing time for big docs → print progress; rely on Tantivy mmap for speed.

### 4) perf/search/limit-and-concurrency

Title: P0: Reduce over-fetching; parallel multi-source search; robust dedupe

Primary files:
- `crates/blz-cli/src/commands/search.rs`

Key changes:
- Effective limit: `effective = if options.all { 10_000 } else { (options.limit * 3).min(1000) }` passed to index search, not 10k.
- Parallelize per-source search using bounded concurrency (`FuturesUnordered` / `buffer_unordered`), then merge, dedupe, sort by score.
- Keep existing dedupe by `(alias, lines, heading_path)`.

Acceptance criteria:
- Lower latency on multi-source queries versus sequential.
- No change in correctness; pagination still works; memory usage bounded.

Risks & mitigations:
- Ordering differences due to concurrency → explicit sort by score; deterministic tie-breakers.

### 5) chore/lints-and-cli-polish

Title: P0: Tighten lints; hide `diff`; friendlier errors (404→NotFound)

Primary files:
- `crates/blz-core/src/lib.rs` — Remove broad `#![allow(...)]` lines; scope any required allows to items.
- `crates/blz-core/src/fetcher.rs` — Map HTTP 404 to `Error::NotFound(url)` for clearer UX.
- `crates/blz-cli/src/cli.rs` — Hide `Diff` command from help (`#[command(hide = true)]`) or remove temporarily; update tests/docs accordingly.
- Docs: `docs/cli.md` — Remove/mark `diff` as “coming soon”.

Acceptance criteria:
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- CLI help no longer shows `diff` (or clearly marks it experimental off by default).
- Error messages: 404 shows a helpful suggestion (try `lookup` or check URL).

Risks & mitigations:
- Minor churn fixing pedantic lints; keep changes surgical.

### 6) docs/release-notes-and-known-limitations

Title: P1: Release notes; storage path clarity; MCP schema stability

Primary files:
- `README.md`, `docs/cli.md`, `docs/architecture.md`, `docs/mcp.md`

Key updates:
- Release notes (what’s in v0.1; known limitations—`diff` disabled, no incremental indexing yet).
- Clarify storage paths per OS.
- Document MCP response shape (stable keys), recommend NDJSON/JSON for agents.

Acceptance criteria:
- Docs match behavior; examples tested.

### Upstack (post-release)

7) `feat/core/feature-flag-optimized`
- Feature-gate `optimized_index`, `memory_pool`, `string_pool`, `cache`, `async_io` behind `optimized`.
- Keep internal until fully integrated via CLI toggles.

8) `perf/index/boost-heading-path`
- Adjust Tantivy `QueryParser` with field boosts (e.g., 2–3× for `heading_path`).
- Benchmark; add regression tests for relevance (best-effort).

9) `tests/e2e/cli-integration`
- Scripted tests running `add`, `search`, `get`, `list`, `update` against temp dirs + wiremock.

## Graphite Workflow (per PR)

For each PR in the stack:

1) Implement changes in working tree.
2) `git add -A`
3) `gt create -m "<type>: <short title>\n\n<concise body>"`
4) After final PR: `gt submit --no-interactive` (submits full stack as drafts).
5) Paste `/github/pr/…` link for the base PR in the issue.

Example commit messages (first lines become PR titles):
- `fix(parser): correct heading-block extraction (exact line slices)`
- `fix(storage): unify storage path to ~/.outfitter/blz; docs + migration`
- `feat(cli): implement update (ETag/Last-Modified); archive + reindex`
- `perf(search): reduce over-fetch and parallelize per-source`
- `chore(lints): tighten clippy; hide diff; 404→NotFound`
- `docs(release): notes, storage paths, MCP shape stability`

## Test Plan (aggregate)

Unit:
- Parser: sentinel-based correctness, Unicode, deep nesting, line-range accuracy.
- Fetcher: 304/200 flows; 404 mapping.
- Storage: path formation, alias validation, archive naming.

Integration:
- CLI: `add → search → get → list → update` on temporary dirs; wiremock for HTTP.
- Search concurrency: compare sequential vs parallel timing sanity.

Performance (smoke):
- Index: <150ms/MB typical; search: baseline under ~50–100ms for small corpora.

## Risks and Mitigations

- Parser regressions → rigorous tests incl. Unicode and edge cases; property tests retained.
- Path migration confusion → loud logs and docs; optional one-time copy.
- Concurrency nondeterminism → explicit sorting; stable tie-breakers.
- Lint tightening churn → scope allows to small regions; avoid disabling groups globally.

## Rollout & Timeline

- Day 0: Parser fix (PR 1) and storage path (PR 2).
- Day 1: Update MVP (PR 3) and search perf (PR 4).
- Day 2: Lints/CLI polish (PR 5) and docs (PR 6). Submit stack, iterate on review.

## Ownership

- Core (parser, storage, index): Core maintainers (@galligan).
- CLI (commands, output, docs): CLI maintainer.
- MCP docs/output shape: MCP maintainer.

## Notes / Future Considerations

- Incremental indexing: maintain change logs and partial reindex surfaces.
- Streaming search results and early termination strategies.
- Optional vector/semantic search path (feature-flagged, opt-in).

