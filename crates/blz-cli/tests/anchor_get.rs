#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "anchors")]

mod common;

use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn toc_entries(value: &Value) -> Vec<Value> {
    value
        .get("entries")
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .cloned()
        .unwrap_or_default()
}

#[tokio::test]
async fn anchor_get_returns_expected_section() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    let doc = "# Title\n\n## A\nalpha line\nsecond line\n\n## B\nbravo\n";
    // HEAD + GET
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

    // Add
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "e2e", &url, "-y"])
        .assert()
        .success();

    // Get TOC JSON
    let mut cmd = blz_cmd();
    let anchors_out = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["toc", "e2e", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Value = serde_json::from_slice(&anchors_out)?;
    let arr = toc_entries(&entries);
    assert!(!arr.is_empty(), "expected toc list");
    // Find anchor for heading A
    let anchor = arr
        .iter()
        .find(|e| {
            e.get("headingPath")
                .and_then(|hp| hp.as_array())
                .is_some_and(|hp| hp.last().and_then(|s| s.as_str()) == Some("A"))
        })
        .and_then(|e| e.get("anchor"))
        .and_then(|a| a.as_str())
        .unwrap_or("")
        .to_string();
    assert!(!anchor.is_empty(), "expected non-empty anchor for A");

    // Anchor get
    let mut cmd = blz_cmd();
    let get_out = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["anchor", "get", "e2e", &anchor, "--context", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(get_out)?;
    assert!(
        s.contains("alpha line"),
        "expected section content in output"
    );

    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn toc_limit_and_depth_flags() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    let doc = "# Title\n\n## A\nalpha line\n\n### A.1\nnested\n\n## B\nbravo\n";
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

    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "e2e", &url, "-y"])
        .assert()
        .success();

    // JSON output respects both limit and max depth
    let mut cmd = blz_cmd();
    let toc_json = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .args([
            "toc",
            "e2e",
            "--max-depth",
            "1",
            "--limit",
            "1",
            "-f",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Value = serde_json::from_slice(&toc_json)?;
    let arr = toc_entries(&entries);
    assert_eq!(arr.len(), 1, "expected only top-level heading with limit 1");
    assert!(
        arr.iter()
            .all(|e| e.get("headingLevel").and_then(Value::as_u64) == Some(1)),
        "expected headingLevel 1 entries only"
    );

    // Text output omits deeper headings when max depth is set
    let mut cmd = blz_cmd();
    let toc_text = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["toc", "e2e", "--max-depth", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(toc_text)?;
    assert!(
        !s.contains("A.1"),
        "expected nested heading to be omitted when max depth is 1"
    );
    assert!(
        s.contains("Title"),
        "expected top-level heading to remain visible"
    );

    // Filter expression restricts to matching headings
    let mut cmd = blz_cmd();
    let filter_json = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["toc", "e2e", "--filter", "A.1", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let filtered: Value = serde_json::from_slice(&filter_json)?;
    let filtered_arr = toc_entries(&filtered);
    assert_eq!(
        filtered_arr.len(),
        1,
        "expected only A.1 heading to match filter"
    );
    assert_eq!(
        filtered_arr[0]
            .get("headingPath")
            .and_then(|hp| hp.as_array())
            .and_then(|hp| hp.last())
            .and_then(|last| last.as_str()),
        Some("A.1")
    );

    let mut cmd = blz_cmd();
    let filter_text = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["toc", "e2e", "--filter", "NOT B"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(filter_text)?;
    assert!(
        !s.contains('B'),
        "expected filter to exclude headings containing 'B'"
    );

    Ok(())
}
