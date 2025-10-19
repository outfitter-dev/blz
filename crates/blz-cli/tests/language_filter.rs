#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;
use common::blz_cmd;

/// Ensure the Anthropic-style multilingual docs are filtered down to English content only.
#[tokio::test]
async fn test_language_filter_blocks_non_english_headings() -> anyhow::Result<()> {
    let data_dir = tempdir()?;
    let config_dir = tempdir()?;
    let server = MockServer::start().await;

    let doc = r"# Anthropic Docs

## Getting Started
English-only onboarding content.

## USA per-region quotas
Guidance for US-based quotas that must remain in English.

## Documentación
Guía rápida para desarrolladores de Anthropic.

## Documentacion
Version sin acentos que debería seguir siendo filtrada.

## Flussi di lavoro comuni
Suggerimenti in lingua italiana per gli agenti.

## Benutzerdefinierte Slash-Befehle erstellen
Anleitung in tedesco con caratteri non inglesi.

## Utilisez notre améliorateur de prompts
Contenu français avec diacritiques.
";

    // Serve fixture content
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

    let url = format!("{}/llms.txt", server.uri());

    // Index the mock source
    blz_cmd()
        .env("BLZ_DATA_DIR", data_dir.path())
        .env("BLZ_CONFIG_DIR", config_dir.path())
        .args(["add", "anthropic", &url, "-y"])
        .assert()
        .success();

    // Helper to run a JSON search and pull out the hits array
    let run_search = |query: &str| -> anyhow::Result<Vec<Value>> {
        let output = blz_cmd()
            .env("BLZ_DATA_DIR", data_dir.path())
            .env("BLZ_CONFIG_DIR", config_dir.path())
            .args(["search", query, "-f", "json"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let value: Value = serde_json::from_slice(&output)?;
        Ok(value
            .get("results")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default())
    };

    // English heading that previously produced a false positive should remain searchable
    let english_hits = run_search("USA per-region quotas")?;
    assert!(
        !english_hits.is_empty(),
        "expected English heading to remain indexed"
    );

    // Non-English headings should not be present after filtering
    for query in [
        "Documentación",
        "Documentacion",
        "Flussi di lavoro comuni",
        "Benutzerdefinierte Slash-Befehle",
        "Utilisez notre améliorateur",
    ] {
        let hits = run_search(query)?;
        assert!(
            hits.is_empty(),
            "expected no hits for non-English heading \"{query}\", got {hits:?}"
        );
    }

    Ok(())
}
