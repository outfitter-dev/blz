# 2025-09-17 — Release Automation Wrap-Up

## Context
- Finalized CI release hardening work from PR #179
- Scoped npm publish permissions and added auth checks
- Ran local `act` dry run (npm path) to verify manual publish workflow
- Follow-up adjustments after merge to enforce Windows build parity and wire actionlint tooling

## Notes
- `manual-publish.yml` now limits `id-token` scope to npm jobs
- Increased crates propagation wait loop to 24 attempts (10s interval)
- Added `npm whoami` preflight for stable & canary paths
- Local dry-run used dockerized `act` with `publish_crates=false` to avoid index mismatch on unpublished versions; npm dry-run completed successfully
- `publish.yml` now requires Windows builds to pass (removed `continue-on-error`) and exposes `workflow_call` for reuse by `release.yml`
- `lefthook` pre-commit runs `actionlint` when workflow files change to catch structural errors early
- Post-merge polish tightened reusable publish permissions, expanded tag validation, and fixed artifact flattening to handle nested `target/release` paths
- Updated `publish.yml` to place downloaded artifacts in deterministic per-target directories and harden the flatten helper against missing Windows zips (with deeper search fallback + duplicate-safe moves)
- Publish workflow now extracts archives to provide raw platform binaries alongside compressed bundles (fixes npm postinstall 404s)
- Ran `scripts/release/semver-bump.sh set 0.2.4` to prep the next release after the npm 0.2.1 asset gap (syncing Cargo manifests, lockfile, npm metadata, and CHANGELOG)

## Follow-ups
- None; monitor next real publish run for parity with dry-run results
- If crates dry-run coverage required later, pre-seed registry or adjust test payload to skip until publish events

## 2025-09-18
- Patched npm wrapper to pass `argv0 = "blz"` so clap help consistently shows the canonical executable name even when dispatching arch-specific binaries.
- Adjusted CLI usage banner to present command-first syntax with an alternate line for command-less searches and hide default-search args from subcommand help.
- Investigated prior CLI formatting work: located `feat/cli-polish` branch with compact result formatting, alias grouping, summary tips, and JSON `json-full` envelope option that never merged to main.
- Ported compact search formatter, `--format` flag rename, and `--show` modifiers into `gt-v0.2/feat/release-polish`; updated JSONL naming, docs, tests, and completions to match the new surface.
- Trimmed `blz instruct` output to curated notes + docs pointer; refreshed shell scripts/docs to use `--format` and staged the new branch in Graphite.
- Updated brief search layout to the latest scratchpad mock (rank/score banner, parenthetical path, hashed heading line, two-space indentation, arrow summary with source count) and removed the redundant `show_rank` toggle.
- Added path truncation for deeply nested headings (keep first and last two segments with ellipsis) so parenthetical context stays readable.
- Introduced a shared Cargo target cache (`../.blz-target`) via `.cargo/config.toml` and added `scripts/cleanup-blz.sh` (now handles both cached binaries and lingering `blz search` invocations) to kill stray test instances or prune the cache when needed.
- Verified the cache + cleanup flow: after running `./scripts/cleanup-blz.sh` the targeted formatter test completes in ~0.2s with no residual `blz` processes. If future test runs feel slow, run the cleanup script first to clear runaway CLI children before retrying.
- Known issue (must fix for v0.3.0): integration tests that spawn the CLI (e.g. `tests/search_pagination.rs`) leave background `blz search …` processes alive when the parent test harness exits. This causes subsequent `cargo test -p blz-cli` runs to hang for minutes. See GitHub issue #188 for investigation details and next steps.
- Registry lookup now short-circuits with a "coming soon" guidance block (pointing folks to `llms-full.txt` + `blz add`) while we finish the new catalog flow, and the CLI now persists search presentation defaults plus exposes `blz history` to inspect recent queries.

- ## 2025-09-19
- Added a parent-process watchdog (`utils::process_guard::spawn_parent_exit_guard`) so CLI children terminate if the spawning test harness or shell dies (fixes GitHub issue #188). Updated docs/notes and bug tracker entry; ran `cargo test -p blz-cli --no-run` to ensure the guard builds across binaries without warnings.
- Pulled orphan cleanup into a reusable `tests/common::blz_cmd()` helper that sets five-second timeouts and wires the guard env automatically. Updated every CLI integration test to use it and added `BLZ_PARENT_GUARD_TIMEOUT_SECS` support in the guard so runaway processes die within the configured window. Full `cargo test -p blz-cli` now completes without leaving stray `blz` binaries behind.
- Implemented persisted CLI history and preferences support (`blz history`), added dedicated integration coverage, and refreshed user docs (README quick start, command reference, config docs) to highlight the new command.
- Added `blz config` with scoped (global/local/project) settings management, wired `add` to honor the prefer-full flag, and introduced the new `blz.json`/`history.jsonl` stores for durable metadata.
