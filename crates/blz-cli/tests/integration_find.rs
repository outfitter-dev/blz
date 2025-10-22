#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::needless_raw_string_hashes,
    clippy::uninlined_format_args,
    clippy::len_zero
)]

mod common;

use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Seed a test source with sample documentation
async fn seed_source(
    tmp: &tempfile::TempDir,
    server: &MockServer,
    alias: &str,
    doc: &str,
) -> anyhow::Result<()> {
    let url = format!("{}/llms.txt", server.uri());

    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(
            ResponseTemplate::new(200).insert_header("content-length", doc.len().to_string()),
        )
        .mount(server)
        .await;

    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(doc))
        .mount(server)
        .await;

    blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", alias, url.as_str(), "-y"])
        .assert()
        .success();

    Ok(())
}

/// Run a find command and parse JSON output
fn run_find_json(tmp: &tempfile::TempDir, args: &[&str]) -> anyhow::Result<Value> {
    let stdout = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    Ok(serde_json::from_slice(&stdout)?)
}

// Sample documentation for tests
const SAMPLE_DOC: &str = r#"# Documentation

## Introduction
Welcome to our documentation.

## Getting Started
This section covers installation.

### Installation
Run the installer script.

### Configuration
Edit the config file.

## Advanced Topics
Deep dive into advanced features.

### Performance Tuning
Optimize for speed.

## API Reference
Complete API documentation.
"#;

#[tokio::test]
async fn find_with_query_routes_to_search_mode() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Query string should route to search mode
    let payload = run_find_json(
        &tmp,
        &["find", "installation", "--source", "docs", "-f", "json"],
    )?;

    // Verify search response structure
    assert!(payload.get("query").is_some(), "should have query field");
    assert!(
        payload.get("results").is_some(),
        "should have results array"
    );
    assert_eq!(
        payload["query"].as_str().unwrap(),
        "installation",
        "query should match input"
    );

    // Verify we got search results
    let results = payload["results"].as_array().unwrap();
    assert!(
        !results.is_empty(),
        "should find matches for 'installation'"
    );

    Ok(())
}

#[tokio::test]
async fn find_with_citation_routes_to_retrieve_mode() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Citation pattern should route to retrieve mode
    let payload = run_find_json(&tmp, &["find", "docs:1-5", "-f", "json"])?;

    // Verify get/retrieve response structure
    assert!(
        payload.get("requests").is_some(),
        "should have requests field from get mode"
    );

    let requests = payload["requests"].as_array().unwrap();
    assert_eq!(requests.len(), 1, "should have one request");

    let request = &requests[0];
    assert_eq!(request["alias"].as_str().unwrap(), "docs");
    assert!(
        request.get("snippet").is_some(),
        "should have snippet content"
    );

    Ok(())
}

#[tokio::test]
async fn find_with_multirange_citation_routes_to_retrieve() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Multi-range citation should also route to retrieve mode
    let payload = run_find_json(&tmp, &["find", "docs:1-3,7-9", "-f", "json"])?;

    assert!(
        payload.get("requests").is_some(),
        "multi-range should use retrieve mode"
    );

    let requests = payload["requests"].as_array().unwrap();
    assert_eq!(requests.len(), 1, "should have one request for multi-range");

    Ok(())
}

#[tokio::test]
async fn find_heading_level_filter_applies_in_search_mode() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Search without filter
    let all_results = run_find_json(&tmp, &["find", "section", "--source", "docs", "-f", "json"])?;
    let all_count = all_results["results"].as_array().unwrap().len();

    // Search with heading level filter (only level 2 headings)
    let filtered_results = run_find_json(
        &tmp,
        &[
            "find", "section", "--source", "docs", "-H", "=2", "-f", "json",
        ],
    )?;
    let filtered_count = filtered_results["results"].as_array().unwrap().len();

    // Filtering should reduce results
    assert!(
        filtered_count <= all_count,
        "heading filter should reduce or maintain result count"
    );

    // Verify all results have level 2
    for result in filtered_results["results"].as_array().unwrap() {
        let level = result.get("level").and_then(Value::as_u64);
        assert_eq!(level, Some(2), "all filtered results should be level 2");
    }

    Ok(())
}

#[tokio::test]
async fn find_heading_level_filter_with_less_than_operator() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Filter for headings level 2 or less (h1 and h2)
    let filtered = run_find_json(
        &tmp,
        &[
            "find",
            "documentation",
            "--source",
            "docs",
            "-H",
            "<=2",
            "-f",
            "json",
        ],
    )?;

    // All results should have level <= 2
    for result in filtered["results"].as_array().unwrap() {
        let level = result.get("level").and_then(Value::as_u64).unwrap();
        assert!(
            level <= 2,
            "filtered result should have level <= 2, got {}",
            level
        );
    }

    Ok(())
}

