#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for batch get operations with multiple line ranges

mod common;

use common::{add_source, blz_cmd_with_dirs};
use predicates::prelude::*;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn serve_test_content(content: String) -> (MockServer, String) {
    let server = MockServer::start().await;

    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(content))
        .mount(&server)
        .await;

    let url = format!("{}/llms.txt", server.uri());
    (server, url)
}

#[tokio::test]
async fn test_get_multiple_ranges_comma_separated() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    // Create test content with numbered lines
    let test_content = (1..=20)
        .map(|i| format!("Line {i}: This is test content."))
        .collect::<Vec<_>>()
        .join("\n");

    let (_server, url) = serve_test_content(test_content).await;
    add_source("batch-test", &url, data_dir.path(), config_dir.path());

    // Get multiple ranges with comma separation
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("get")
        .arg("batch-test")
        .arg("--lines")
        .arg("1-3,5-7,10")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    let requests = json["requests"].as_array().expect("requests array");
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(
        request["alias"].as_str().unwrap(),
        "batch-test",
        "alias should echo request"
    );

    let ranges = request["ranges"].as_array().expect("ranges array");
    assert_eq!(ranges.len(), 3, "expected three discrete ranges");

    let starts: Vec<u64> = ranges
        .iter()
        .map(|range| range["lineStart"].as_u64().unwrap())
        .collect();
    assert_eq!(starts, vec![1, 5, 10], "lineStart mismatch");

    let ends: Vec<u64> = ranges
        .iter()
        .map(|range| range["lineEnd"].as_u64().unwrap())
        .collect();
    assert_eq!(ends, vec![3, 7, 10], "lineEnd mismatch");

    let snippets: Vec<&str> = ranges
        .iter()
        .map(|range| range["snippet"].as_str().unwrap())
        .collect();
    assert!(
        snippets[0].contains("Line 1:"),
        "Range 1 should include line 1"
    );
    assert!(
        snippets[0].contains("Line 3:"),
        "Range 1 should include line 3"
    );
    assert!(
        snippets[1].contains("Line 5:"),
        "Range 2 should include line 5"
    );
    assert!(
        snippets[1].contains("Line 7:"),
        "Range 2 should include line 7"
    );
    assert!(
        snippets[2].contains("Line 10:"),
        "Single-line range should include line 10"
    );
}

#[tokio::test]
async fn test_get_multiple_ranges_with_context() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    // Create test content
    let test_content = (1..=30)
        .map(|i| format!("Line {i}"))
        .collect::<Vec<_>>()
        .join("\n");

    let (_server, url) = serve_test_content(test_content).await;
    add_source("batch-context", &url, data_dir.path(), config_dir.path());

    // Get multiple ranges with context
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("get")
        .arg("batch-context")
        .arg("--lines")
        .arg("5,15,25")
        .arg("--context")
        .arg("2")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    let requests = json["requests"].as_array().expect("requests array");
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(
        request["contextApplied"].as_u64().unwrap(),
        2,
        "contextApplied should reflect symmetric context"
    );

    let ranges = request["ranges"].as_array().expect("ranges array");
    assert_eq!(ranges.len(), 3);

    let expected = [(3_u64, 7_u64), (13, 17), (23, 27)];
    for (range, (start, end)) in ranges.iter().zip(expected) {
        assert_eq!(
            range["lineStart"].as_u64().unwrap(),
            start,
            "lineStart mismatch with context"
        );
        assert_eq!(
            range["lineEnd"].as_u64().unwrap(),
            end,
            "lineEnd mismatch with context"
        );
        let snippet = range["snippet"].as_str().unwrap();
        assert!(
            snippet.contains(&format!("Line {start}")),
            "snippet should include contextual start line {start}"
        );
        assert!(
            snippet.contains(&format!("Line {end}")),
            "snippet should include contextual end line {end}"
        );
    }
}

