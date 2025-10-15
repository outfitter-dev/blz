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
    let requests = json["requests"].as_array().expect("requests array");
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request["lineStart"].as_u64().unwrap(), 3);
    assert_eq!(request["lineEnd"].as_u64().unwrap(), 7);
    let snippet = request["snippet"].as_str().unwrap();
    assert!(
        snippet.contains("line 4"),
        "Block snippet should include surrounding lines"
    );
    assert!(
        request.get("truncated").is_none(),
        "Non-truncated block should omit truncated flag"
    );

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
    let truncated_requests = truncated_json["requests"]
        .as_array()
        .expect("requests array");
    assert_eq!(truncated_requests.len(), 1);
    let truncated_request = &truncated_requests[0];
    assert_eq!(truncated_request["lineStart"].as_u64().unwrap(), 3);
    assert_eq!(truncated_request["lineEnd"].as_u64().unwrap(), 5);
    assert!(
        truncated_request["snippet"]
            .as_str()
            .unwrap()
            .contains("line 2 target"),
        "Snippet should still include target line"
    );
    assert_eq!(truncated_request["truncated"].as_bool(), Some(true));

    Ok(())
}
