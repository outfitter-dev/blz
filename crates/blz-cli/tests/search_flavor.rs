#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;
use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[allow(clippy::too_many_lines)]
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
        .is_none_or(std::vec::Vec::is_empty);
    assert!(empty_hits, "expected no hits when default flavor is llms");

    // Explicitly request full flavor without adjusting preferences
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args([
            "search",
            "full-only",
            "--format",
            "json",
            "--flavor",
            "full",
        ])
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
        "expected full flavor results when using --flavor full override"
    );

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

    // Force base flavor to fall back to llms.txt even with full preference
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["search", "full-only", "--format", "json", "--flavor", "txt"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let results: Value = serde_json::from_slice(&output)?;
    let empty_hits = results
        .get("results")
        .and_then(Value::as_array)
        .is_none_or(std::vec::Vec::is_empty);
    assert!(
        empty_hits,
        "expected no hits when forcing base flavor after full preference"
    );

    Ok(())
}

#[tokio::test]
async fn text_formatter_uses_correct_flavor_for_snippets() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let server = MockServer::start().await;

    // Create two flavors with DIFFERENT content at the same lines
    // This is critical: if formatter loads wrong flavor, snippets will be incorrect
    let base_doc = r#"# API Documentation

## Authentication
Line 4: Base flavor authentication method
Line 5: Use API key in header

## Rate Limits
Line 8: Base flavor rate limit info
"#;

    let full_doc = r#"# API Documentation

## Authentication
Line 4: FULL FLAVOR authentication with OAuth2
Line 5: Detailed OAuth2 flow explanation

## Rate Limits
Line 8: FULL FLAVOR detailed rate limit tiers
"#;

    // Setup mock server
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
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args(["add", "api", &url, "-y"])
        .assert()
        .success();

    // Search full flavor in TEXT format and capture output
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args([
            "search",
            "authentication",
            "--flavor",
            "full",
            "--format",
            "text",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    // Critical assertion: text output MUST contain snippets from FULL flavor, not base
    assert!(
        stdout.contains("FULL FLAVOR"),
        "Text formatter must display snippets from full flavor when searching full flavor.\n\
         Expected 'FULL FLAVOR' in output but got:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("Base flavor"),
        "Text formatter must NOT display snippets from base flavor when searching full flavor.\n\
         Found 'Base flavor' in output:\n{}",
        stdout
    );

    // Verify the opposite: searching base flavor shows base snippets
    let output = blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_PREFER_LLMS_FULL", "0")
        .env("BLZ_CONFIG_DIR", data_dir.path())
        .args([
            "search",
            "authentication",
            "--flavor",
            "txt",
            "--format",
            "text",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);

    assert!(
        stdout.contains("Base flavor"),
        "Text formatter must display snippets from base flavor when searching base flavor.\n\
         Expected 'Base flavor' in output but got:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("FULL FLAVOR"),
        "Text formatter must NOT display snippets from full flavor when searching base flavor.\n\
         Found 'FULL FLAVOR' in output:\n{}",
        stdout
    );

    Ok(())
}
