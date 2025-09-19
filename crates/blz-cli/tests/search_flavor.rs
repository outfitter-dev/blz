#![allow(missing_docs)]

mod common;
use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn search_defaults_to_base_flavor_and_respects_override() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    // Base flavor content: does not mention "full-only" token
    let base_doc = "# Docs\n\n## Overview\nbase-only insight\n";
    let full_doc = "# Docs\n\n## Overview\nfull-only expansion\n";

    // HEAD handlers
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .and(path("/llms-full.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // GET handlers
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(base_doc))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms-full.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(full_doc))
        .mount(&server)
        .await;

    let url = format!("{}/llms.txt", server.uri());

    // Add source (fetches both variants)
    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "docs", &url, "-y"])
        .assert()
        .success();

    // Search for token only present in full flavor; should return zero hits by default
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["search", "full-only", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let results: Value = serde_json::from_slice(&output)?;
    let empty_hits = results
        .get("results")
        .and_then(Value::as_array)
        .map_or(true, |arr| arr.is_empty());
    assert!(empty_hits, "expected no hits when default flavor is llms");

    // Upgrade preference to llms-full
    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["update", "docs", "--flavor", "full", "--quiet", "--yes"])
        .assert()
        .success();

    // Search again; should now yield hits from full flavor
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["search", "full-only", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let results: Value = serde_json::from_slice(&output)?;
    let hits = results
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        !hits.is_empty(),
        "expected full flavor results after override"
    );

    Ok(())
}
