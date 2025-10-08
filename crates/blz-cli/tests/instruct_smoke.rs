#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd;
use serde_json::Value;

#[test]
fn prompt_global_returns_json() -> anyhow::Result<()> {
    let mut cmd = blz_cmd();
    let stdout = cmd
        .arg("--prompt")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&stdout)?;
    assert_eq!(value["target"], "blz");
    let summary = value["summary"].as_str().unwrap().to_ascii_lowercase();
    assert!(summary.contains("local-first"));
    Ok(())
}

#[test]
fn prompt_for_specific_command() -> anyhow::Result<()> {
    let mut cmd = blz_cmd();
    let stdout = cmd
        .args(["--prompt", "search"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&stdout)?;
    assert_eq!(value["target"], "search");
    assert!(value["primary_usage"].is_array());
    Ok(())
}
