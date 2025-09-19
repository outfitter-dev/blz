#![allow(missing_docs)]

mod common;

use common::blz_cmd;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn update_preflight_fails_on_non_2xx_head() -> anyhow::Result<()> {
    // Temp data dir
    let tmp = tempdir()?;

    // Mock server
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    // Initial add succeeds (HEAD 200, GET 200)
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "10"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("# Title\n\n## A\nalpha\n"))
        .mount(&server)
        .await;

    // Add via CLI
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "e2e", &url, "-y"])
        .assert()
        .success();

    // Now HEAD returns 404 → update should fail fast
    server.reset().await;
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .args(["update", "e2e", "--quiet"])
        .assert()
        .failure();

    // No strict stderr assertion (platform-dependent), but test ensures non-zero exit
    Ok(())
}
