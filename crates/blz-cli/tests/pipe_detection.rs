#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for automatic JSON output when piped

mod common;

use assert_cmd::cargo::cargo_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn blz_binary_path() -> PathBuf {
    cargo_bin("blz")
}

fn command_with_env(bin: &Path, data_dir: &Path, config_dir: &Path) -> Command {
    let mut cmd = Command::new(bin);
    cmd.env("BLZ_DISABLE_GUARD", "1")
        .env("BLZ_FORCE_NON_INTERACTIVE", "1")
        .env("BLZ_SUPPRESS_DEPRECATIONS", "1")
        .env("NO_COLOR", "1")
        .env("BLZ_DATA_DIR", data_dir)
        .env("BLZ_CONFIG_DIR", config_dir);
    cmd
}

#[tokio::test]
async fn test_search_outputs_json_when_piped() {
    let bin = blz_binary_path();
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("# Title\n\nSome content for pipe detection tests.\n"),
        )
        .mount(&server)
        .await;
    let url = format!("{}/llms.txt", server.uri());

    let mut add_cmd = command_with_env(&bin, data_dir.path(), config_dir.path());
    let add_output = add_cmd
        .arg("add")
        .arg("pipe-search-json")
        .arg(&url)
        .arg("-y")
        .output()
        .expect("Failed to add source");
    assert!(
        add_output.status.success(),
        "failed to add source: {}{}",
        String::from_utf8_lossy(&add_output.stdout),
        String::from_utf8_lossy(&add_output.stderr)
    );

    // Run blz search piped to cat (simulating pipe usage)
    let mut cmd = command_with_env(&bin, data_dir.path(), config_dir.path());
    let output = cmd
        .arg("search")
        .arg("Title")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute blz");

    assert!(
        output.status.success(),
        "blz search failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // When piped and no format specified, should output JSON
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Try to parse as JSON - should succeed if it's JSON format
    if !stdout.is_empty() && !stdout.contains("No sources found") {
        let result: Result<Value, _> = serde_json::from_str(&stdout);
        assert!(
            result.is_ok(),
            "Expected JSON output when piped, got: {stdout}"
        );
    }
}

#[tokio::test]
async fn test_search_respects_explicit_format_when_piped() {
    let bin = blz_binary_path();
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("# Title\n\nSome content for pipe detection tests.\n"),
        )
        .mount(&server)
        .await;
    let url = format!("{}/llms.txt", server.uri());

    let mut add_cmd = command_with_env(&bin, data_dir.path(), config_dir.path());
    let add_output = add_cmd
        .arg("add")
        .arg("pipe-search-text")
        .arg(&url)
        .arg("-y")
        .output()
        .expect("Failed to add source");
    assert!(
        add_output.status.success(),
        "failed to add source: {}{}",
        String::from_utf8_lossy(&add_output.stdout),
        String::from_utf8_lossy(&add_output.stderr)
    );

    // Even when piped, explicit --format text should be respected
    let mut cmd = command_with_env(&bin, data_dir.path(), config_dir.path());
    let output = cmd
        .arg("search")
        .arg("Title")
        .arg("--format")
        .arg("text")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute blz");

    assert!(
        output.status.success(),
        "blz search --format text failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT be JSON when explicitly requesting text
    if !stdout.is_empty() && !stdout.contains("No sources found") {
        let result: Result<Value, _> = serde_json::from_str(&stdout);
        assert!(
            result.is_err(),
            "Should output text when explicitly requested, even when piped"
        );
    }
}

#[test]
fn test_list_outputs_json_when_piped() {
    let bin = blz_binary_path();
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let mut cmd = command_with_env(&bin, data_dir.path(), config_dir.path());
    let output = cmd
        .arg("list")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute blz");

    assert!(
        output.status.success(),
        "blz list failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // When piped and no format specified, should output JSON
    if !stdout.is_empty() && !stdout.contains("No sources") {
        let value: Value = serde_json::from_str(&stdout).expect("Expected JSON output when piped");
        assert!(
            value.is_array(),
            "Expected list JSON to be an array, got: {value}"
        );
    }
}

#[tokio::test]
async fn test_get_outputs_json_when_piped() {
    let bin = blz_binary_path();
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    // Serve a small llms.txt over HTTP so add/get can succeed
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("# Title\n\n## Section\nLine 1\nLine 2\nLine 3\n"),
        )
        .mount(&server)
        .await;
    let url = format!("{}/llms.txt", server.uri());

    // Add source
    let mut add_cmd = command_with_env(&bin, data_dir.path(), config_dir.path());
    let add_output = add_cmd
        .arg("add")
        .arg("pipe-test")
        .arg(&url)
        .arg("-y")
        .output()
        .expect("Failed to add source");
    assert!(
        add_output.status.success(),
        "failed to add source: {}{}",
        String::from_utf8_lossy(&add_output.stdout),
        String::from_utf8_lossy(&add_output.stderr)
    );

    // Test get command when piped
    let mut get_cmd = command_with_env(&bin, data_dir.path(), config_dir.path());
    let output = get_cmd
        .arg("get")
        .arg("pipe-test")
        .arg("--lines")
        .arg("1-3")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute blz");

    assert!(
        output.status.success(),
        "blz get failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // When piped and no format specified, should output JSON
    if !stdout.is_empty() && !stdout.contains("Source not found") {
        let result: Result<Value, _> = serde_json::from_str(&stdout);
        assert!(
            result.is_ok(),
            "Expected JSON output when piped, got: {stdout}"
        );

        if let Ok(json) = result {
            let request = json["requests"]
                .as_array()
                .and_then(|arr| arr.first())
                .expect("request entry");
            assert_eq!(request["alias"], "pipe-test");
            let snippet = request.get("snippet").and_then(Value::as_str);
            let ranges = request.get("ranges").and_then(Value::as_array);

            assert!(
                snippet.is_some() || ranges.is_some_and(|arr| !arr.is_empty()),
                "Expected snippet or ranges in JSON output"
            );

            if let Some(snippet) = snippet {
                assert!(!snippet.is_empty());
            } else if let Some(array) = ranges {
                assert!(!array.is_empty());
                assert!(array[0]["snippet"].is_string());
            }
        }
    }
}
