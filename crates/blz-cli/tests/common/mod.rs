#![allow(clippy::expect_used, clippy::unwrap_used)]

use assert_cmd::Command;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;
use tempfile::TempDir;

#[allow(dead_code)]
pub const CMD_TIMEOUT: Duration = Duration::from_secs(15);
#[allow(dead_code)]
pub const DEFAULT_GUARD_TIMEOUT_SECS: &str = "10";

#[allow(dead_code)]
fn data_dir() -> &'static Path {
    static DATA_DIR: OnceLock<TempDir> = OnceLock::new();
    DATA_DIR
        .get_or_init(|| tempfile::tempdir().expect("failed to create data dir for tests"))
        .path()
}

/// Create a configured `blz` command suitable for integration tests.
/// Ensures child processes are cleaned up even when the harness aborts.
#[allow(dead_code)]
pub fn blz_cmd() -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("blz"));
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
    cmd.env("BLZ_SUPPRESS_DEPRECATIONS", "1");
    cmd.env("NO_COLOR", "1");
    cmd
}

#[allow(dead_code)]
pub fn blz_cmd_with_dirs(data_dir: &Path, config_dir: &Path) -> Command {
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", data_dir);
    cmd.env("BLZ_CONFIG_DIR", config_dir);
    cmd
}

#[allow(dead_code)]
pub fn add_source(alias: &str, url: &str, data_dir: &Path, config_dir: &Path) {
    let mut cmd = blz_cmd_with_dirs(data_dir, config_dir);
    cmd.arg("add")
        .arg(alias)
        .arg(url)
        .arg("-y")
        .assert()
        .success();
}
