#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

use std::fs;

use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;
use common::blz_cmd;

#[tokio::test]
async fn add_fetches_all_discovered_flavors() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    let base_doc = "# Docs\n\nBase content";
    let full_doc = "# Docs\n\nFull content";

    // HEAD handlers so flavor discovery succeeds.
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

    // GET handlers for both variants.
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
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "docs", &url, "-y"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output)?;

    assert!(
        stdout.contains("llms: "),
        "expected summary to include base flavor"
    );
    assert!(
        stdout.contains("llms-full: "),
        "expected summary to include llms-full flavor"
    );

    let docs_dir = data_dir.path().join("docs");
    let files = [
        "llms.txt",
        "llms-full.txt",
        "llms.json",
        "llms-full.json",
        "metadata.json",
        "metadata-llms-full.json",
    ];
    for name in files {
        assert!(
            docs_dir.join(name).exists(),
            "expected {name:?} to exist after add"
        );
    }

    // Index directory should be populated with Tantivy files.
    let index_dir = docs_dir.join(".index");
    assert!(
        index_dir.exists(),
        "expected .index directory to be created"
    );
    assert!(
        fs::read_dir(&index_dir)?.next().is_some(),
        "expected index directory to contain files"
    );

    // `blz list` should enumerate all flavors including llms-full.
    let list_output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .args(["list", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list_value: Value = serde_json::from_slice(&list_output)?;
    let list_arr = list_value.as_array().cloned().unwrap_or_default();
    assert!(
        !list_arr.is_empty(),
        "expected list output to contain the added source"
    );
    let flavor_entries = list_arr[0]
        .get("flavors")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        flavor_entries.iter().any(|entry| {
            entry
                .get("flavor")
                .and_then(|v| v.as_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("llms-full"))
        }),
        "expected list output to include llms-full flavor"
    );

    Ok(())
}
