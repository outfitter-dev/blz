#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn history_handles_empty_state() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let config_dir = tempdir()?;

    let out = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", config_dir.path())
        .args(["history", "--limit", "3", "--format", "text"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(out)?;
    assert!(text.contains("No recent searches recorded."));
    Ok(())
}

#[tokio::test]
async fn history_captures_recent_searches() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let config_dir = tempdir()?;

    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());
    let doc = "# Title\n\n## Section\nRust history test\n";
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(doc))
        .mount(&server)
        .await;

    // add source
    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", config_dir.path())
        .args(["add", "fixture", &url, "-y"])
        .assert()
        .success();

    // run first search (json output)
    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", config_dir.path())
        .args([
            "search", "history", "--source", "fixture", "--format", "json",
        ])
        .assert()
        .success();

    // run second search with custom snippet & precision so defaults update
    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", config_dir.path())
        .args([
            "search",
            "history",
            "--source",
            "fixture",
            "--format",
            "text",
            "--show",
            "url",
            "--snippet-lines",
            "4",
            "--score-precision",
            "2",
        ])
        .assert()
        .success();

    // history text should reflect defaults from last search
    let text_out = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", config_dir.path())
        .args(["history", "--limit", "2", "--format", "text"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(text_out)?;
    assert!(text.contains("Default show: url"));
    assert!(text.contains("Default snippet lines: 4"));
    assert!(text.contains("Default score precision: 2"));
    assert!(text.contains("1."));

    // history JSON should include two entries in reverse chronological order
    let json_out = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", config_dir.path())
        .args(["history", "--limit", "2", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<Value> = serde_json::from_slice(&json_out)?;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["format"], "text");
    assert_eq!(entries[0]["snippet_lines"], 4);
    assert_eq!(entries[0]["score_precision"], 2);
    assert!(
        entries[0]["show"]
            .as_array()
            .unwrap()
            .contains(&Value::from("url"))
    );

    assert_eq!(entries[1]["format"], "json");
    assert!(entries[1]["show"].as_array().unwrap().is_empty());

    Ok(())
}