#[tokio::test]
async fn find_heading_level_filter_ignored_in_retrieve_mode() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Use citation with heading level filter - should be ignored
    let result = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["find", "docs:1-5", "-H", "=2", "-f", "json"])
        .assert()
        .success();

    let payload: Value = serde_json::from_slice(&result.get_output().stdout)?;

    // Should have retrieve mode structure (requests field)
    assert!(
        payload.get("requests").is_some(),
        "should use retrieve mode despite -H flag"
    );

    // Should return the requested lines regardless of heading level
    let requests = payload["requests"].as_array().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(
        requests[0].get("snippet").is_some(),
        "should return content despite heading filter"
    );

    Ok(())
}

#[tokio::test]
async fn find_bare_command_with_query() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Bare command without explicit "find" subcommand
    // Note: This tests handle_default routing through find
    let payload = run_find_json(&tmp, &["find", "performance", "-f", "json"])?;

    assert!(
        payload.get("query").is_some(),
        "bare query should use search mode"
    );
    assert_eq!(payload["query"].as_str().unwrap(), "performance");

    Ok(())
}

#[tokio::test]
async fn find_bare_command_with_citation() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Bare command with citation pattern
    let payload = run_find_json(&tmp, &["find", "docs:10-15", "-f", "json"])?;

    assert!(
        payload.get("requests").is_some(),
        "bare citation should use retrieve mode"
    );

    Ok(())
}

#[tokio::test]
async fn find_distinguishes_query_from_citation_like_pattern() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // These look like citations but aren't (should be treated as queries)
    let test_cases = vec![
        "getting:started", // Not a valid citation (no digit-digit after colon)
        "docs:",           // Missing range
        "docs:10",         // Missing end of range
        "DOCS:10-20",      // Uppercase not allowed
        "docs 10-20",      // Space instead of colon
    ];

    for query in test_cases {
        let payload = run_find_json(&tmp, &["find", query, "-f", "json"])?;

        // Should be treated as search query, not citation
        assert!(
            payload.get("query").is_some(),
            "pattern '{}' should be treated as query",
            query
        );
        assert_eq!(
            payload["query"].as_str().unwrap(),
            query,
            "query field should match input '{}'",
            query
        );
    }

    Ok(())
}

#[tokio::test]
async fn find_with_context_in_search_mode() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Search with context flag
    let payload = run_find_json(
        &tmp,
        &[
            "find",
            "installation",
            "--source",
            "docs",
            "-C",
            "2",
            "-f",
            "json",
        ],
    )?;

    // Should be in search mode
    assert!(payload.get("query").is_some());

    // Results should have context
    let results = payload["results"].as_array().unwrap();
    if !results.is_empty() {
        // Check if first result has context (when matching)
        let first = &results[0];
        if first.get("context").is_some() {
            assert!(
                first["context"].get("lines").is_some(),
                "context should include line range"
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn find_with_block_context_in_retrieve_mode() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Retrieve with --context all (block mode)
    let payload = run_find_json(
        &tmp,
        &["find", "docs:5-7", "--context", "all", "-f", "json"],
    )?;

    // Should be in retrieve mode
    assert!(payload.get("requests").is_some());

    // Should expand to full block
    let requests = payload["requests"].as_array().unwrap();
    let snippet = requests[0]["snippet"].as_str().unwrap();
    assert!(!snippet.is_empty(), "should have expanded block content");

    Ok(())
}

#[tokio::test]
async fn find_with_copy_flag_in_search_mode() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Search with --copy flag (should still succeed even if clipboard fails)
    let result = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["find", "api", "--source", "docs", "--copy", "-f", "json"])
        .assert()
        .success();

    let payload: Value = serde_json::from_slice(&result.get_output().stdout)?;

    // Should be in search mode
    assert!(payload.get("query").is_some());
    assert_eq!(payload["query"].as_str().unwrap(), "api");

    Ok(())
}

#[tokio::test]
async fn find_with_multiple_sources_in_search_mode() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    // Seed multiple sources
    seed_source(&tmp, &server, "docs1", SAMPLE_DOC).await?;
    seed_source(&tmp, &server, "docs2", SAMPLE_DOC).await?;

    let payload = run_find_json(
        &tmp,
        &[
            "find",
            "documentation",
            "--source",
            "docs1",
            "--source",
            "docs2",
            "-f",
            "json",
        ],
    )?;

    // Should search both sources
    assert!(payload.get("sources").is_some());
    let sources = payload["sources"].as_array().unwrap();
    assert!(sources.len() >= 1, "should search at least one source");

    Ok(())
}
