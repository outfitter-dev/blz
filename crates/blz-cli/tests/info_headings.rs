#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn info_json_includes_headings_count() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    // Document with multiple headings
    let doc = "# Title\n\n## Section A\nalpha content\n\n### Subsection A1\nmore content\n\n## Section B\nbeta content\n";
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
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["add", "testdoc", &url, "-y"])
        .assert()
        .success();

    // Get info with JSON format
    let mut cmd = blz_cmd();
    let out = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["info", "testdoc", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let info: Value = serde_json::from_slice(&out)?;

    // Verify headings field is present and not null
    assert!(
        info.get("headings").is_some(),
        "headings field should be present"
    );
    assert!(
        !info.get("headings").unwrap().is_null(),
        "headings field should not be null"
    );

    // Verify it's a number (should be 4 headings: Title, Section A, Subsection A1, Section B)
    let headings_count = info
        .get("headings")
        .unwrap()
        .as_u64()
        .expect("headings should be a number");
    assert_eq!(
        headings_count, 4,
        "should count 4 headings in the test document"
    );

    // Also verify other essential fields are present
    for key in ["alias", "url", "lines", "headings"] {
        assert!(info.get(key).is_some(), "missing key: {key}");
    }

    Ok(())
}

#[tokio::test]
async fn info_headings_matches_list() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    // Document with headings
    let doc = "# Title\n\n## A\nalpha\n\n## B\nbeta\n";
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
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["add", "testdoc", &url, "-y"])
        .assert()
        .success();

    // Get info headings count
    let mut cmd = blz_cmd();
    let info_out = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["info", "testdoc", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let info: Value = serde_json::from_slice(&info_out)?;
    let info_headings = info
        .get("headings")
        .unwrap()
        .as_u64()
        .expect("info headings should be a number");

    // Get list headings count
    let mut cmd = blz_cmd();
    let list_out = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .env("BLZ_CONFIG_DIR", tmp.path())
        .args(["list", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list: Value = serde_json::from_slice(&list_out)?;
    let list_arr = list.as_array().expect("list should be an array");
    assert!(
        !list_arr.is_empty(),
        "list should contain at least one source"
    );
    let first_source = &list_arr[0];
    let list_headings = first_source
        .get("headings")
        .unwrap()
        .as_u64()
        .expect("list headings should be a number");

    // They should match
    assert_eq!(
        info_headings, list_headings,
        "info and list commands should return the same headings count"
    );

    Ok(())
}
