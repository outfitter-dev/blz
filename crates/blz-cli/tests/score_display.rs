#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for score percentage display

mod common;

use assert_cmd::Command;
use common::{add_source, blz_cmd_with_dirs};
use predicates::prelude::*;
use regex::Regex;
use tempfile::{TempDir, tempdir};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cmd_with_dir(dir: &TempDir) -> Command {
    blz_cmd_with_dirs(dir.path(), dir.path())
}

async fn serve_llms(body: &str) -> (MockServer, String) {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body.to_string()))
        .mount(&server)
        .await;
    let url = format!("{}/llms.txt", server.uri());
    (server, url)
}

#[tokio::test]
async fn test_search_shows_percentage_scores_in_text_mode() {
    // Create test content
    let (_server, url) = serve_llms(
        "# React Hooks\n\nUse useEffect for side effects.\nuseState manages state.\nuseCallback for memoization.\n",
    )
    .await;

    let dir = tempdir().expect("tempdir");

    // Add test source
    add_source("score-test", &url, dir.path(), dir.path());

    // Search and check for percentage display
    let mut cmd = cmd_with_dir(&dir);
    cmd.arg("search")
        .arg("useEffect")
        .arg("--source")
        .arg("score-test")
        .arg("--format")
        .arg("text")
        .assert()
        .success()
        .stdout(predicate::str::contains("100%")); // Top result should be 100%
}

#[tokio::test]
async fn test_search_percentage_scores_with_multiple_results() {
    // Create test content with varying relevance
    let (_server, url) = serve_llms(
        r"# Documentation

## useEffect Hook
The useEffect hook is essential for React applications.
useEffect useEffect useEffect - maximum relevance here.

## useState Hook
useState is another important hook.
This has less relevance to useEffect.

## General Hooks
React provides many hooks for different purposes.
Minimal relevance to our search term.
",
    )
    .await;

    let dir = tempdir().expect("tempdir");

    // Add test source
    add_source("score-multi", &url, dir.path(), dir.path());

    // Search and verify percentage ordering
    let mut cmd = cmd_with_dir(&dir);
    let result = cmd
        .arg("search")
        .arg("useEffect")
        .arg("--source")
        .arg("score-multi")
        .arg("--format")
        .arg("text")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&result.get_output().stdout);

    // Extract all percentages that look like "NNN%"
    let regex = Regex::new(r"(\d+)%").expect("valid regex");
    let percents: Vec<u32> = regex
        .captures_iter(&stdout)
        .filter_map(|caps| caps.get(1))
        .filter_map(|m| m.as_str().parse::<u32>().ok())
        .collect();
    assert!(
        !percents.is_empty(),
        "Should contain at least one percentage"
    );
    assert_eq!(percents[0], 100, "Top result should be 100%");
    assert!(
        percents.windows(2).all(|window| window[0] >= window[1]),
        "Percentages should be non-increasing"
    );
}

#[tokio::test]
async fn test_search_score_precision_flag() {
    // Create simple test content
    let (_server, url) = serve_llms("Test content for precision.\nAnother line.\n").await;

    let dir = tempdir().expect("tempdir");

    // Add test source
    add_source("precision-test", &url, dir.path(), dir.path());

    // Test with --show-raw-score flag (new flag to show raw scores)
    let mut cmd = cmd_with_dir(&dir);
    cmd.arg("search")
        .arg("test")
        .arg("--source")
        .arg("precision-test")
        .arg("--format")
        .arg("text")
        .arg("--show")
        .arg("raw-score")
        .assert()
        .success()
        .stdout(predicate::str::contains("Score")); // Should show raw score when requested
}

#[tokio::test]
async fn test_search_json_includes_percentage() {
    // Create test content
    let (_server, url) = serve_llms("Test content.\n").await;

    let dir = tempdir().expect("tempdir");

    // Add test source
    add_source("json-percent", &url, dir.path(), dir.path());

    // Search with JSON output
    let mut cmd = cmd_with_dir(&dir);
    let result = cmd
        .arg("search")
        .arg("test")
        .arg("--source")
        .arg("json-percent")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&result.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    // Check meta and percentage field
    assert!(json.get("query").is_some(), "json should include 'query'");
    assert!(
        json.get("total_hits").is_some(),
        "json should include 'total_hits'"
    );
    assert!(
        json.get("execution_time_ms").is_some(),
        "json should include 'execution_time_ms'"
    );
    if let Some(results) = json["results"].as_array() {
        if !results.is_empty() {
            assert!(
                results[0]["scorePercentage"].is_number(),
                "Should include scorePercentage field"
            );
            let pct = results[0]["scorePercentage"].as_f64().unwrap();
            assert!(
                (0.0..=100.0).contains(&pct),
                "scorePercentage must be within [0, 100]"
            );
            assert!(
                results[0]["score"].is_number(),
                "Raw score should be present"
            );
        }
    }
}

#[tokio::test]
async fn test_search_json_includes_raw_score_when_requested() {
    // Create test content
    let (_server, url) = serve_llms("Test content.\n").await;

    let dir = tempdir().expect("tempdir");

    add_source("json-raw-score", &url, dir.path(), dir.path());

    let mut cmd = cmd_with_dir(&dir);
    let result = cmd
        .arg("search")
        .arg("test")
        .arg("--source")
        .arg("json-raw-score")
        .arg("--format")
        .arg("json")
        .arg("--show")
        .arg("raw-score")
        .arg("--score-precision")
        .arg("0")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&result.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");

    let results = json["results"].as_array().expect("Expected results array");
    assert!(!results.is_empty(), "Expected at least one result");
    let score = results[0]["score"]
        .as_f64()
        .expect("Raw score should be included");
    assert!(
        (score - score.round()).abs() < f64::EPSILON,
        "Score should respect precision"
    );
}
