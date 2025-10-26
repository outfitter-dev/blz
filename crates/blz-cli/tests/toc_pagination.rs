#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::needless_raw_string_hashes,
    clippy::items_after_statements,
    clippy::uninlined_format_args
)]

//! Integration tests for TOC pagination feature (BLZ-250)
//!
//! Tests the following pagination flags:
//! - --limit N: Show N results per page
//! - --page N: Jump to specific page
//! - --next: Navigate to next page
//! - --previous: Navigate to previous page
//! - --last: Jump to last page
//! - --all: Override limit and show all results

mod common;

use std::convert::TryFrom;

use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to count entries from JSON array (for pagination tests)
const fn count_all_entries(entries: &[Value]) -> usize {
    entries.len()
}

/// Sample document with 25 headings for comprehensive pagination testing
const SAMPLE_DOC: &str = r#"# Section 1
Content for section 1

## Subsection 1.1
Details for subsection 1.1

## Subsection 1.2
Details for subsection 1.2

# Section 2
Content for section 2

## Subsection 2.1
Details for subsection 2.1

## Subsection 2.2
Details for subsection 2.2

# Section 3
Content for section 3

## Subsection 3.1
Details for subsection 3.1

## Subsection 3.2
Details for subsection 3.2

# Section 4
Content for section 4

## Subsection 4.1
Details for subsection 4.1

## Subsection 4.2
Details for subsection 4.2

# Section 5
Content for section 5

## Subsection 5.1
Details for subsection 5.1

## Subsection 5.2
Details for subsection 5.2

# Section 6
Content for section 6

## Subsection 6.1
Details for subsection 6.1

## Subsection 6.2
Details for subsection 6.2

# Section 7
Content for section 7

## Subsection 7.1
Details for subsection 7.1

## Subsection 7.2
Details for subsection 7.2

# Section 8
Content for section 8
"#;

/// Helper: Seed test source with sample document
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
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["add", alias, url.as_str(), "-y"])
        .assert()
        .success();

    Ok(())
}

/// Test 1: Basic pagination with limit
#[tokio::test]
async fn test_toc_pagination_basic() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Run toc with limit of 5
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--limit", "5", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output)?;

    // JSON output is now always an object with pagination metadata
    assert!(json.is_object(), "output should be an object");

    let entries = json["entries"]
        .as_array()
        .expect("output should have 'entries' array");

    // With a limit of 5, we should get exactly 5 entries
    assert_eq!(
        entries.len(),
        5,
        "Should return exactly 5 entries with --limit 5"
    );

    // Verify pagination metadata
    assert_eq!(json["page"].as_u64(), Some(1), "Should be on page 1");
    assert_eq!(json["page_size"].as_u64(), Some(5), "Page size should be 5");
    assert!(
        json["total_pages"].as_u64().unwrap() >= 1,
        "Should have at least 1 page"
    );
    assert!(
        json["total_results"].as_u64().unwrap() >= 5,
        "Should have at least 5 results"
    );

    // Verify entries have expected structure
    for entry in entries {
        assert!(entry["alias"].is_string(), "Each entry should have alias");
        assert!(
            entry["headingPath"].is_array(),
            "Each entry should have headingPath"
        );
        assert!(entry["lines"].is_string(), "Each entry should have lines");
    }

    Ok(())
}

/// Test 2: Navigate to next page
#[tokio::test]
async fn test_toc_pagination_next() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // First, establish page 1
    let output1 = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--limit", "5", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json1: Value = serde_json::from_slice(&output1)?;
    let entries1 = json1["entries"]
        .as_array()
        .expect("output should have entries array");

    // Get first entry's lines from page 1
    let first_lines_page1 = entries1[0]["lines"].as_str().expect("should have lines");

    // Now navigate to next page
    let output2 = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--next", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json2: Value = serde_json::from_slice(&output2)?;
    let entries2 = json2["entries"]
        .as_array()
        .expect("output should have entries array");

    // Verify page number changed
    assert_eq!(json2["page"].as_u64(), Some(2), "Should be on page 2");

    // Verify results are different (page 2 should have different entries)
    if !entries2.is_empty() {
        let first_lines_page2 = entries2[0]["lines"].as_str().expect("should have lines");
        assert_ne!(
            first_lines_page1, first_lines_page2,
            "Page 2 should have different results than page 1"
        );
    }

    Ok(())
}

/// Test 3: Navigate to previous page
#[tokio::test]
async fn test_toc_pagination_previous() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Start at page 1 and record first entry
    let output1 = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--limit", "5", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json1: Value = serde_json::from_slice(&output1)?;
    let entries1 = json1["entries"]
        .as_array()
        .expect("output should have entries array");
    let first_lines_page1 = entries1[0]["lines"].as_str().expect("should have lines");

    // Go to page 2
    blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--next", "-f", "json"])
        .assert()
        .success();

    // Now go back to previous (page 1)
    let output_prev = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--previous", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_prev: Value = serde_json::from_slice(&output_prev)?;
    let entries_prev = json_prev["entries"]
        .as_array()
        .expect("output should have entries array");

    // Verify we're back on page 1
    assert_eq!(
        json_prev["page"].as_u64(),
        Some(1),
        "Should be back on page 1"
    );

    // Should have same entries as original page 1
    let first_lines_after_prev = entries_prev[0]["lines"]
        .as_str()
        .expect("should have lines");
    assert_eq!(
        first_lines_page1, first_lines_after_prev,
        "Should return to same entries as page 1 after --previous"
    );

    Ok(())
}

