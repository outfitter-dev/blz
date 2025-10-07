//! Source validation command - verify source integrity and availability

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use blz_core::Storage;
use colored::Colorize;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::output::OutputFormat;
use crate::utils::resolver;
use crate::utils::staleness::{self, DEFAULT_STALE_AFTER_DAYS};

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub alias: String,
    pub status: ValidationStatus,
    pub url: String,
    pub url_accessible: bool,
    pub url_status_code: Option<u16>,
    pub checksum_matches: bool,
    pub expected_checksum: String,
    pub actual_checksum: Option<String>,
    pub days_since_update: i64,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ValidationStatus {
    Healthy,
    Warning,
    Error,
}

pub async fn execute(alias: Option<String>, all: bool, format: OutputFormat) -> Result<()> {
    let storage = Storage::new()?;

    // Determine which sources to validate
    let sources = if let Some(alias) = alias {
        vec![alias]
    } else if all {
        storage.list_sources()
    } else {
        anyhow::bail!(
            "Specify a source or use --all to validate all sources.\n\
             Examples:\n  \
             blz validate react\n  \
             blz validate --all"
        );
    };

    let mut results = Vec::new();

    for source_alias in &sources {
        let result = validate_source(&storage, source_alias).await?;
        results.push(result);
    }

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&results)?;
            println!("{json}");
        },
        OutputFormat::Jsonl => {
            for result in &results {
                let json = serde_json::to_string(result)?;
                println!("{json}");
            }
        },
        OutputFormat::Text | OutputFormat::Raw => {
            print_text_results(&results);
        },
    }

    // Exit with error code if any sources have errors
    let has_errors = results.iter().any(|r| r.status == ValidationStatus::Error);
    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

async fn validate_source(storage: &Storage, alias: &str) -> Result<ValidationResult> {
    let canonical = resolver::resolve_source(storage, alias)?.unwrap_or_else(|| alias.to_string());

    let metadata = storage
        .load_source_metadata(&canonical)?
        .with_context(|| format!("Source '{alias}' not found"))?;

    let mut issues = Vec::new();
    let mut url_accessible = false;
    let mut url_status_code = None;
    let mut checksum_matches = false;
    let mut actual_checksum = None;

    // Check if URL/file is accessible based on source type
    match &metadata.origin.source_type {
        Some(blz_core::SourceType::LocalFile { path }) => {
            // For local files, check filesystem existence
            if std::path::Path::new(path).exists() {
                url_accessible = true;
                // No HTTP status code for local files
            } else {
                issues.push(format!("Local file not found: {path}"));
            }
        },
        Some(blz_core::SourceType::Remote { url: _ }) | None => {
            // For remote sources (or when source_type is not set), check HTTP accessibility
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()?;

            let check_url = metadata
                .origin
                .source_type
                .as_ref()
                .and_then(|st| {
                    if let blz_core::SourceType::Remote { url } = st {
                        Some(url.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or(&metadata.url);

            match client.head(check_url).send().await {
                Ok(response) => {
                    url_accessible = response.status().is_success();
                    url_status_code = Some(response.status().as_u16());

                    if !url_accessible {
                        issues.push(format!("URL returned status code {}", response.status()));
                    }
                },
                Err(e) => {
                    issues.push(format!("Failed to connect to URL: {e}"));
                },
            }
        },
    }

    // Verify SHA-256 checksum
    let llms_path = storage.llms_txt_path(&canonical)?;
    if llms_path.exists() {
        let content = tokio::fs::read(&llms_path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&content);

        // Convert to base64 to match metadata format
        let checksum = STANDARD.encode(hasher.finalize());

        actual_checksum = Some(checksum.clone());
        checksum_matches = checksum == metadata.sha256;

        if !checksum_matches {
            issues.push("Checksum mismatch - file may be corrupted".to_string());
        }
    } else {
        issues.push("Local file not found".to_string());
    }

    // Check staleness
    let days_since_update = staleness::days_since(metadata.fetched_at);

    if staleness::is_stale(metadata.fetched_at, DEFAULT_STALE_AFTER_DAYS) {
        issues.push(format!(
            "Source is stale ({days_since_update} days since update)"
        ));
    }

    // Determine overall status
    let status = if issues.is_empty() {
        ValidationStatus::Healthy
    } else if url_accessible && checksum_matches {
        ValidationStatus::Warning
    } else {
        ValidationStatus::Error
    };

    Ok(ValidationResult {
        alias: canonical,
        status,
        url: metadata.url,
        url_accessible,
        url_status_code,
        checksum_matches,
        expected_checksum: metadata.sha256,
        actual_checksum,
        days_since_update,
        issues,
    })
}

fn print_text_results(results: &[ValidationResult]) {
    for result in results {
        let status_icon = match result.status {
            ValidationStatus::Healthy => "✓".green(),
            ValidationStatus::Warning => "⚠".yellow(),
            ValidationStatus::Error => "✗".red(),
        };

        println!("\n{status_icon} {}", result.alias.bold());
        println!("  URL: {}", result.url);

        if let Some(code) = result.url_status_code {
            let code_str = if result.url_accessible {
                format!("{code}").green()
            } else {
                format!("{code}").red()
            };
            println!("  Status: {code_str}");
        }

        if let Some(ref actual) = result.actual_checksum {
            let checksum_str = if result.checksum_matches {
                "matches".green()
            } else {
                "MISMATCH".red()
            };
            println!("  Checksum: {checksum_str}");
            if !result.checksum_matches {
                println!("    Expected: {}", result.expected_checksum);
                println!("    Actual:   {actual}");
            }
        }

        println!("  Last updated: {} days ago", result.days_since_update);

        if !result.issues.is_empty() {
            println!("  Issues:");
            for issue in &result.issues {
                println!("    • {}", issue.yellow());
            }
        }
    }

    // Summary
    let healthy = results
        .iter()
        .filter(|r| r.status == ValidationStatus::Healthy)
        .count();
    let warning = results
        .iter()
        .filter(|r| r.status == ValidationStatus::Warning)
        .count();
    let error = results
        .iter()
        .filter(|r| r.status == ValidationStatus::Error)
        .count();

    println!("\n{}", "Summary:".bold());
    println!(
        "  {} healthy, {} warnings, {} errors",
        healthy.to_string().green(),
        warning.to_string().yellow(),
        error.to_string().red()
    );
}
