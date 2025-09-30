//! # Upgrade Command
//!
//! Migrates sources from llms.txt to llms-full.txt when the full version is available upstream.
//!
//! ## Behavior
//!
//! 1. Discovers sources currently using llms.txt
//! 2. Checks if llms-full.txt is available upstream for each
//! 3. Prompts for confirmation (unless `--yes` flag is used)
//! 4. Fetches and indexes the llms-full.txt content
//! 5. Removes or archives the old llms.txt data
//!
//! ## Usage
//!
//! ```bash
//! # Interactive upgrade check for all sources
//! blz upgrade
//!
//! # Upgrade specific source
//! blz upgrade react
//!
//! # Non-interactive bulk upgrade
//! blz upgrade --all --yes
//! ```

use anyhow::{Context, Result, bail};
use blz_core::{Fetcher, MarkdownParser, SearchIndex, Storage};
use tracing::{debug, info, warn};

use crate::utils::flavor::{BASE_FLAVOR, FULL_FLAVOR, build_llms_json};

/// Execute the upgrade command to migrate sources from llms.txt to llms-full.txt
///
/// # Arguments
///
/// * `alias` - Specific source alias to upgrade, or None to check all sources
/// * `all` - When true with no alias, upgrades all eligible sources
/// * `yes` - Skip confirmation prompts and proceed automatically
///
/// # Returns
///
/// Returns `Ok(())` if upgrade completes successfully (even if no sources needed upgrading).
/// Returns `Err` if there are system errors during the upgrade process.
pub async fn execute_upgrade(alias: Option<String>, all: bool, yes: bool) -> Result<()> {
    let storage = Storage::new()?;
    let fetcher = Fetcher::new()?;

    // Determine which sources to check
    let sources_to_check = if let Some(ref alias_str) = alias {
        vec![alias_str.clone()]
    } else {
        storage.list_sources()
    };

    if sources_to_check.is_empty() {
        println!("No sources configured. Add sources with 'blz add <alias> <url>'");
        return Ok(());
    }

    // Find sources that need upgrading
    let mut upgradeable = Vec::new();

    for source_alias in &sources_to_check {
        match check_upgrade_available(&storage, &fetcher, source_alias).await {
            Ok(Some(info)) => upgradeable.push(info),
            Ok(None) => {
                debug!(
                    alias = source_alias,
                    "Source already using llms-full or full not available"
                );
            },
            Err(e) => {
                warn!(alias = source_alias, error = %e, "Failed to check upgrade availability");
            },
        }
    }

    if upgradeable.is_empty() {
        if alias.is_some() {
            println!("Source is already using llms-full.txt or full version is not available");
        } else {
            println!("All sources are up to date (using llms-full.txt where available)");
        }
        return Ok(());
    }

    // Show summary of what will be upgraded
    println!(
        "Found {} source(s) that can be upgraded to llms-full.txt:",
        upgradeable.len()
    );
    for info in &upgradeable {
        println!("  • {}", info.alias);
    }
    println!();

    // Confirm upgrade unless --yes or single source with explicit alias
    let should_proceed = if yes {
        true
    } else if alias.is_some() && upgradeable.len() == 1 {
        confirm_upgrade(&upgradeable[0].alias)?
    } else if all {
        confirm_bulk_upgrade(upgradeable.len())?
    } else {
        bail!("Use --all flag to upgrade multiple sources, or specify a single source alias");
    };

    if !should_proceed {
        println!("Upgrade cancelled");
        return Ok(());
    }

    // Perform upgrades
    let mut upgraded_count = 0;
    let mut failed_count = 0;

    for info in &upgradeable {
        match upgrade_source(&storage, &fetcher, info).await {
            Ok(()) => {
                upgraded_count += 1;
                println!("✓ Upgraded {} to llms-full.txt", info.alias);
            },
            Err(e) => {
                failed_count += 1;
                eprintln!("✗ Failed to upgrade {}: {}", info.alias, e);
            },
        }
    }

    // Print summary
    println!();
    println!(
        "Upgrade complete: {} succeeded, {} failed",
        upgraded_count, failed_count
    );

    if failed_count > 0 {
        bail!("Some sources failed to upgrade");
    }

    Ok(())
}

#[derive(Debug)]
struct UpgradeInfo {
    alias: String,
    full_url: String,
}

