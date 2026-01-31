#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd_with_dirs;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn write_fixture(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, contents).expect("failed to write fixture");
    path
}

fn run_query_command(data_dir: &Path, config_dir: &Path, args: &[&str]) -> Value {
    let output = blz_cmd_with_dirs(data_dir, config_dir)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("valid json output")
}

#[tokio::test]
async fn test_source_flag_ordering_variations() {
    let data_dir = tempdir().expect("data dir");
    let config_dir = tempdir().expect("config dir");

    let doc = r"# Test Documentation

## Section A
This is a test document with sample content for testing flag ordering.
The target keyword appears here for searching.

## Section B
More content with different keywords and phrases.
Another test line with various words.
";

    let _local = write_fixture(data_dir.path(), "test.md", doc);

    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(
            ResponseTemplate::new(200).insert_header("content-length", doc.len().to_string()),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(doc))
        .mount(&server)
        .await;

    // Add test source
    blz_cmd_with_dirs(data_dir.path(), config_dir.path())
        .args([
            "add",
            "test",
            format!("{}/llms.txt", server.uri()).as_str(),
            "-y",
        ])
        .assert()
        .success();

    // Test 1: flags after positional arguments (original working pattern)
    let payload1 = run_query_command(
        data_dir.path(),
        config_dir.path(),
        &[
            "query",
            "target",
            "--source",
            "test",
            "--format",
            "json",
        ],
    );
    assert!(payload1["results"].as_array().is_some_and(|a| !a.is_empty()));

    // Test 2: flags before positional arguments (fixed pattern)
    let payload2 = run_query_command(
        data_dir.path(),
        config_dir.path(),
        &[
            "query",
            "--source",
            "test",
            "target", 
            "--format",
            "json",
        ],
    );
    assert!(payload2["results"].as_array().is_some_and(|a| !a.is_empty()));

    // Test 3: short flag before positional arguments
    let payload3 = run_query_command(
        data_dir.path(),
        config_dir.path(),
        &[
            "query",
            "-s",
            "test",
            "target",
            "--format", 
            "json",
        ],
    );
    assert!(payload3["results"].as_array().is_some_and(|a| !a.is_empty()));

    // Test 4: short flag after positional arguments
    let payload4 = run_query_command(
        data_dir.path(),
        config_dir.path(),
        &[
            "query",
            "target",
            "-s",
            "test",
            "--format",
            "json",
        ],
    );
    assert!(payload4["results"].as_array().is_some_and(|a| !a.is_empty()));

    // Test 5: mixed flag ordering with comma-separated sources
    let payload5 = run_query_command(
        data_dir.path(),
        config_dir.path(),
        &[
            "query",
            "--source",
            "test,test",  // comma-separated (though redundant here)
            "target",
            "--format",
            "json",
        ],
    );
    assert!(payload5["results"].as_array().is_some_and(|a| !a.is_empty()));

    // Test 6: repeated flags before positional arguments
    let payload6 = run_query_command(
        data_dir.path(),
        config_dir.path(),
        &[
            "query",
            "-s",
            "test",
            "-s", 
            "test",  // repeated flag (though redundant here)
            "target",
            "--format",
            "json",
        ],
    );
    assert!(payload6["results"].as_array().is_some_and(|a| !a.is_empty()));

    // Verify all results are equivalent (same search, different flag ordering)
    let results1 = &payload1["results"];
    let results2 = &payload2["results"];
    let results3 = &payload3["results"];
    let results4 = &payload4["results"];
    let results5 = &payload5["results"];
    let results6 = &payload6["results"];

    // All should find the same results
    assert_eq!(results1, results2, "Long flag before/after should be equivalent");
    assert_eq!(results1, results3, "Short flag before should be equivalent");  
    assert_eq!(results1, results4, "Short flag after should be equivalent");
    assert_eq!(results1, results5, "Comma-separated sources should be equivalent");
    assert_eq!(results1, results6, "Repeated flags should be equivalent");
}

#[tokio::test]
async fn test_search_command_flag_ordering() {
    let data_dir = tempdir().expect("data dir");
    let config_dir = tempdir().expect("config dir");

    let doc = r"# Search Test

## Section One
Search target content for testing.
More content here.
";

    let _local = write_fixture(data_dir.path(), "search_test.md", doc);

    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(
            ResponseTemplate::new(200).insert_header("content-length", doc.len().to_string()),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(doc))
        .mount(&server)
        .await;

    // Add test source
    blz_cmd_with_dirs(data_dir.path(), config_dir.path())
        .args([
            "add",
            "searchtest",
            format!("{}/llms.txt", server.uri()).as_str(),
            "-y",
        ])
        .assert()
        .success();

    // Test deprecated search command with flag ordering (still needs to work)
    let payload1 = run_query_command(
        data_dir.path(),
        config_dir.path(),
        &[
            "search",
            "target",
            "--source",
            "searchtest",
            "--format",
            "json",
        ],
    );
    assert!(payload1["results"].as_array().is_some_and(|a| !a.is_empty()));

    let payload2 = run_query_command(
        data_dir.path(),
        config_dir.path(),
        &[
            "search",
            "--source",
            "searchtest",
            "target",
            "--format",
            "json",
        ],
    );
    assert!(payload2["results"].as_array().is_some_and(|a| !a.is_empty()));

    // Results should be equivalent
    assert_eq!(
        payload1["results"], 
        payload2["results"], 
        "Search command flag ordering should not affect results"
    );
}