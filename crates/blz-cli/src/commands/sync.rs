//! Sync command implementation - fetch latest documentation content
//!
//! This module provides the `blz sync` command for refreshing documentation sources
//! by fetching the latest content from their URLs.
//!
//! # Examples
//!
//! ```bash
//! blz sync bun                   # Sync single source
//! blz sync --all                 # Sync all sources
//! blz sync bun react             # Sync multiple sources
//! ```

use anyhow::Result;

use blz_core::PerformanceMetrics;

/// Execute the sync command to fetch latest documentation
///
/// This command refreshes documentation sources by fetching the latest content
/// from their configured URLs. It delegates to the internal refresh implementation.
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
        super::refresh::execute_all(metrics, quiet, reindex, filter.as_ref(), no_filter).await
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
        for alias in &aliases {
            super::refresh::execute(
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
