#![allow(missing_docs)]
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
    assert_cmd::Command::cargo_bin("blz")?
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "e2e", &url, "-y"])
        .assert()
        .success();

    // Get anchors JSON
    let anchors_out = assert_cmd::Command::cargo_bin("blz")?
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["anchors", "e2e", "-o", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Value = serde_json::from_slice(&anchors_out)?;
    let arr = entries.as_array().cloned().unwrap_or_default();
    assert!(!arr.is_empty(), "expected anchors list");
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
    let get_out = assert_cmd::Command::cargo_bin("blz")?
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
