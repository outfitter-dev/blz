#![allow(missing_docs)]

mod common;

use anyhow::Result;
use blz_core::Storage;
use common::blz_cmd;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

#[tokio::test]
async fn add_respects_prefer_full_setting() -> Result<()> {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/llms-full.txt"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "2048"))
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "1024"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms-full.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("# Full doc"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("# Base doc"))
        .mount(&server)
        .await;

    let home_dir = tempdir()?;
    let data_dir = tempdir()?;
    let config_dir = tempdir()?;
    let global_dir = tempdir()?;
    let work_dir = tempdir()?;

    let base_url = server.uri();
    let source_url = format!("{base_url}/llms.txt");

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

    run_cmd(&[
        "config",
        "set",
        "add.prefer_full",
        "true",
        "--scope",
        "local",
    ]);
    let prefer_full_output = run_cmd(&["add", "fullpref", &source_url, "--yes"]);
    let prefer_full_text = String::from_utf8(prefer_full_output)?;
    assert!(prefer_full_text.contains("llms-full"));
    assert!(prefer_full_text.contains("llms"));
    assert!(prefer_full_text.contains("✓ Added"));

    run_cmd(&[
        "config",
        "set",
        "add.prefer_full",
        "false",
        "--scope",
        "local",
    ]);
    let prefer_base_output = run_cmd(&["add", "basepref", &source_url, "--yes"]);
    let prefer_base_text = String::from_utf8(prefer_base_output)?;
    assert!(prefer_base_text.contains("llms"));

    let storage = Storage::with_root(data_dir.path().to_path_buf())?;

    let full_json = storage
        .load_flavor_json("fullpref", "llms-full")?
        .expect("fullpref llms-full.json present");
    assert!(
        full_json
            .files
            .first()
            .map(|f| f.path.contains("llms-full.txt"))
            .unwrap_or(false)
    );

    let base_json = storage
        .load_flavor_json("basepref", "llms")?
        .expect("basepref llms.json present");
    assert!(
        base_json
            .files
            .first()
            .map(|f| f.path.ends_with("llms.txt"))
            .unwrap_or(false)
    );

    Ok(())
}
