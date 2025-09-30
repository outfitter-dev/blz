#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;
use common::blz_cmd;

/// Test that add command fetches and indexes source content successfully
#[tokio::test]
async fn test_add_indexes_source_content() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    let doc =
        "# Documentation\n\n## Getting Started\nHere is some comprehensive documentation content.";

    // Setup mock to serve content
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

    let url = format!("{}/llms.txt", server.uri());
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "test-source", &url, "-y"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output)?;

    // Verify success message
    assert!(
        stdout.contains("Added") || stdout.contains("✓"),
        "expected add success message"
    );

    // Verify source directory and files exist
    let source_dir = data_dir.path().join("test-source");
    assert!(source_dir.exists(), "expected source directory to exist");

    // Verify content file exists
    let content_file = source_dir.join("llms.txt");
    assert!(content_file.exists(), "expected content file to exist");

    // Verify content was saved correctly
    let content = std::fs::read_to_string(content_file)?;
    assert!(
        content.contains("Getting Started"),
        "expected indexed content to match source"
    );

    Ok(())
}

/// Test that search finds content that was indexed during add
#[tokio::test]
async fn test_search_finds_indexed_content() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    let doc =
        "# Documentation\n\n## API Reference\nThe UNIQUE_MARKER function is used for testing.";

    // Setup mocks
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

    let url = format!("{}/llms.txt", server.uri());

    // Add source
    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "test-source", &url, "-y"])
        .assert()
        .success();

    // Search for content that should be indexed
    let search_output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["search", "UNIQUE_MARKER", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let search_results: Value = serde_json::from_slice(&search_output)?;

    // Search results are wrapped in an object with "results" field
    let hits = search_results
        .get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    assert!(
        !hits.is_empty(),
        "expected search to find results from indexed content"
    );

    // Verify the search result includes the unique marker
    let first_hit = &hits[0];
    let snippet = first_hit
        .get("snippet")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    assert!(
        snippet.contains("UNIQUE_MARKER"),
        "expected search result to contain indexed content, got snippet: {snippet}"
    );

    Ok(())
}

/// Test that list command shows added sources correctly
#[tokio::test]
async fn test_list_shows_added_sources() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    let doc = "# Documentation";

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

    let url = format!("{}/llms.txt", server.uri());

    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "test-source", &url, "-y"])
        .assert()
        .success();

    // List should show the added source
    let list_output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["list", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let list_value: Value = serde_json::from_slice(&list_output)?;
    let list_arr = list_value.as_array().cloned().unwrap_or_default();

    assert!(!list_arr.is_empty(), "expected list to contain source");

    let source = &list_arr[0];
    let alias = source.get("alias").and_then(|v| v.as_str()).unwrap_or("");

    assert_eq!(
        alias, "test-source",
        "expected correct alias in list output"
    );

    Ok(())
}

/// Test that sources with comprehensive content are searchable
#[tokio::test]
async fn test_comprehensive_content_is_searchable() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    // Create comprehensive documentation (>100 lines)
    let mut doc = String::from("# Comprehensive Documentation\n\n");
    for i in 1..=50 {
        doc.push_str(&format!(
            "## Section {}\n\nContent for section {}.\n\n",
            i, i
        ));
    }
    doc.push_str("## Special Section\n\nThis has a SEARCHABLE_TERM for testing.");

    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(&doc))
        .mount(&server)
        .await;

    let url = format!("{}/llms.txt", server.uri());

    // Add source
    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "comprehensive", &url, "-y"])
        .assert()
        .success();

    // Search should find content
    let search_output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["search", "SEARCHABLE_TERM", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let search_results: Value = serde_json::from_slice(&search_output)?;
    let hits = search_results
        .get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    assert!(
        !hits.is_empty(),
        "expected comprehensive content to be searchable"
    );

    Ok(())
}

/// Test that index-only sources (TOC) are handled correctly
#[tokio::test]
async fn test_index_only_source_handling() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    // Create a small TOC-only file
    let doc = "# Table of Contents\n\n- [Section 1](./section1)\n- [Section 2](./section2)";

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

    let url = format!("{}/llms.txt", server.uri());

    // Add source - should succeed but may be tagged as index-only
    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "toc-only", &url, "-y"])
        .assert()
        .success();

    // Verify source was added
    let source_dir = data_dir.path().join("toc-only");
    assert!(source_dir.exists(), "expected source directory to exist");

    let content_file = source_dir.join("llms.txt");
    assert!(content_file.exists(), "expected content file to exist");

    Ok(())
}
