//! Sync command implementation - fetch latest documentation content
//!
//! This module provides the `blz sync` command for refreshing documentation sources
//! by fetching the latest content from their URLs.
//!
//! # Generated Source Optimization
//!
//! For sources created via `blz generate` (which have a `generate.json` manifest),
//! sync uses sitemap lastmod timestamps to skip unchanged pages:
//!
//! 1. Detect: source has `generate.json` -> generated source
//! 2. Fetch `sitemap.xml` (FREE - direct HTTP)
//! 3. Compare each URL's lastmod vs cached `sitemap_lastmod`
//! 4. Skip unchanged pages (FREE!)
//! 5. Scrape only new/changed pages (costs credits)
//! 6. Retry failed pages from previous sync
//! 7. Re-assemble with updated pages
//!
//! # Examples
//!
//! ```bash
//! blz sync bun                   # Sync single source
//! blz sync --all                 # Sync all sources
//! blz sync bun react             # Sync multiple sources
//! ```

pub mod generated;

use anyhow::Result;
use blz_core::{PerformanceMetrics, Storage};
use colored::Colorize;

use crate::utils::resolver;

// Re-export generated source types and functions for public API.
// Some are not yet used internally but are exported for future Firecrawl integration.
#[allow(unused_imports)]
pub use generated::{
    FailedPage, GenerateManifest, PageCacheEntry, UrlWithLastmod, categorize_sync_pages,
    is_generated_source, load_generate_manifest,
};

// These functions are available via the `generated` module for direct use:
// - pages_needing_update: Determine which pages need re-scraping
// - pages_to_retry: Get failed pages for retry
// - save_generate_manifest: Persist updated manifest
// - should_scrape: Compare cached vs sitemap lastmod

/// Execute the sync command to fetch latest documentation
///
/// This command refreshes documentation sources by fetching the latest content
/// from their configured URLs. For generated sources (created via `blz generate`),
/// it uses sitemap lastmod timestamps to skip unchanged pages.
///
/// # Arguments
///
/// * `aliases` - Source aliases to sync
/// * `all` - Sync all sources
/// * `yes` - Skip confirmation prompts (reserved for future use)
/// * `reindex` - Force re-parse and re-index even if content unchanged
/// * `filter` - Content filters to enable (comma-separated)
/// * `no_filter` - Disable all content filters
/// * `metrics` - Performance metrics collector
/// * `quiet` - Suppress informational output
#[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
pub async fn execute(
    aliases: Vec<String>,
    all: bool,
    _yes: bool,
    reindex: bool,
    filter: Option<String>,
    no_filter: bool,
    metrics: PerformanceMetrics,
    quiet: bool,
) -> Result<()> {
    if all {
        execute_all(metrics, quiet, reindex, filter, no_filter).await
    } else if aliases.is_empty() {
        // No aliases and no --all: error out
        anyhow::bail!(
            "No source specified.\n\n\
             Usage:\n  \
             blz sync <alias>      # Sync specific source\n  \
             blz sync --all        # Sync all sources"
        );
    } else {
        // Sync specified aliases
        let storage = Storage::new()?;
        for alias in &aliases {
            execute_single(
                &storage,
                alias,
                metrics.clone(),
                quiet,
                reindex,
                filter.as_ref(),
                no_filter,
            )
            .await?;
        }
        Ok(())
    }
}

/// Execute sync for all sources.
async fn execute_all(
    metrics: PerformanceMetrics,
    quiet: bool,
    reindex: bool,
    filter: Option<String>,
    no_filter: bool,
) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources();

    if sources.is_empty() {
        anyhow::bail!("No sources configured. Use 'blz add' to add sources.");
    }

    let mut refreshed_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    for alias in sources {
        match execute_single(
            &storage,
            &alias,
            metrics.clone(),
            quiet,
            reindex,
            filter.as_ref(),
            no_filter,
        )
        .await
        {
            Ok(true) => refreshed_count += 1,
            Ok(false) => skipped_count += 1,
            Err(e) => {
                if !quiet {
                    eprintln!("{}: {}", alias.red(), e);
                }
                error_count += 1;
            },
        }
    }

    if !quiet {
        println!(
            "\nSummary: {} synced, {} unchanged, {} errors",
            refreshed_count.to_string().green(),
            skipped_count,
            if error_count > 0 {
                error_count.to_string().red()
            } else {
                error_count.to_string().normal()
            }
        );
        metrics.print_summary();
    }

    Ok(())
}

