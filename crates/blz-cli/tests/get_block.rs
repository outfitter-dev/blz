#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn get_block_returns_heading_section_with_optional_truncation() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    let doc = "# Title\n\n## Section\nline 1\nline 2 target\nline 3\nline 4\n\n## Next\nline 5\n";

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

    blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", "getter", &url, "-y"])
        .assert()
        .success();

    // Request block around line 3 (Section heading lines should be included)
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["get", "getter:3", "--block", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output)?;
    assert_eq!(json["lines"].as_str().unwrap(), "3-7");
    assert_eq!(json["lineNumbers"].as_array().unwrap().len(), 4);
    assert!(json["content"].as_str().unwrap().contains("line 4"));

    // Truncated block should respect max-lines and flag truncation
    let truncated = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args([
            "get",
            "getter:3",
            "--block",
            "--max-lines",
            "2",
            "-f",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let truncated_json: Value = serde_json::from_slice(&truncated)?;
    assert_eq!(truncated_json["lines"].as_str().unwrap(), "3-5");
    assert_eq!(truncated_json["lineNumbers"].as_array().unwrap().len(), 2);
    assert!(
        truncated_json["content"]
            .as_str()
            .unwrap()
            .contains("line 2 target")
    );
    assert_eq!(truncated_json["truncated"].as_bool(), Some(true));

    Ok(())
}
