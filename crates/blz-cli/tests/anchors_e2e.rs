#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "anchors")]

mod common;

use common::blz_cmd;
use std::path::PathBuf;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn find_lines<'a>(list: &'a [blz_core::TocEntry], name: &str) -> Option<&'a str> {
    for e in list {
        if e.heading_path.last().map(std::string::String::as_str) == Some(name) {
            return Some(e.lines.as_str());
        }
        if let Some(l) = find_lines(&e.children, name) {
            return Some(l);
        }
    }
    None
}

#[tokio::test]
async fn add_update_generates_anchors_mapping() -> anyhow::Result<()> {
    // Temporary data dir to isolate test
    let tmp = tempdir()?;

    // Mock server with initial and updated content
    let server = MockServer::start().await;
    let url = format!("{}/llms.txt", server.uri());

    // v1 content: A then B then C
    let v1 = "# Title\n\n## A\nalpha\n\n## B\nbravo\n\n## C\ncharlie\n";
    // v2 content: C then A then B (A moved)
    let v2 = "# Title\n\n## C\ncharlie\n\n## A\nalpha\n\n## B\nbravo\n";

    // HEAD for preflight
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(
            ResponseTemplate::new(200).insert_header("content-length", v1.len().to_string()),
        )
        .mount(&server)
        .await;
    // GET for add
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(v1))
        .mount(&server)
        .await;

    // Run add via CLI (non-interactive)
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .arg("add")
        .arg("e2e")
        .arg(&url)
        .arg("-y");
    cmd.assert().success();

    // Read pre-update JSON
    let old_json_path = PathBuf::from(tmp.path()).join("e2e").join("llms.json");
    let old_json_txt = std::fs::read_to_string(&old_json_path)?;
    let old_llms: blz_core::LlmsJson = serde_json::from_str(&old_json_txt)?;

    // Reset mocks and update server responses for update path (HEAD + GET)
    server.reset().await;
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(
            ResponseTemplate::new(200).insert_header("content-length", v2.len().to_string()),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(v2))
        .mount(&server)
        .await;

    // Run update via CLI
    let mut cmd = blz_cmd();
    cmd.env("BLZ_DATA_DIR", tmp.path())
        .arg("update")
        .arg("e2e")
        .arg("--quiet");
    cmd.assert().success();

    // Verify mappings by recomputing on llms.json old vs new
    let new_json_path = PathBuf::from(tmp.path()).join("e2e").join("llms.json");
    let new_json_txt = std::fs::read_to_string(&new_json_path)?;
    let new_llms: blz_core::LlmsJson = serde_json::from_str(&new_json_txt)?;
    // Quick sanity: ensure 'A' moved lines
    let a_old = find_lines(&old_llms.toc, "A").unwrap_or("");
    let a_new = find_lines(&new_llms.toc, "A").unwrap_or("");
    assert_ne!(
        a_old, a_new,
        "expected 'A' lines to change (old: {a_old}, new: {a_new})"
    );

    let computed = blz_core::compute_anchor_mappings(&old_llms.toc, &new_llms.toc);
    assert!(
        !computed.is_empty(),
        "expected computed mappings when sections moved"
    );

    // If anchors.json exists, ensure it matches non-empty
    let map_path = PathBuf::from(tmp.path()).join("e2e").join("anchors.json");
    if map_path.exists() {
        let txt = std::fs::read_to_string(&map_path)?;
        let map: blz_core::AnchorsMap = serde_json::from_str(&txt)?;
        assert!(
            !map.mappings.is_empty(),
            "expected mappings saved in anchors.json"
        );
    }

    // Also verify CLI anchors output JSON shape
    let mut cmd = blz_cmd();
    let stdout = cmd
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["anchors", "e2e", "--mappings", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let map: blz_core::AnchorsMap = serde_json::from_slice(&stdout)?;
    assert!(
        !map.mappings.is_empty(),
        "anchors CLI should output non-empty mappings"
    );
    let first = &map.mappings[0];
    assert!(!first.anchor.is_empty());
    assert!(!first.old_lines.is_empty());
    assert!(!first.new_lines.is_empty());
    assert!(!first.heading_path.is_empty());

    Ok(())
}
