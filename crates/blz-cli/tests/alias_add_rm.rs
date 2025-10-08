#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn alias_add_and_remove_updates_list_json() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    // Basic doc
    let doc = "# Title\n\n## Section\nContent\n";
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

    // Add source
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["add", "e2e", &url, "-y"])
        .assert()
        .success();

    // Add alias
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["alias", "add", "e2e", "@scope/package"])
        .assert()
        .success();

    // List JSON should include aliases
    let mut cmd = blz_cmd();
    let out = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["list", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out)?;
    let arr = v.as_array().cloned().unwrap_or_default();
    let s0 = &arr[0];
    assert!(s0.get("aliases").is_some());
    let aliases = s0
        .get("aliases")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(aliases.iter().any(|a| a.as_str() == Some("@scope/package")));

    // Remove alias
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["alias", "rm", "e2e", "@scope/package"])
        .assert()
        .success();

    // List JSON no longer contains alias
    let mut cmd = blz_cmd();
    let out2 = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["list", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v2: Value = serde_json::from_slice(&out2)?;
    let arr2 = v2.as_array().cloned().unwrap_or_default();
    let s02 = &arr2[0];
    let aliases2 = s02
        .get("aliases")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        !aliases2
            .iter()
            .any(|a| a.as_str() == Some("@scope/package"))
    );

    Ok(())
}
