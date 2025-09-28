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
        .arg("1:3,5-7,10")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    // Should have combined content from all ranges
    let content = json["content"].as_str().unwrap();
    assert!(content.contains("Line 1:"), "Should include line 1");
    assert!(content.contains("Line 2:"), "Should include line 2");
    assert!(content.contains("Line 3:"), "Should include line 3");
    assert!(!content.contains("Line 4:"), "Should NOT include line 4");
    assert!(content.contains("Line 5:"), "Should include line 5");
    assert!(content.contains("Line 6:"), "Should include line 6");
    assert!(content.contains("Line 7:"), "Should include line 7");
    assert!(!content.contains("Line 8:"), "Should NOT include line 8");
    assert!(content.contains("Line 10:"), "Should include line 10");

    // Check line numbers array
    let line_nums = json["lineNumbers"].as_array().unwrap();
    let got: Vec<i64> = line_nums.iter().map(|v| v.as_i64().unwrap()).collect();
    let expected: Vec<i64> = vec![1, 2, 3, 5, 6, 7, 10];
    assert_eq!(got, expected, "lineNumbers mismatch");
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

    let content = json["content"].as_str().unwrap();

    // Should include context around each line
    // Line 5 with context should include 3-7
    assert!(
        content.contains("Line 3"),
        "Should include context before line 5"
    );
    assert!(content.contains("Line 5"), "Should include line 5");
    assert!(
        content.contains("Line 7"),
        "Should include context after line 5"
    );

    // Line 15 with context should include 13-17
    assert!(
        content.contains("Line 13"),
        "Should include context before line 15"
    );
    assert!(content.contains("Line 15"), "Should include line 15");
    assert!(
        content.contains("Line 17"),
        "Should include context after line 15"
    );
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

    let line_nums = json["lineNumbers"].as_array().unwrap();

    // Should have: 6 (from 5-10) + 3 (from 20+3) + 1 (from 35) + 3 (from 40-42) = 13 lines
    let line_nums_vec: Vec<i64> = line_nums.iter().map(|v| v.as_i64().unwrap()).collect();
    assert_eq!(line_nums_vec.len(), 13);

    let expected: Vec<i64> = vec![5, 6, 7, 8, 9, 10, 20, 21, 22, 35, 40, 41, 42];
    assert_eq!(
        line_nums_vec, expected,
        "lineNumbers mismatch for mixed formats"
    );
    assert!(
        line_nums_vec.windows(2).all(|window| window[0] < window[1]),
        "lineNumbers should be strictly increasing"
    );
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

    // Get overlapping ranges - should be merged
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

    let line_nums = json["lineNumbers"].as_array().unwrap();

    // Should merge to continuous range 5-15 (11 lines total)
    let nums: Vec<i64> = line_nums.iter().map(|v| v.as_i64().unwrap()).collect();
    assert_eq!(nums.len(), 11);
    assert_eq!(nums.first(), Some(&5));
    assert_eq!(nums.last(), Some(&15));
    assert!(nums.windows(2).all(|window| window[0] < window[1]));

    // No duplicates
    let mut seen = std::collections::HashSet::new();
    for n in &nums {
        assert!(seen.insert(*n), "Line {n} appeared twice");
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
        .stderr(predicate::str::contains("Invalid --lines format"));
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
        .stderr(predicate::str::contains("Invalid --lines format"));
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
    assert_eq!(value["lineNumbers"].as_array().unwrap().len(), 3);
}
