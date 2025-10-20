#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test that a single line reference with --context all expands to the full heading block
#[tokio::test]
async fn single_line_with_context_all_expands_to_full_block() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    // Document with clear heading structure (must have proper spacing for TOC parsing)
    let doc = "\
# Main Title

## Second Section

Target line
Another line
Final line

## Third Section

Other content
";

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

    blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "testdoc", &url, "-y"])
        .assert()
        .success();

    // Request a single line (line 5: "Target line") with --context all
    // Should expand to the entire "Second Section" block (lines 3-8)
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["get", "testdoc:5", "--context", "all", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output)?;
    let request = json["requests"]
        .as_array()
        .expect("requests array")
        .first()
        .expect("request entry");

    let line_start = request["lineStart"].as_u64().expect("lineStart present");
    let line_end = request["lineEnd"].as_u64().expect("lineEnd present");
    assert!(
        line_end > line_start,
        "Expected multiple lines for block expansion, got range {line_start}-{line_end}"
    );

    let content = request["snippet"].as_str().expect("snippet string");
    assert!(content.contains("Target line"), "Missing target line");
    assert!(content.contains("Another line"), "Missing section content");
    assert!(content.contains("Final line"), "Missing section content");

    assert!(
        (line_end - line_start + 1) >= 3,
        "Block expansion should return at least 3 lines, got range {line_start}-{line_end}"
    );

    Ok(())
}

/// Test that a range with --context all respects the requested span when no headings exist
#[tokio::test]
async fn range_with_context_all_without_headings_respects_range() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/plain.txt", server.uri());

    let doc = "Prelude\nLine 1\nLine 2\nLine 3\nLine 4\nLine 5\n";

    Mock::given(method("HEAD"))
        .and(path("/plain.txt"))
        .respond_with(
            ResponseTemplate::new(200).insert_header("content-length", doc.len().to_string()),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/plain.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(doc))
        .mount(&server)
        .await;

    blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "plain", &url, "-y"])
        .assert()
        .success();

    let output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["get", "plain:3-4", "--context", "all", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output)?;
    let request = json["requests"]
        .as_array()
        .expect("requests array")
        .first()
        .expect("request entry");

    let line_start = request["lineStart"].as_u64().expect("lineStart present");
    let line_end = request["lineEnd"].as_u64().expect("lineEnd present");
    assert_eq!(line_start, 3, "Expected block to start at requested range");
    assert_eq!(line_end, 4, "Expected block to end at requested range");

    let content = request["snippet"].as_str().expect("snippet string");
    assert!(content.contains("Line 3"), "Missing requested content");
    assert!(
        !content.contains("Line 5"),
        "Should not include lines beyond requested span"
    );

    Ok(())
}

/// Test that legacy --block flag works identically to --context all for single lines
#[tokio::test]
async fn legacy_block_flag_works_like_context_all() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    let doc = "# Document

## Section A
Line 1
Line 2
Line 3

## Section B
Next section
";

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

    blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "legacy", &url, "-y"])
        .assert()
        .success();

    // Get output with --context all
    let context_all_output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["get", "legacy:4", "--context", "all", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Get output with --block (legacy flag)
    let block_output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["get", "legacy:4", "--block", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let context_all_json: Value = serde_json::from_slice(&context_all_output)?;
    let block_json: Value = serde_json::from_slice(&block_output)?;

    // Both should produce identical output
    let context_request = context_all_json["requests"]
        .as_array()
        .and_then(|arr| arr.first())
        .expect("context request");
    let block_request = block_json["requests"]
        .as_array()
        .and_then(|arr| arr.first())
        .expect("block request");

    assert_eq!(
        context_request["lineStart"], block_request["lineStart"],
        "lineStart should match between --context all and --block"
    );
    assert_eq!(
        context_request["lineEnd"], block_request["lineEnd"],
        "lineEnd should match between --context all and --block"
    );
    assert_eq!(
        context_request["snippet"], block_request["snippet"],
        "Snippet content should match between --context all and --block"
    );

    Ok(())
}

/// Test that a single line WITHOUT --context all returns only that line
#[tokio::test]
async fn single_line_without_context_all_returns_single_line() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    let doc = "# Title

## Section
Line 1
Target line
Line 3
";

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

    blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "single", &url, "-y"])
        .assert()
        .success();

    // Request single line (line 5: "Target line") WITHOUT --context all
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["get", "single:5", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output)?;
    let request = json["requests"]
        .as_array()
        .and_then(|arr| arr.first())
        .expect("request entry");

    if let Some(ranges) = request["ranges"].as_array() {
        assert_eq!(ranges.len(), 1, "Expected single range for one line");
        let range = &ranges[0];
        assert_eq!(range["lineStart"].as_u64(), Some(5));
        assert_eq!(range["lineEnd"].as_u64(), Some(5));
        let snippet = range["snippet"].as_str().expect("snippet string");
        assert_eq!(snippet.trim(), "Target line");
    } else {
        assert_eq!(request["lineStart"].as_u64(), Some(5));
        assert_eq!(request["lineEnd"].as_u64(), Some(5));
        let snippet = request["snippet"].as_str().expect("snippet string");
        assert_eq!(snippet.trim(), "Target line");
    }

    Ok(())
}

