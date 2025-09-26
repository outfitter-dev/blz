#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd;
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
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "e2e", &url, "-y"])
        .assert()
        .success();

    // List with status JSON
    let mut cmd = blz_cmd();
    let out = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["list", "--status", "-f", "json"])
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
    let flavors = s0
        .get("flavors")
        .and_then(|v| v.as_array())
        .expect("expected flavors array in list output");
    assert!(!flavors.is_empty(), "expected at least one flavor entry");
    assert_eq!(
        flavors[0]
            .get("flavor")
            .and_then(|v| v.as_str())
            .unwrap_or_default(),
        "llms"
    );

    // Verify searchFlavor matches the resolved default
    assert_eq!(
        s0.get("searchFlavor")
            .and_then(|v| v.as_str())
            .unwrap_or_default(),
        "llms",
        "expected searchFlavor to match the resolved default flavor"
    );
    Ok(())
}
