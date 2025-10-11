#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for the --previous flag functionality in search

mod common;

use common::{add_source, blz_cmd_with_dirs};
use predicates::prelude::*;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn serve_paged_content() -> (MockServer, String) {
    let server = MockServer::start().await;
    let body = (1..=100)
        .map(|i| format!("# Heading {i}\n\nThis is test content line {i} for pagination.\n"))
        .collect::<Vec<_>>()
        .join("\n");
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;
    let url = format!("{}/llms.txt", server.uri());
    (server, url)
}

#[tokio::test]
async fn test_search_previous_flag_basic() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");
    let (_server, url) = serve_paged_content().await;
    add_source(
        "test-previous-basic",
        &url,
        data_dir.path(),
        config_dir.path(),
    );

    // First search - page 1
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("content")
        .arg("--source")
        .arg("test-previous-basic")
        .arg("--limit")
        .arg("5")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    // Move to page 2 with --next
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("search")
        .arg("--next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&output).expect("Should be valid JSON");
    assert_eq!(json["page"].as_u64(), Some(2));

    // Go back to page 1 using --previous
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("search")
        .arg("--previous")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&output).expect("Should be valid JSON");

    // Should be back on page 1
    assert_eq!(json["page"].as_u64(), Some(1));
    assert_eq!(json["query"].as_str(), Some("content")); // Same query as before
    assert_eq!(json["limit"].as_u64(), Some(5));
}

#[tokio::test]
async fn test_search_previous_without_prior_search() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("--previous")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No previous search found"));
}

#[tokio::test]
async fn test_search_previous_at_first_page() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");
    let (_server, url) = serve_paged_content().await;
    add_source(
        "test-previous-first",
        &url,
        data_dir.path(),
        config_dir.path(),
    );

    // First search - page 1
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("content")
        .arg("--source")
        .arg("test-previous-first")
        .arg("--limit")
        .arg("5")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    // Try --previous when already on page 1
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("--previous")
        .arg("--format")
        .arg("json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Already on first page"));
}

#[tokio::test]
async fn test_search_previous_respects_source_filter() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");
    let (_server, url) = serve_paged_content().await;
    add_source(
        "test-previous-filter",
        &url,
        data_dir.path(),
        config_dir.path(),
    );

    // First search with source filter
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("test")
        .arg("--source")
        .arg("test-previous-filter")
        .arg("--limit")
        .arg("3")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    // Move to page 2
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("--next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    // Use --previous, should maintain the source filter
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("search")
        .arg("--previous")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&output).expect("Should be valid JSON");

    if let Some(results) = json["results"].as_array() {
        for result in results {
            assert_eq!(result["alias"].as_str(), Some("test-previous-filter"));
        }
    }
}

#[tokio::test]
async fn test_search_previous_conflicts_with_page() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("test")
        .arg("--previous")
        .arg("--page")
        .arg("2")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[tokio::test]
async fn test_search_previous_conflicts_with_next() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("test")
        .arg("--previous")
        .arg("--next")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[tokio::test]
async fn test_search_previous_conflicts_with_last() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("test")
        .arg("--previous")
        .arg("--last")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[tokio::test]
async fn test_search_navigation_forward_and_back() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");
    let (_server, url) = serve_paged_content().await;
    add_source("test-nav-both", &url, data_dir.path(), config_dir.path());

    // Start on page 1
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("search")
        .arg("content")
        .arg("--source")
        .arg("test-nav-both")
        .arg("--limit")
        .arg("5")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&output).expect("Should be valid JSON");
    assert_eq!(json["page"].as_u64(), Some(1));

    // Go to page 2 with --next
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("search")
        .arg("--next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&output).expect("Should be valid JSON");
    assert_eq!(json["page"].as_u64(), Some(2));

    // Go to page 3 with --next
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("search")
        .arg("--next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&output).expect("Should be valid JSON");
    assert_eq!(json["page"].as_u64(), Some(3));

    // Go back to page 2 with --previous
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("search")
        .arg("--previous")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&output).expect("Should be valid JSON");
    assert_eq!(json["page"].as_u64(), Some(2));

    // Go back to page 1 with --previous
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("search")
        .arg("--previous")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&output).expect("Should be valid JSON");
    assert_eq!(json["page"].as_u64(), Some(1));
}

#[tokio::test]
async fn test_search_previous_with_explicit_query_fails() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");
    let (_server, url) = serve_paged_content().await;
    add_source(
        "test-previous-query",
        &url,
        data_dir.path(),
        config_dir.path(),
    );

    // First search
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("content")
        .arg("--source")
        .arg("test-previous-query")
        .arg("--limit")
        .arg("5")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    // Move to page 2
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("--next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    // Try --previous with explicit query (should fail)
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("different query")
        .arg("--previous")
        .arg("--format")
        .arg("json")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Cannot combine --previous with an explicit query",
        ));
}

#[tokio::test]
async fn test_search_previous_with_explicit_source_fails() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");
    let (_server, url) = serve_paged_content().await;
    add_source(
        "test-previous-source",
        &url,
        data_dir.path(),
        config_dir.path(),
    );

    // First search
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("content")
        .arg("--source")
        .arg("test-previous-source")
        .arg("--limit")
        .arg("5")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    // Move to page 2
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("--next")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    // Try --previous with explicit source (should fail)
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("--previous")
        .arg("--source")
        .arg("test-previous-source")
        .arg("--format")
        .arg("json")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Cannot combine --previous with --source",
        ));
}
