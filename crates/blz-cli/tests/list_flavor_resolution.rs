//! Test flavor resolution in list command

use anyhow::Result;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;

#[tokio::test]
async fn list_resolves_flavor_with_preferences() -> Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    let base_doc = "# Base Docs\n\nBase content";
    let full_doc = "# Full Docs\n\nFull content with more details";

    // Mock both flavors available
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

    // Add source - should fetch both flavors
    common::blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "test", &format!("{}/llms.txt", server.uri()), "-y"])
        .output()?;

    // Test 1: Default behavior (no preference) - should use llms
    let output = common::blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .args(["list", "--format", "json"])
        .output()?;
    assert!(output.status.success());

    let json: Value = serde_json::from_slice(&output.stdout)?;
    let sources = json.as_array().expect("Expected JSON array");
    assert_eq!(sources.len(), 1);

    let source = &sources[0];
    assert_eq!(
        source["searchFlavor"], "llms",
        "Default should be llms without preferences"
    );
    assert_eq!(
        source["defaultFlavor"], "llms",
        "defaultFlavor should mirror searchFlavor"
    );

    // Verify both flavors are listed
    let flavors = source["flavors"]
        .as_array()
        .expect("Expected flavors array");
    assert_eq!(flavors.len(), 2);
    let flavor_names: Vec<&str> = flavors
        .iter()
        .filter_map(|f| f.get("flavor").and_then(|v| v.as_str()))
        .collect();
    assert!(flavor_names.contains(&"llms"));
    assert!(flavor_names.contains(&"llms-full"));

    // Test 2: With BLZ_PREFER_LLMS_FULL=1 - should use llms-full
    let output = common::blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "1")
        .args(["list", "--format", "json"])
        .output()?;
    assert!(output.status.success());

    let json: Value = serde_json::from_slice(&output.stdout)?;
    let sources = json.as_array().expect("Expected JSON array");
    let source = &sources[0];
    assert_eq!(
        source["searchFlavor"], "llms-full",
        "Should use llms-full with preference"
    );
    assert_eq!(
        source["defaultFlavor"], "llms-full",
        "defaultFlavor should mirror searchFlavor for llms-full preference"
    );

    Ok(())
}

#[tokio::test]
async fn list_handles_missing_flavors_gracefully() -> Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    let base_doc = "# Docs\n\nOnly base content";

    // Only llms.txt available
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("HEAD"))
        .and(path("/llms-full.txt"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(base_doc))
        .mount(&server)
        .await;

    // Add source - should only fetch llms.txt
    common::blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "test", &format!("{}/llms.txt", server.uri()), "-y"])
        .output()?;

    // List should still work with only one flavor
    let output = common::blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .args(["list", "--format", "json"])
        .output()?;
    assert!(output.status.success());

    let json: Value = serde_json::from_slice(&output.stdout)?;
    let sources = json.as_array().expect("Expected JSON array");
    assert_eq!(sources.len(), 1);

    let source = &sources[0];
    assert_eq!(source["searchFlavor"], "llms");
    assert_eq!(source["defaultFlavor"], "llms");

    // Should only have one flavor
    let flavors = source["flavors"]
        .as_array()
        .expect("Expected flavors array");
    assert_eq!(flavors.len(), 1);
    assert_eq!(flavors[0]["flavor"], "llms");

    Ok(())
}

#[tokio::test]
async fn list_jsonl_format_includes_search_flavor() -> Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    let base_doc = "# Docs\n\nContent";

    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(base_doc))
        .mount(&server)
        .await;

    // Add source
    common::blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "test", &format!("{}/llms.txt", server.uri()), "-y"])
        .output()?;

    // Test JSONL format
    let output = common::blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .args(["list", "--format", "jsonl"])
        .output()?;
    assert!(output.status.success());

    let output_str = String::from_utf8(output.stdout)?;
    let lines: Vec<&str> = output_str.lines().collect();
    assert_eq!(lines.len(), 1, "Expected one JSONL line");

    let json: Value = serde_json::from_str(lines[0])?;
    assert!(
        json["searchFlavor"].is_string(),
        "searchFlavor should be present in JSONL"
    );
    assert_eq!(json["searchFlavor"], "llms");
    assert_eq!(json["defaultFlavor"], "llms");

    Ok(())
}