/// Test 4: Jump to last page
#[tokio::test]
async fn test_toc_pagination_last() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // First, get all entries to know total count
    let output_all = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--all", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_all: Value = serde_json::from_slice(&output_all)?;
    let total_count = json_all["total_results"]
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or_else(|| {
            let all_entries = json_all["entries"]
                .as_array()
                .expect("entries should be an array");
            count_all_entries(all_entries)
        });

    // Jump directly to last page with limit of 5
    let output_last = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--limit", "5", "--last", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_last: Value = serde_json::from_slice(&output_last)?;
    let entries_last = json_last["entries"]
        .as_array()
        .expect("output should have entries array");

    // The last page should have entries
    assert!(
        !entries_last.is_empty(),
        "Last page should have at least one entry"
    );

    // Verify we're on the last page
    let last_page_num = json_last["page"]
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or(1);
    let total_pages = json_last["total_pages"]
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or(1);
    assert_eq!(last_page_num, total_pages, "Should be on last page");

    // The last page might have fewer than 5 entries (if total isn't divisible by 5)
    let expected_on_last_page = if total_count % 5 == 0 {
        5
    } else {
        total_count % 5
    };
    assert_eq!(
        entries_last.len(),
        expected_on_last_page,
        "Last page should have {} entries (total: {}, limit: 5)",
        expected_on_last_page,
        total_count
    );

    Ok(())
}

/// Test 5: Jump to specific page
#[tokio::test]
async fn test_toc_pagination_page_jump() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Get page 1
    let output1 = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--limit", "5", "--page", "1", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json1: Value = serde_json::from_slice(&output1)?;
    let entries1 = json1["entries"]
        .as_array()
        .expect("output should have entries array");

    // Jump to page 3
    let output3 = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--limit", "5", "--page", "3", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json3: Value = serde_json::from_slice(&output3)?;
    let entries3 = json3["entries"]
        .as_array()
        .expect("output should have entries array");

    // Verify we're on page 3
    assert_eq!(json3["page"].as_u64(), Some(3), "Should be on page 3");

    // Page 3 should have entries (sample doc has enough headings)
    assert!(!entries3.is_empty(), "Page 3 should have entries");

    // Page 3 should have different entries than page 1
    let first_lines_page1 = entries1[0]["lines"].as_str().expect("should have lines");
    let first_lines_page3 = entries3[0]["lines"].as_str().expect("should have lines");
    assert_ne!(
        first_lines_page1, first_lines_page3,
        "Page 3 should have different entries than page 1"
    );

    Ok(())
}

/// Test 6: --all flag overrides limit
#[tokio::test]
async fn test_toc_pagination_all_overrides_limit() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // First, get limited results
    let output_limited = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--limit", "5", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_limited: Value = serde_json::from_slice(&output_limited)?;
    let entries_limited = json_limited["entries"]
        .as_array()
        .expect("output should have entries array");
    let limited_count = entries_limited.len();

    // Now get all results with --all (should override the limit)
    let output_all = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--limit", "5", "--all", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_all: Value = serde_json::from_slice(&output_all)?;
    let all_count = json_all["total_results"]
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or_else(|| {
            let entries_all = json_all["entries"]
                .as_array()
                .expect("output should have entries array");
            count_all_entries(entries_all)
        });

    // --all should return more results than the limited query
    assert!(
        all_count >= limited_count * 2,
        "--all should return significantly more entries than a single page (got {}, page size {})",
        all_count,
        limited_count
    );

    // Verify we got exactly 5 with limit
    assert_eq!(limited_count, 5, "Limited query should return 5 entries");

    Ok(())
}

/// Test 7: Default behavior (no limit) shows all results
#[tokio::test]
async fn test_toc_pagination_no_limit_shows_all() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    seed_source(&tmp, &server, "docs", SAMPLE_DOC).await?;

    // Get results without any limit flag
    let output_no_limit = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_no_limit: Value = serde_json::from_slice(&output_no_limit)?;
    let entries_no_limit = json_no_limit["entries"]
        .as_array()
        .expect("entries should be an array");
    let no_limit_count = entries_no_limit.len();

    // Get results with explicit --all flag
    let output_all = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["toc", "docs", "--all", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_all: Value = serde_json::from_slice(&output_all)?;
    let entries_all = json_all["entries"]
        .as_array()
        .expect("entries should be an array");
    let all_count = entries_all.len();

    // Both should return the same number of results
    assert_eq!(
        no_limit_count, all_count,
        "Default (no limit) should return same count as --all: {} vs {}",
        no_limit_count, all_count
    );

    Ok(())
}
