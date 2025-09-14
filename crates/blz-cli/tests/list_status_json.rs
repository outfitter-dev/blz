use assert_cmd::prelude::*;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn list_status_json_includes_source_and_keys() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    let doc = "# Title\n\n## A\nalpha\n";
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

    // Add a source
    assert_cmd::Command::cargo_bin("blz")?
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "e2e", &url, "-y"])
        .assert()
        .success();

    // List with status JSON
    let out = assert_cmd::Command::cargo_bin("blz")?
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["list", "--status", "-o", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out)?;
    let arr = v.as_array().cloned().unwrap_or_default();
    assert!(!arr.is_empty(), "expected at least one source in list");
    let s0 = &arr[0];
    for key in ["alias", "source", "url", "fetchedAt", "lines", "sha256"] {
        assert!(s0.get(key).is_some(), "missing key: {key}");
    }
    Ok(())
}
