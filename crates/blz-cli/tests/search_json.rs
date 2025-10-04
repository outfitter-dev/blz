#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use chrono::{DateTime, Utc};
use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

fn run_json(tmp: &tempfile::TempDir, args: &[&str]) -> anyhow::Result<Value> {
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

fn assert_search_payload_schema(payload: &Value) {
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
        assert!(payload.get(key).is_some(), "missing top-level key: {key}");
    }

    assert!(
        payload.get("total_hits").is_some(),
        "missing compatibility key: total_hits"
    );
    assert!(
        payload.get("execution_time_ms").is_some(),
        "missing compatibility key: execution_time_ms"
    );
    assert_eq!(
        payload.get("total_hits"),
        payload.get("totalResults"),
        "compat total_hits should mirror totalResults"
    );
    assert_eq!(
        payload.get("execution_time_ms"),
        payload.get("searchTimeMs"),
        "compat execution_time_ms should mirror searchTimeMs"
    );
}

fn assert_result_schema(result: &Value) {
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
        "fetchedAt",
        "isStale",
    ] {
        assert!(result.get(key).is_some(), "missing result key: {key}");
    }

    assert!(
        result["score"].is_number(),
        "score should be present and numeric"
    );

    let sp = result
        .get("scorePercentage")
        .and_then(Value::as_f64)
        .expect("scorePercentage number");
    assert!(
        (0.0..=100.0).contains(&sp),
        "scorePercentage out of range: {sp}"
    );
}

#[tokio::test]
async fn search_json_schema_contains_expected_fields() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = "# Title\n\n## A\nalpha beta gamma\n\n## B\nbravo charlie\n";
    seed_source(&tmp, &server, "e2e", doc).await?;

    let payload = run_json(&tmp, &["search", "alpha", "--source", "e2e", "-f", "json"])?;

    assert_search_payload_schema(&payload);

    let results = payload
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!results.is_empty(), "expected at least one result");

    let first = &results[0];
    assert_result_schema(first);

    // fetchedAt should be ISO-8601 parsable and isStale should default to false for fresh sources
    let fetched_at = first
        .get("fetchedAt")
        .and_then(Value::as_str)
        .expect("fetchedAt string present");
    let parsed: DateTime<Utc> = fetched_at
        .parse()
        .expect("fetchedAt should parse as RFC3339 timestamp");
    let now = Utc::now();
    assert!(
        parsed <= now,
        "fetchedAt should not be in the future: {parsed} > {now}"
    );
    let is_stale = first
        .get("isStale")
        .and_then(Value::as_bool)
        .expect("isStale boolean present");
    assert!(!is_stale, "freshly added source should not be stale");

    Ok(())
}

#[tokio::test]
async fn search_marks_stale_results_when_metadata_is_old() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = "# Title\n\n## Section\ncontent line one\ncontent line two\n";
    seed_source(&tmp, &server, "aging", doc).await?;

    // Rewrite metadata to simulate an old fetch timestamp (60 days ago)
    let metadata_path = tmp
        .path()
        .join("sources")
        .join("aging")
        .join("metadata.json");
    let mut metadata: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&metadata_path)?)?;
    let old_timestamp = (Utc::now() - chrono::Duration::days(60)).to_rfc3339();
    if let Value::Object(ref mut map) = metadata {
        map.insert("fetched_at".to_string(), Value::String(old_timestamp));
    }
    std::fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)?;

    let v = run_json(
        &tmp,
        &["search", "content", "--source", "aging", "-f", "json"],
    )?;
    let result = v
        .get("results")
        .and_then(|r| r.as_array())
        .and_then(|arr| arr.first())
        .cloned()
        .expect("expected single result");
    let is_stale = result
        .get("isStale")
        .and_then(Value::as_bool)
        .expect("isStale boolean present");
    assert!(is_stale, "stale metadata should yield isStale = true");

    Ok(())
}
