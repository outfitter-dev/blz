use assert_cmd::Command;
use std::time::Duration;

pub const CMD_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_GUARD_TIMEOUT_SECS: &str = "10";

/// Create a configured `blz` command suitable for integration tests.
/// Ensures child processes are cleaned up even when the harness aborts.
pub fn blz_cmd() -> Command {
    let mut cmd = Command::cargo_bin("blz").expect("blz binary should build for tests");
    cmd.timeout(CMD_TIMEOUT);
    if std::env::var("BLZ_PARENT_GUARD_TIMEOUT_SECS").is_err() {
        cmd.env("BLZ_PARENT_GUARD_TIMEOUT_SECS", DEFAULT_GUARD_TIMEOUT_SECS);
    }
    cmd
}
