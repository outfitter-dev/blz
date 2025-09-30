#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use anyhow::Result;
use common::blz_cmd;
use tempfile::tempdir;

#[test]
fn config_command_manages_scopes() -> Result<()> {
    let home_dir = tempdir()?;
    let data_dir = tempdir()?;
    let config_dir = tempdir()?;
    let global_dir = tempdir()?;
    let work_dir = tempdir()?;

    let run_cmd = |args: &[&str]| {
        blz_cmd()
            .env("HOME", home_dir.path())
            .env("XDG_CONFIG_HOME", home_dir.path())
            .env("BLZ_DATA_DIR", data_dir.path())
            .env("BLZ_CONFIG_DIR", config_dir.path())
            .env("BLZ_GLOBAL_CONFIG_DIR", global_dir.path())
            .current_dir(work_dir.path())
            .args(args)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    };

    let _ = run_cmd(&[
        "config",
        "set",
        "add.prefer_full",
        "true",
        "--scope",
        "global",
    ]);
    let _ = run_cmd(&[
        "config",
        "set",
        "add.prefer_full",
        "false",
        "--scope",
        "project",
    ]);
    let _ = run_cmd(&[
        "config",
        "set",
        "add.prefer_full",
        "true",
        "--scope",
        "local",
    ]);

    let summary = run_cmd(&["config", "get"]);
    let summary_text = String::from_utf8(summary)?;
    assert!(summary_text.contains("  global : true"));
    assert!(summary_text.contains("  project: false"));
    assert!(summary_text.contains("  local  : true"));
    assert!(summary_text.contains("  effective: true"));

    let project_value = run_cmd(&["config", "get", "add.prefer_full", "--scope", "project"]);
    let project_text = String::from_utf8(project_value)?;
    assert!(project_text.contains("add.prefer_full [project"));
    assert!(project_text.trim_end().ends_with("= false"));

    Ok(())
}

// Test removed: flavor preference is no longer supported in simplified single-flavor model
