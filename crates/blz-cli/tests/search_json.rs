#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn search_json_schema_contains_expected_fields() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    let doc = "# Title\n\n## A\nalpha beta gamma\n\n## B\nbravo charlie\n";

    // HEAD + GET for add
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

    // Search JSON
    let mut cmd = blz_cmd();
    let out = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["search", "alpha", "--source", "e2e", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v: Value = serde_json::from_slice(&out)?;
    // Top-level keys
    for key in [
        "query",
        "page",
        "limit",
        "totalResults",
        "totalPages",
        "totalLinesSearched",
        "searchTimeMs",
        "sources",
        "results",
    ] {
        assert!(v.get(key).is_some(), "missing top-level key: {key}");
    }
    assert!(
        v.get("total_hits").is_some(),
        "missing compatibility key: total_hits"
    );
    assert!(
        v.get("execution_time_ms").is_some(),
        "missing compatibility key: execution_time_ms"
    );
    assert_eq!(
        v.get("total_hits"),
        v.get("totalResults"),
        "compat total_hits should mirror totalResults"
    );
    assert_eq!(
        v.get("execution_time_ms"),
        v.get("searchTimeMs"),
        "compat execution_time_ms should mirror searchTimeMs"
    );

    // Results shape (at least one)
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!results.is_empty(), "expected at least one result");
    let r0 = &results[0];
    for key in [
        "source",
        "file",
        "headingPath",
        "lines",
        "snippet",
        "scorePercentage",
        "sourceUrl",
        "checksum",
        "anchor",
    ] {
        assert!(r0.get(key).is_some(), "missing result key: {key}");
    }
    assert!(
        r0["score"].is_number(),
        "score should be present and numeric"
    );
    let sp = r0
        .get("scorePercentage")
        .and_then(Value::as_f64)
        .expect("scorePercentage number");
    assert!(
        (0.0..=100.0).contains(&sp),
        "scorePercentage out of range: {sp}"
    );

    Ok(())
}
