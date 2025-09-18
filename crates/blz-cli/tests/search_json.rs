#![allow(missing_docs)]
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
    assert_cmd::Command::cargo_bin("blz")?
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "e2e", &url, "-y"])
        .assert()
        .success();

    // Search JSON
    let out = assert_cmd::Command::cargo_bin("blz")?
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["search", "alpha", "--alias", "e2e", "-f", "json"])
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

    // Results shape (at least one)
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!results.is_empty(), "expected at least one result");
    let r0 = &results[0];
    for key in [
        "alias",
        "file",
        "headingPath",
        "lines",
        "snippet",
        "score",
        "sourceUrl",
        "checksum",
        "anchor",
    ] {
        assert!(r0.get(key).is_some(), "missing result key: {key}");
    }

    Ok(())
}
