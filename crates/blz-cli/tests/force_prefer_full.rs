#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;
use common::blz_cmd;

/// Test that add command fetches llms-full.txt first when FORCE_PREFER_FULL is enabled
#[tokio::test]
async fn test_add_fetches_full_when_available() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    let base_doc = "# Base Docs\n\nBase flavor content";
    let full_doc = "# Full Docs\n\nFull flavor content with more details";

    // HEAD handlers for flavor discovery
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

    // GET handlers - full should be fetched first due to FORCE_PREFER_FULL
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
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "test-source", &url, "-y"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output)?;

    // With FORCE_PREFER_FULL, both flavors should be fetched
    assert!(
        stdout.contains("llms-full: "),
        "expected summary to include llms-full flavor"
    );

    // Verify llms-full.txt file exists
    let source_dir = data_dir.path().join("test-source");
    assert!(
        source_dir.join("llms-full.txt").exists(),
        "expected llms-full.txt to exist"
    );
    assert!(
        source_dir.join("llms-full.json").exists(),
        "expected llms-full.json to exist"
    );

    // Verify the content is from llms-full
    let content = std::fs::read_to_string(source_dir.join("llms-full.txt"))?;
    assert!(
        content.contains("Full flavor content"),
        "expected llms-full.txt to contain full flavor content"
    );

    Ok(())
}

/// Test that search automatically uses llms-full when available
#[tokio::test]
async fn test_search_uses_full_automatically() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    let base_doc = "# Base Docs\n\n## Section\nBase content here";
    let full_doc = "# Full Docs\n\n## Section\nFull content with UNIQUE_MARKER here";

    // Setup mocks
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

    let url = format!("{}/llms.txt", server.uri());

    // Add source
    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "test-source", &url, "-y"])
        .assert()
        .success();

    // Search should use llms-full automatically
    let search_output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["search", "UNIQUE_MARKER", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let search_results: Value = serde_json::from_slice(&search_output)?;

    // Search results are wrapped in an object with "results" field
    let hits = search_results
        .get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    assert!(
        !hits.is_empty(),
        "expected search to find results from llms-full"
    );

    // Verify the search result includes the unique marker from llms-full
    let first_hit = &hits[0];
    let snippet = first_hit
        .get("snippet")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    assert!(
        snippet.contains("UNIQUE_MARKER"),
        "expected search result to contain content from llms-full, got snippet: {}",
        snippet
    );

    Ok(())
}

/// Test that list command shows llms-full as the search flavor
#[tokio::test]
async fn test_list_shows_full_as_search_flavor() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    let base_doc = "# Base";
    let full_doc = "# Full";

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

    let url = format!("{}/llms.txt", server.uri());

    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "test-source", &url, "-y"])
        .assert()
        .success();

    // List should show llms-full as the search flavor
    let list_output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["list", "-f", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let list_value: Value = serde_json::from_slice(&list_output)?;
    let list_arr = list_value.as_array().cloned().unwrap_or_default();

    assert!(!list_arr.is_empty(), "expected list to contain source");

    let search_flavor = list_arr[0]
        .get("searchFlavor")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    assert_eq!(
        search_flavor, "llms-full",
        "expected searchFlavor to be 'llms-full' when FORCE_PREFER_FULL is true"
    );

    Ok(())
}

/// Test that upgrade command works correctly
///
/// This test uses a workaround because wiremock mocks override each other.
/// We disable FORCE_PREFER_FULL for the initial add by using an env override.
#[tokio::test]
async fn test_upgrade_command_basic() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    let base_doc = "# Base Only\n\nBase content";
    let full_doc = "# Full Version\n\nFull content with extras";

    // Setup mocks: llms.txt always available, llms-full.txt initially 404
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

    // Make llms-full return 404 initially (will be overridden later)
    let mock_404 = Mock::given(method("HEAD"))
        .and(path("/llms-full.txt"))
        .respond_with(ResponseTemplate::new(404))
        .named("llms-full-404")
        .mount_as_scoped(&server)
        .await;

    let url = format!("{}/llms.txt", server.uri());

    // Add source with only base flavor
    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "upgradeable", &url, "-y"])
        .assert()
        .success();

    // Verify only llms.txt exists
    let source_dir = data_dir.path().join("upgradeable");
    assert!(
        source_dir.join("llms.txt").exists(),
        "expected llms.txt to exist after initial add"
    );
    assert!(
        !source_dir.join("llms-full.txt").exists(),
        "expected llms-full.txt to NOT exist after initial add"
    );

    // Drop the 404 mock and add 200 mocks for llms-full
    drop(mock_404);

    Mock::given(method("HEAD"))
        .and(path("/llms-full.txt"))
        .respond_with(ResponseTemplate::new(200))
        .named("llms-full-200")
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms-full.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(full_doc))
        .mount(&server)
        .await;

    // Run upgrade command
    let upgrade_output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["upgrade", "upgradeable", "-y"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(upgrade_output)?;
    assert!(
        stdout.contains("Upgraded upgradeable to llms-full.txt") || stdout.contains("✓ Upgraded"),
        "expected upgrade success message, got: {}",
        stdout
    );

    // Verify llms-full.txt now exists
    assert!(
        source_dir.join("llms-full.txt").exists(),
        "expected llms-full.txt to exist after upgrade"
    );

    Ok(())
}

/// Test that upgrade command skips sources already using llms-full
#[tokio::test]
async fn test_upgrade_command_skip_if_already_full() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    let full_doc = "# Already Full\n\nFull content";

    // Setup with llms-full already available
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
        .respond_with(ResponseTemplate::new(200).set_body_string("# Base"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms-full.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(full_doc))
        .mount(&server)
        .await;

    let url = format!("{}/llms.txt", server.uri());

    // Add source - will fetch both flavors
    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "already-full", &url, "-y"])
        .assert()
        .success();

    // Try to upgrade - should report already up to date
    let upgrade_output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["upgrade", "already-full", "-y"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(upgrade_output)?;
    assert!(
        stdout.contains("already using llms-full.txt") || stdout.contains("up to date"),
        "expected message indicating source is already using llms-full"
    );

    Ok(())
}
