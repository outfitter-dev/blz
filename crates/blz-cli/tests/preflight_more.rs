#![allow(missing_docs)]
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn update_preflight_fails_on_500() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    // Initial add
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("# T\n\n## A\nalpha\n"))
        .mount(&server)
        .await;
    assert_cmd::Command::cargo_bin("blz")?
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "e2e", &url, "-y"])
        .assert()
        .success();

    // HEAD 500 should fail
    server.reset().await;
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    assert_cmd::Command::cargo_bin("blz")?
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["update", "e2e", "--quiet"])
        .assert()
        .failure();
    Ok(())
}

#[tokio::test]
async fn update_preflight_no_content_length_still_proceeds() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    // Initial add
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("# T\n\n## A\nalpha\n"))
        .mount(&server)
        .await;
    assert_cmd::Command::cargo_bin("blz")?
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "e2e", &url, "-y"])
        .assert()
        .success();

    // HEAD 200 but no Content-Length; GET 200 with same body → update should succeed (unchanged or updated)
    server.reset().await;
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("# T\n\n## A\nalpha\n"))
        .mount(&server)
        .await;
    assert_cmd::Command::cargo_bin("blz")?
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["update", "e2e", "--quiet"])
        .assert()
        .success();
    Ok(())
}