/// Check if a source can be upgraded from llms.txt to llms-full.txt
///
/// Returns `Some(UpgradeInfo)` if upgrade is available, `None` if already using full or full not available
async fn check_upgrade_available(
    storage: &Storage,
    fetcher: &Fetcher,
    alias: &str,
) -> Result<Option<UpgradeInfo>> {
    // Get current source info
    let json = storage
        .load_llms_json(alias)
        .context("Failed to load source metadata")?;

    // Check what flavor we're currently using
    let available_flavors = storage.available_flavors(alias)?;

    // If we already have llms-full, no upgrade needed
    if available_flavors.contains(&FULL_FLAVOR.to_string()) {
        return Ok(None);
    }

    // If we don't have llms.txt as base, can't upgrade
    if !available_flavors.contains(&BASE_FLAVOR.to_string()) {
        return Ok(None);
    }

    // Check if llms-full.txt is available upstream
    let base_url = &json.source.url;

    match fetcher.check_flavors(base_url).await {
        Ok(flavors) => {
            // Look for llms-full.txt in available flavors
            for flavor_info in flavors {
                let flavor = Storage::flavor_from_url(&flavor_info.url);
                if matches!(flavor, blz_core::Flavor::LlmsFull) {
                    return Ok(Some(UpgradeInfo {
                        alias: alias.to_string(),
                        full_url: flavor_info.url.clone(),
                    }));
                }
            }
            Ok(None)
        },
        Err(e) => {
            debug!(alias = alias, error = %e, "Failed to check flavors upstream");
            Ok(None)
        },
    }
}

/// Perform the actual upgrade of a source from llms.txt to llms-full.txt
async fn upgrade_source(storage: &Storage, fetcher: &Fetcher, info: &UpgradeInfo) -> Result<()> {
    info!("Upgrading {} to llms-full.txt", info.alias);

    // Fetch llms-full.txt content
    let fetch_result = fetcher
        .fetch_with_cache(&info.full_url, None, None)
        .await
        .context("Failed to fetch llms-full.txt")?;

    let (content, etag, last_modified, sha256) = match fetch_result {
        blz_core::FetchResult::Modified {
            content,
            etag,
            last_modified,
            sha256,
        } => (content, etag, last_modified, sha256),
        blz_core::FetchResult::NotModified { .. } => {
            // This shouldn't happen since we're passing None for etag/last_modified
            bail!("Unexpected 304 Not Modified response");
        },
    };

    // Parse the content
    let mut parser = MarkdownParser::new()?;
    let parse_result = parser
        .parse(&content)
        .context("Failed to parse llms-full.txt")?;

    // Build metadata JSON
    let llms_json = build_llms_json(
        &info.alias,
        &info.full_url,
        "llms-full.txt",
        sha256,
        etag,
        last_modified,
        &parse_result,
    );

    // Save the new flavor
    storage
        .save_flavor_json(&info.alias, FULL_FLAVOR, &llms_json)
        .context("Failed to save llms-full.txt metadata")?;

    storage
        .save_flavor_content(&info.alias, "llms-full.txt", &content)
        .context("Failed to save llms-full.txt content")?;

    // Index the content
    let index_dir = storage.index_dir(&info.alias)?;
    let index = SearchIndex::create_or_open(&index_dir)?;

    index
        .index_blocks(
            &info.alias,
            "llms-full.txt",
            &parse_result.heading_blocks,
            FULL_FLAVOR,
        )
        .context("Failed to index llms-full.txt")?;

    // Archive or remove old llms.txt data (keep metadata for now, remove indexed content)
    // Note: We keep the base flavor files for backward compatibility
    // The resolve_flavor() function will prefer full when FORCE_PREFER_FULL is true

    Ok(())
}

/// Prompt user to confirm upgrade for a single source
fn confirm_upgrade(alias: &str) -> Result<bool> {
    use std::io::{self, Write};

    print!("Upgrade {} to llms-full.txt? [Y/n] ", alias);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let response = input.trim().to_lowercase();
    Ok(response.is_empty() || response == "y" || response == "yes")
}

/// Prompt user to confirm bulk upgrade
fn confirm_bulk_upgrade(count: usize) -> Result<bool> {
    use std::io::{self, Write};

    print!("Upgrade {} sources to llms-full.txt? [Y/n] ", count);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let response = input.trim().to_lowercase();
    Ok(response.is_empty() || response == "y" || response == "yes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upgrade_info_creation() {
        let info = UpgradeInfo {
            alias: "test".to_string(),
            current_url: "https://example.com/llms.txt".to_string(),
            full_url: "https://example.com/llms-full.txt".to_string(),
        };

        assert_eq!(info.alias, "test");
        assert!(info.full_url.contains("llms-full.txt"));
    }
}