/// Execute sync for a single source.
///
/// Returns `Ok(true)` if the source was updated, `Ok(false)` if unchanged.
async fn execute_single(
    storage: &Storage,
    alias: &str,
    metrics: PerformanceMetrics,
    quiet: bool,
    reindex: bool,
    filter: Option<&String>,
    no_filter: bool,
) -> Result<bool> {
    let canonical_alias =
        resolver::resolve_source(storage, alias)?.unwrap_or_else(|| alias.to_string());

    if !storage.exists(&canonical_alias) {
        anyhow::bail!("Source '{alias}' not found");
    }

    // Check if this is a generated source
    if is_generated_source(storage, &canonical_alias) {
        // Generated source: use lastmod-based sync
        sync_generated_source(storage, &canonical_alias, quiet).await
    } else {
        // Standard source: use existing refresh flow
        super::refresh::execute(&canonical_alias, metrics, quiet, reindex, filter, no_filter)
            .await?;
        Ok(true) // Assume updated for now
    }
}

/// Sync a generated source using sitemap lastmod optimization.
///
/// This function:
/// 1. Loads the generate manifest
/// 2. Fetches the sitemap
/// 3. Compares lastmod timestamps
/// 4. Reports which pages need updates (actual scraping deferred)
///
/// Returns `Ok(true)` if updates are needed, `Ok(false)` if unchanged.
async fn sync_generated_source(storage: &Storage, alias: &str, quiet: bool) -> Result<bool> {
    use blz_core::discovery::fetch_sitemap;

    if !quiet {
        println!("Syncing {} {}...", alias.green(), "(generated)".dimmed());
    }

    // Load manifest
    let manifest = load_generate_manifest(storage, alias)?;

    // Fetch current sitemap
    if !quiet {
        println!("  Fetching sitemap...");
    }
    let sitemap_entries = fetch_sitemap(&manifest.sitemap_url).await?;

    if !quiet {
        println!("  {} URLs in sitemap", sitemap_entries.len());
    }

    // Compare with cached pages
    let (unchanged, updates, retries) =
        categorize_sync_pages(&manifest.pages, &sitemap_entries, &manifest.failed);

    let total_to_scrape = updates.len() + retries.len();

    if !quiet {
        println!("  Comparing with cache...");
        println!(
            "    {} unchanged (skipping)",
            format!("{unchanged}").dimmed()
        );
        if !updates.is_empty() {
            println!("    {} updated (will scrape)", updates.len());
        }
        if !retries.is_empty() {
            println!("    {} previously failed (retrying)", retries.len());
        }
    }

    if total_to_scrape == 0 {
        if !quiet {
            println!("{} {} (unchanged)", "".green(), alias.green());
        }
        return Ok(false);
    }

    // Report what would be done (actual scraping requires Firecrawl integration)
    if !quiet {
        println!(
            "\n{} {} pages would be scraped",
            "Note:".yellow(),
            total_to_scrape
        );
        println!("  Full scraping requires Firecrawl CLI integration (coming soon)");
        println!("  For now, use 'blz generate' to regenerate with updated content");
    }

    // Return true to indicate updates are available
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports_generated_types() {
        // Verify types are properly exported
        let _: fn() -> GenerateManifest =
            || GenerateManifest::new("https://example.com".to_string());
        let _: fn() -> PageCacheEntry =
            || PageCacheEntry::new("https://example.com".to_string(), "content".to_string());
        let _: fn() -> FailedPage =
            || FailedPage::new("https://example.com".to_string(), "error".to_string());
        let _: fn() -> UrlWithLastmod = || UrlWithLastmod::new("https://example.com".to_string());
    }
}
