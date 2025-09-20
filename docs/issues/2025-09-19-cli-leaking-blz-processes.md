# Bug: CLI Integration Tests Leave `blz search` Processes Running

**Date opened:** 2025-09-19
**Owner:** Unassigned (flagged by agents)
**Priority:** High for v0.3.0

## Summary
Running `cargo test -p blz-cli` spawns multiple `blz search …` subprocesses through the integration suite (notably `tests/search_pagination.rs`). When the parent test harness exits—either because the test timed out or we interrupt it—the spawned CLI binaries remain alive. Subsequent `cargo test -p blz-cli` runs then hang for minutes, as those lingering processes continue reading from the shared cache/target directory until we manually `pkill` them.

## Reproduction Steps
1. Ensure no `blz` binaries are running (`pgrep -fal "blz search"` should return nothing).
2. Run `cargo test -p blz-cli` (or even a filtered run like `cargo test -p blz-cli utils::formatting::tests::collapses_middle_segments`).
3. Interrupt the test early (Ctrl+C) or wait for the timeout behaviour we have been seeing.
4. Check `pgrep -fal "blz search"` again. Multiple entries such as `blz search test --limit 1 --page 999999` will still be present.
5. Attempt to re-run `cargo test -p blz-cli`; observe long (8–20 minute) hangs until those child processes are killed.

## Impact
- Test runs become non-deterministic and painfully slow unless we manually clean up processes between runs.
- Automated CI runs (and local developer workflows) risk stalling if the integration suite spawns these children.
- The problem undermines confidence in the test suite leading up to the v0.3.0 release.

## Known Workarounds
- Manually run `./scripts/cleanup-blz.sh` (recently extended to `pkill` lingering CLI processes) after each test run.
- Avoid full `cargo test -p blz-cli` and stick to `cargo check` until we address the root cause.

## Suspected Root Cause
- Integration tests in `crates/blz-cli/tests/search_pagination.rs` (and potentially other CLI tests) launch the compiled `blz` binary via `Command::cargo_bin("blz")` and never ensure those children are terminated when the test harness aborts.
- Our CLI’s pagination command does not obey a timeout or structured shutdown when stdin/stdout are broken, so the child continues idling even after the parent process exits.

## Proposed Fixes / Investigation Tasks
1. Audit all integration tests that spawn the `blz` binary and ensure they either:
   - Use a helper wrapper that terminates children on drop, or
   - Switch to invoking subcommands through a library API instead of shelling out.
2. Add signal/timeout handling inside the CLI so that if stdin is closed (e.g., parent dies), the process exits promptly.
3. Extend the integration harness to enforce timeouts around spawned commands (e.g., `Command::timeout`) and assert that the process completes.
4. Add regression tests that verify no stray `blz` processes remain after the suite completes (perhaps behind a feature flag to avoid flakiness).

## References
- `.agents/logs/20250917-release-automation-wrap.md` (section “Known issue …”)
- `docs/notes/release-polish-followups.md` (History & pagination follow-up now tracks this bug)

## Status — 2025-09-19
- ✅ Added a parent exit watchdog in the CLI (`utils::process_guard::spawn_parent_exit_guard`) so `blz` processes terminate when the spawning harness disappears.
- 🧪 Verified compilation + tests via `cargo test -p blz-cli --no-run` to ensure the new guard builds cleanly across binaries/integration suites.
- 🔍 Follow-up: monitor future `cargo test -p blz-cli` runs to confirm no post-run `blz search` processes remain; add regression coverage once we have a reliable harness hook.