#[tokio::test]
async fn test_get_multiple_ranges_mixed_formats() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    // Create test content
    let test_content = (1..=50)
        .map(|i| format!("Line {i}"))
        .collect::<Vec<_>>()
        .join("\n");

    let (_server, url) = serve_test_content(test_content).await;
    add_source("batch-mixed", &url, data_dir.path(), config_dir.path());

    // Get with mixed range formats: A-B, A+N, single line
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("get")
        .arg("batch-mixed")
        .arg("--lines")
        .arg("5-10,20+3,35,40-42")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    let requests = json["requests"].as_array().expect("requests array");
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    let ranges = request["ranges"].as_array().expect("ranges array");
    assert_eq!(ranges.len(), 4);

    let expected = [(5_u64, 10_u64), (20, 22), (35, 35), (40, 42)];
    for (range, (start, end)) in ranges.iter().zip(expected) {
        assert_eq!(
            range["lineStart"].as_u64().unwrap(),
            start,
            "lineStart mismatch for mixed formats"
        );
        assert_eq!(
            range["lineEnd"].as_u64().unwrap(),
            end,
            "lineEnd mismatch for mixed formats"
        );
    }
}

#[tokio::test]
async fn test_get_overlapping_ranges_merged() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    // Create test content
    let test_content = (1..=20)
        .map(|i| format!("Line {i}"))
        .collect::<Vec<_>>()
        .join("\n");

    let (_server, url) = serve_test_content(test_content).await;
    add_source("batch-overlap", &url, data_dir.path(), config_dir.path());

    // Get overlapping ranges - expect separate entries
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("get")
        .arg("batch-overlap")
        .arg("--lines")
        .arg("5-10,8-12,11-15")  // Overlapping ranges
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    let requests = json["requests"].as_array().expect("requests array");
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    let ranges = request["ranges"].as_array().expect("ranges array");
    assert_eq!(ranges.len(), 3);

    let expected = [(5_u64, 10_u64), (8, 12), (11, 15)];
    for (range, (start, end)) in ranges.iter().zip(expected) {
        assert_eq!(
            range["lineStart"].as_u64().unwrap(),
            start,
            "lineStart mismatch for overlapping range"
        );
        assert_eq!(
            range["lineEnd"].as_u64().unwrap(),
            end,
            "lineEnd mismatch for overlapping range"
        );
    }
}

#[tokio::test]
async fn test_get_invalid_range_in_batch() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    // Create small test content
    let test_content = "Line 1\nLine 2\nLine 3\n";

    let (_server, url) = serve_test_content(test_content.to_string()).await;
    add_source("batch-invalid", &url, data_dir.path(), config_dir.path());

    // Try to get with some invalid ranges
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("get")
        .arg("batch-invalid")
        .arg("--lines")
        .arg("1-2,invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid line specification"));
}

#[tokio::test]
async fn test_get_rejects_zero_count_range() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let test_content = (1..=20)
        .map(|i| format!("Line {i}"))
        .collect::<Vec<_>>()
        .join("\n");

    let (_server, url) = serve_test_content(test_content).await;
    add_source("batch-zero", &url, data_dir.path(), config_dir.path());

    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("get")
        .arg("batch-zero")
        .arg("--lines")
        .arg("5+0")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid line specification"));
}

#[tokio::test]
async fn test_get_jsonl_outputs_one_line_per_entry() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let test_content = (1..=10)
        .map(|i| format!("Line {i}"))
        .collect::<Vec<_>>()
        .join("\n");

    let (_server, url) = serve_test_content(test_content).await;
    add_source("batch-jsonl", &url, data_dir.path(), config_dir.path());

    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let output = cmd
        .arg("get")
        .arg("batch-jsonl")
        .arg("--lines")
        .arg("1-2,4")
        .arg("--format")
        .arg("jsonl")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let lines: Vec<&str> = stdout.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "JSONL should emit a single line for the response"
    );
    let value: Value = serde_json::from_str(lines[0]).expect("valid jsonl entry");
    let requests = value["requests"].as_array().expect("requests array");
    assert_eq!(requests.len(), 1);
    let ranges = requests[0]["ranges"].as_array().expect("ranges array");
    assert_eq!(ranges.len(), 2, "expected two discrete ranges");
    assert_eq!(ranges[0]["lineStart"].as_u64().unwrap(), 1);
    assert_eq!(ranges[0]["lineEnd"].as_u64().unwrap(), 2);
    assert_eq!(ranges[1]["lineStart"].as_u64().unwrap(), 4);
    assert_eq!(ranges[1]["lineEnd"].as_u64().unwrap(), 4);
}
