#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for the --next flag functionality in search

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
async fn test_search_next_flag_basic() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");
    let (_server, url) = serve_paged_content().await;
    add_source("test-next-basic", &url, data_dir.path(), config_dir.path());

    // First search - page 1
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("search")
        .arg("content")
        .arg("--source")
        .arg("test-next-basic")
        .arg("--limit")
        .arg("5")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&output).expect("Should be valid JSON");

    assert_eq!(json["page"].as_u64(), Some(1));
    assert_eq!(json["limit"].as_u64(), Some(5));
    let first_page_results = json["results"].as_array().expect("results array").len();
    assert!(first_page_results > 0 && first_page_results <= 5);

    // Second search using --next
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

    // Should be on page 2 now
    assert_eq!(json["page"].as_u64(), Some(2));
    assert_eq!(json["query"].as_str(), Some("content")); // Same query as before
    assert_eq!(json["limit"].as_u64(), Some(5));
}

#[tokio::test]
async fn test_search_next_without_prior_search() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("--next")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No previous search found"));
}

#[tokio::test]
async fn test_search_next_respects_source_filter() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");
    let (_server, url) = serve_paged_content().await;
    add_source("test-next-filter", &url, data_dir.path(), config_dir.path());

    // First search with source filter
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("test")
        .arg("--source")
        .arg("test-next-filter")
        .arg("--limit")
        .arg("3")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    // Use --next, should maintain the source filter
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

    if let Some(results) = json["results"].as_array() {
        for result in results {
            assert_eq!(result["alias"].as_str(), Some("test-next-filter"));
        }
    }
}

#[tokio::test]
async fn test_search_next_at_last_page() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");
    let (_server, url) = serve_paged_content().await;
    add_source("test-next-last", &url, data_dir.path(), config_dir.path());

    // Jump to last page first
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    let result = cmd
        .arg("search")
        .arg("content")
        .arg("--source")
        .arg("test-next-last")
        .arg("--last")
        .arg("--limit")
        .arg("5")
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8_lossy(&result.get_output().stdout);
    let json: Value = serde_json::from_str(&output).expect("Should be valid JSON");
    let last_page = json["page"].as_u64().unwrap();
    let total_pages = json["totalPages"].as_u64().unwrap();
    assert_eq!(last_page, total_pages);

    // Try --next at the last page
    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("--next")
        .arg("--format")
        .arg("json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Already at the last page"));
}

#[tokio::test]
async fn test_search_next_conflicts_with_page() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("test")
        .arg("--next")
        .arg("--page")
        .arg("2")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[tokio::test]
async fn test_search_next_conflicts_with_last() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("test")
        .arg("--next")
        .arg("--last")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[tokio::test]
async fn test_search_last_conflicts_with_page() {
    let data_dir = tempdir().expect("temp data dir");
    let config_dir = tempdir().expect("temp config dir");

    let mut cmd = blz_cmd_with_dirs(data_dir.path(), config_dir.path());
    cmd.arg("search")
        .arg("test")
        .arg("--last")
        .arg("--page")
        .arg("2")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
