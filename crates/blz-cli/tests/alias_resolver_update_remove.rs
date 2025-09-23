#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn update_and_remove_accept_metadata_alias() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    // HEAD + GET
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

    // Add canonical
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "canon", &url, "-y"])
        .assert()
        .success();

    // Add metadata alias
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .args(["alias", "add", "canon", "@scope/pkg"])
        .assert()
        .success();

    // Update using metadata alias
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .args(["update", "@scope/pkg", "--quiet"])
        .assert()
        .success();

    // Remove using metadata alias
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .args(["remove", "@scope/pkg", "-y"]) // should resolve to canonical and delete
        .assert()
        .success();

    // List should be empty now
    let mut cmd = blz_cmd();
    let out = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["list", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out)?;
    assert!(v.as_array().is_none_or(std::vec::Vec::is_empty));

    Ok(())
}
