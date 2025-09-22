#![allow(clippy::expect_used, clippy::unwrap_used)]

use assert_cmd::Command;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;
use tempfile::TempDir;

pub const CMD_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_GUARD_TIMEOUT_SECS: &str = "10";

fn data_dir() -> &'static Path {
    static DATA_DIR: OnceLock<TempDir> = OnceLock::new();
    DATA_DIR
        .get_or_init(|| tempfile::tempdir().expect("failed to create data dir for tests"))
        .path()
}

/// Create a configured `blz` command suitable for integration tests.
/// Ensures child processes are cleaned up even when the harness aborts.
pub fn blz_cmd() -> Command {
    let mut cmd = Command::cargo_bin("blz").expect("blz binary should build for tests");
    cmd.timeout(CMD_TIMEOUT);
    if std::env::var("BLZ_PARENT_GUARD_TIMEOUT_SECS").is_err() {
        cmd.env("BLZ_PARENT_GUARD_TIMEOUT_SECS", DEFAULT_GUARD_TIMEOUT_SECS);
    }
    cmd.env("BLZ_DISABLE_GUARD", "1");
    cmd.env("BLZ_FORCE_NON_INTERACTIVE", "1");
    let dir = data_dir();
    cmd.env("BLZ_DATA_DIR", dir);
    if std::env::var_os("BLZ_CONFIG").is_none() && std::env::var_os("BLZ_CONFIG_DIR").is_none() {
        cmd.env("BLZ_CONFIG_DIR", dir);
    }
    cmd.env("NO_COLOR", "1");
    cmd
}
