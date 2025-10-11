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

    // Should get the full section, not just the single line
    let line_numbers = json["lineNumbers"].as_array().unwrap();
    assert!(
        line_numbers.len() > 1,
        "Expected multiple lines for block expansion, got {}",
        line_numbers.len()
    );

    // Content should include the target line and surrounding section content
    let content = json["content"].as_str().unwrap();
    assert!(content.contains("Target line"), "Missing target line");
    assert!(content.contains("Another line"), "Missing section content");
    assert!(content.contains("Final line"), "Missing section content");

    // The key assertion: single line WITH --context all should expand to full block
    // This verifies the BLZ-115 fix
    assert!(
        line_numbers.len() >= 3,
        "Block expansion should return at least 3 lines, got {}",
        line_numbers.len()
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
    let line_numbers = json["lineNumbers"].as_array().unwrap();
    let collected: Vec<_> = line_numbers
        .iter()
        .map(|value| usize::try_from(value.as_u64().unwrap()).expect("line numbers fit usize"))
        .collect();
    assert_eq!(
        collected,
        vec![3, 4],
        "Expected context all to respect range end"
    );

    let content = json["content"].as_str().unwrap();
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
    assert_eq!(
        context_all_json["lineNumbers"], block_json["lineNumbers"],
        "Line numbers should match between --context all and --block"
    );
    assert_eq!(
        context_all_json["content"], block_json["content"],
        "Content should match between --context all and --block"
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

    // Should get exactly one line
    let line_numbers = json["lineNumbers"].as_array().unwrap();
    assert_eq!(
        line_numbers.len(),
        1,
        "Expected exactly one line without context expansion"
    );
    assert_eq!(
        line_numbers[0].as_u64(),
        Some(5),
        "Should return the requested line number"
    );

    // Content should only be the single line
    let content = json["content"].as_str().unwrap();
    assert_eq!(
        content.trim(),
        "Target line",
        "Should only contain the target line"
    );

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

    // Should get the full section including the heading
    let line_numbers = json["lineNumbers"].as_array().unwrap();
    assert!(
        line_numbers.len() >= 4,
        "Expected at least 4 lines for range with context all"
    );

    let content = json["content"].as_str().unwrap();
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

    // Should expand to include the entire section
    let line_numbers = json["lineNumbers"].as_array().unwrap();
    assert!(
        line_numbers.len() >= 2,
        "Expected multiple lines including section content"
    );

    let content = json["content"].as_str().unwrap();
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

    // Should get line 4 plus 2 lines before and 2 after (lines 2-6)
    let line_numbers = json["lineNumbers"].as_array().unwrap();
    assert_eq!(line_numbers.len(), 5, "Expected 5 lines with context 2");

    let content = json["content"].as_str().unwrap();
    assert!(content.contains("Line 2"), "Missing context before");
    assert!(content.contains("Line 3"), "Missing context before");
    assert!(content.contains("Target line"), "Missing target line");
    assert!(content.contains("Line 5"), "Missing context after");
    assert!(content.contains("Line 6"), "Missing context after");

    // Should NOT include line 1 or line 7
    assert!(!content.contains("Line 1"), "Should not include line 1");
    assert!(!content.contains("Line 7"), "Should not include line 7");

    Ok(())
}