/// Test that a range with --context all continues to work correctly
#[tokio::test]
async fn range_with_context_all_still_works() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    let doc = "# Main

## First Section
Line 1
Line 2
Line 3
Line 4

## Second Section
Line 5
";

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

    blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "rangedoc", &url, "-y"])
        .assert()
        .success();

    // Request a range (lines 4-5) with --context all
    // Should expand to include the full section
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["get", "rangedoc:4-5", "--context", "all", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output)?;
    let request = json["requests"]
        .as_array()
        .and_then(|arr| arr.first())
        .expect("request entry");
    let line_start = request["lineStart"].as_u64().expect("lineStart present");
    let line_end = request["lineEnd"].as_u64().expect("lineEnd present");
    assert!(
        (line_end - line_start + 1) >= 4,
        "Expected at least 4 lines for range with context all, got {line_start}-{line_end}"
    );

    let content = request["snippet"].as_str().expect("snippet string");
    assert!(content.contains("Line 1"), "Missing section start");
    assert!(content.contains("Line 2"), "Missing section content");
    assert!(content.contains("Line 3"), "Missing section content");
    assert!(content.contains("Line 4"), "Missing section content");

    Ok(())
}

/// Test edge case: single line at the start of a heading
#[tokio::test]
async fn single_line_on_heading_with_context_all() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    let doc = "# Title

## Section One
Content 1
Content 2

## Section Two
More content
";

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

    blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "heading", &url, "-y"])
        .assert()
        .success();

    // Request the heading line itself (line 3: "## Section One") with --context all
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["get", "heading:3", "--context", "all", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output)?;
    let request = json["requests"]
        .as_array()
        .and_then(|arr| arr.first())
        .expect("request entry");
    let line_start = request["lineStart"].as_u64().expect("lineStart");
    let line_end = request["lineEnd"].as_u64().expect("lineEnd");
    assert!(
        (line_end - line_start + 1) >= 2,
        "Expected multiple lines including section content"
    );

    let content = request["snippet"].as_str().expect("snippet");
    assert!(content.contains("Content 1"), "Missing section content");
    assert!(content.contains("Content 2"), "Missing section content");

    Ok(())
}

/// Test that --context N (numeric) still works correctly for single lines
#[tokio::test]
async fn single_line_with_numeric_context_adds_surrounding_lines() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    let doc = "Line 1
Line 2
Line 3
Target line
Line 5
Line 6
Line 7
";

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

    blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "ctx", &url, "-y"])
        .assert()
        .success();

    // Request line 4 with context of 2 lines before and after
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["get", "ctx:4", "--context", "2", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output)?;
    let request = json["requests"]
        .as_array()
        .and_then(|arr| arr.first())
        .expect("request entry");

    let (line_start, line_end, content) = request["ranges"].as_array().map_or_else(
        || {
            (
                request["lineStart"].as_u64().expect("lineStart"),
                request["lineEnd"].as_u64().expect("lineEnd"),
                request["snippet"].as_str().expect("snippet").to_owned(),
            )
        },
        |ranges| {
            assert_eq!(
                ranges.len(),
                1,
                "Expected single range with symmetric context"
            );
            let range = &ranges[0];
            (
                range["lineStart"].as_u64().expect("lineStart"),
                range["lineEnd"].as_u64().expect("lineEnd"),
                range["snippet"].as_str().expect("snippet").to_owned(),
            )
        },
    );

    assert_eq!(line_start, 2);
    assert_eq!(line_end, 6);

    assert!(content.contains("Line 2"), "Missing context before");
    assert!(content.contains("Line 3"), "Missing context before");
    assert!(content.contains("Target line"), "Missing target line");
    assert!(content.contains("Line 5"), "Missing context after");
    assert!(content.contains("Line 6"), "Missing context after");

    assert!(!content.contains("Line 1"), "Should not include line 1");
    assert!(!content.contains("Line 7"), "Should not include line 7");

    Ok(())
}
