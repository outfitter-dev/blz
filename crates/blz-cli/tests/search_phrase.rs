#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn search_phrase_queries_match_exact_sequence() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    let doc = "# Claude Guide\n\n## Claude Code\nClaude Code lets you edit collaboratively.\n\n## Separately\nClaude excels at many tasks including writing code.\n";

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
        .args(["add", "e2e", &url, "-y"])
        .assert()
        .success();

    let output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["search", "\"claude code\"", "--alias", "e2e", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output)?;
    let results = json
        .get("results")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();

    assert_eq!(results.len(), 1, "expected only the phrase match to return");
    let snippet = results[0]
        .get("snippet")
        .and_then(|s| s.as_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    assert!(snippet.contains("claude code"));

    Ok(())
}
