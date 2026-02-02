//! Refresh command implementation

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Result, anyhow};
use blz_core::numeric::safe_percentage;
use blz_core::refresh::{
    DefaultRefreshIndexer, RefreshContext, RefreshOutcome, RefreshStorage, RefreshUrlResolution,
    refresh_source_with_metadata, reindex_source, resolve_refresh_url,
};
use blz_core::{Fetcher, PerformanceMetrics, Storage};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::config::SyncConfig;
use crate::utils::filter_flags;
use crate::utils::resolver;

fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message(message.to_string());
    pb
}

/// Execute reindex: re-parse and re-index from cached content.
fn execute_reindex(
    storage: &Storage,
    alias: &str,
    metrics: PerformanceMetrics,
    quiet: bool,
    filter: Option<&String>,
    no_filter: bool,
) -> Result<()> {
    let spinner = if quiet {
        ProgressBar::hidden()
    } else {
        create_spinner(format!("Re-indexing {alias}...").as_str())
    };

    let start = Instant::now();

    let existing_metadata = storage.load_metadata(alias).map_err(anyhow::Error::from)?;

    let filter_flags = filter_flags::parse_filter_flags(filter);
    let filter_preference = if no_filter {
        false
    } else if filter_flags.any_enabled() {
        filter_flags.language
    } else {
        existing_metadata.filter_non_english.unwrap_or(true)
    };

    let indexer = DefaultRefreshIndexer;
    let outcome = reindex_source(storage, alias, metrics, &indexer, filter_preference)?;

    spinner.finish_and_clear();

    if !quiet {
        let elapsed = start.elapsed();
        let filtered_count = outcome.filtered;
        if filtered_count > 0 {
            println!(
                "{} {}: {} → {} headings ({:.1}% {}) in {:?}",
                "✓ Re-indexed".green(),
                alias.green(),
                outcome.headings_before,
                outcome.headings_after,
                percentage(filtered_count, outcome.headings_before),
                if filter_preference {
                    "filtered"
                } else {
                    "restored"
                },
                elapsed
            );
        } else {
            println!(
                "{} {}: {} headings in {:?}",
                "✓ Re-indexed".green(),
                alias.green(),
                outcome.headings_after,
                elapsed
            );
        }
    }

    Ok(())
}

fn announce_upgrade(resolution: &RefreshUrlResolution, alias: &str, quiet: bool) {
    if !resolution.upgraded || quiet {
        return;
    }

    println!(
        "{} llms-full.txt is now available for {}",
        "✨".green(),
        alias.green()
    );
    println!(
        "  Upgrading from {} to {}",
        "llms.txt".yellow(),
        "llms-full.txt".green()
    );
}

/// Execute refresh for a specific source.
#[allow(clippy::too_many_lines)]
pub async fn execute(
    alias: &str,
    metrics: PerformanceMetrics,
    quiet: bool,
    reindex: bool,
    filter: Option<&String>,
    no_filter: bool,
) -> Result<()> {
    let storage = Storage::new()?;
    let canonical_alias =
        resolver::resolve_source(&storage, alias)?.unwrap_or_else(|| alias.to_string());

    if !storage.exists(&canonical_alias) {
        return Err(anyhow!("Source '{alias}' not found"));
    }

    if reindex {
        return execute_reindex(
            &storage,
            &canonical_alias,
            metrics,
            quiet,
            filter,
            no_filter,
        );
    }

    let spinner = if quiet {
        ProgressBar::hidden()
    } else {
        create_spinner(format!("Checking {canonical_alias}...").as_str())
    };

    let start = Instant::now();
    let existing_metadata = storage.load_metadata(&canonical_alias)?;
    let existing_aliases = storage.load_llms_aliases(&canonical_alias)?;
    let fetcher = Fetcher::new()?;

    let filter_flags = filter_flags::parse_filter_flags(filter);
    let filter_preference = if no_filter {
        false
    } else if filter_flags.any_enabled() {
        filter_flags.language
    } else {
        existing_metadata.filter_non_english.unwrap_or(true)
    };

    let resolution = resolve_refresh_url(&fetcher, &existing_metadata).await?;
    spinner.finish_and_clear();
    announce_upgrade(&resolution, &canonical_alias, quiet);

    let indexer = DefaultRefreshIndexer;
    let ctx = RefreshContext::new(existing_metadata, existing_aliases, resolution);
    let outcome = refresh_source_with_metadata(
        &storage,
        &fetcher,
        &canonical_alias,
        &ctx,
        metrics,
        &indexer,
        filter_preference,
    )
    .await?;

    if !quiet {
        let elapsed = start.elapsed();
        match outcome {
            RefreshOutcome::Refreshed {
                alias,
                headings,
                lines,
            } => println!(
                "{} {} ({} headings, {} lines) in {:?}",
                "✓ Refreshed".green(),
                alias.green(),
                headings,
                lines,
                elapsed
            ),
            RefreshOutcome::Unchanged { alias } => println!(
                "{} {} (unchanged in {:?})",
                "✓".green(),
                alias.green(),
                elapsed
            ),
        }
    }

    Ok(())
}

/// Execute refresh for all sources.
#[allow(clippy::too_many_lines)]
pub async fn execute_all(metrics: PerformanceMetrics, config: &SyncConfig) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources();

    if sources.is_empty() {
        anyhow::bail!("No sources configured. Use 'blz add' to add sources.");
    }

    if config.reindex {
        let mut updated_count = 0;
        let mut error_count = 0;

        for alias in sources {
            match execute_reindex(
                &storage,
                &alias,
                metrics.clone(),
                config.quiet,
                config.filter.as_ref(),
                config.no_filter,
            ) {
                Ok(()) => {
                    updated_count += 1;
                },
                Err(e) => {
                    if !config.quiet {
                        eprintln!("{}: {}", alias.red(), e);
                    }
                    error_count += 1;
                },
            }
        }

        if !config.quiet {
            println!(
                "\nSummary: {} re-indexed, {} errors",
                updated_count.to_string().green(),
                if error_count > 0 {
                    error_count.to_string().red()
                } else {
                    error_count.to_string().normal()
                }
            );
            metrics.print_summary();
        }

        return Ok(());
    }

    let fetcher = Fetcher::new()?;
    let mut refreshed_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;
    let indexer = DefaultRefreshIndexer;
    let filter_flags = filter_flags::parse_filter_flags(config.filter.as_ref());

    for alias in sources {
        let spinner = if config.quiet {
            ProgressBar::hidden()
        } else {
            create_spinner(format!("Checking {alias}...").as_str())
        };

        let metadata = storage.load_metadata(&alias)?;
        let aliases = storage.load_llms_aliases(&alias)?;

        let filter_preference = if config.no_filter {
            false
        } else if filter_flags.any_enabled() {
            filter_flags.language
        } else {
            metadata.filter_non_english.unwrap_or(true)
        };

        let resolution = resolve_refresh_url(&fetcher, &metadata).await?;
        spinner.finish_and_clear();
        announce_upgrade(&resolution, &alias, config.quiet);

        let ctx = RefreshContext::new(metadata, aliases, resolution);
        match refresh_source_with_metadata(
            &storage,
            &fetcher,
            &alias,
            &ctx,
            metrics.clone(),
            &indexer,
            filter_preference,
        )
        .await
        {
            Ok(RefreshOutcome::Refreshed { .. }) => {
                refreshed_count += 1;
                if !config.quiet {
                    println!("{} {}", "✓ Refreshed".green(), alias.green());
                }
            },
            Ok(RefreshOutcome::Unchanged { .. }) => {
                skipped_count += 1;
                if !config.quiet {
                    println!("{} {} (unchanged)", "✓".green(), alias.green());
                }
            },
            Err(e) => {
                if !config.quiet {
                    eprintln!("{}: {}", alias.red(), e);
                }
                error_count += 1;
            },
        }
    }

    if !config.quiet {
        println!(
            "\nSummary: {} refreshed, {} unchanged, {} errors",
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

fn percentage(part: usize, total: usize) -> f64 {
    safe_percentage(part, total)
}

/// Parameters for the deprecated refresh command.
#[allow(clippy::struct_excessive_bools)]
pub struct DeprecatedRefreshParams {
    /// Aliases to refresh.
    pub aliases: Vec<String>,
    /// Refresh all sources.
    pub all: bool,
    /// Force re-parse and re-index.
    pub reindex: bool,
    /// Content filters to enable.
    pub filter: Option<String>,
    /// Disable all content filters.
    pub no_filter: bool,
    /// Performance metrics collector.
    pub metrics: PerformanceMetrics,
    /// Suppress informational output.
    pub quiet: bool,
}

/// Dispatch a deprecated Refresh command.
///
/// This function handles the deprecated `refresh` command, printing a deprecation
/// warning and delegating to `handle_refresh`.
#[deprecated(since = "1.5.0", note = "use 'sync' instead")]
#[allow(deprecated)]
pub async fn dispatch_deprecated(params: DeprecatedRefreshParams) -> Result<()> {
    let DeprecatedRefreshParams {
        aliases,
        all,
        reindex,
        filter,
        no_filter,
        metrics,
        quiet,
    } = params;

    if !crate::utils::cli_args::deprecation_warnings_suppressed() {
        eprintln!(
            "{}",
            "Warning: 'refresh' is deprecated, use 'sync' instead".yellow()
        );
    }

    let config = SyncConfig::new()
        .with_reindex(reindex)
        .with_filter(filter)
        .with_no_filter(no_filter)
        .with_quiet(quiet);

    handle_refresh(aliases, all, &config, metrics).await
}

/// Dispatch a deprecated Update command.
///
/// This function handles the deprecated `update` command, printing a deprecation
/// warning and delegating to `handle_refresh`.
#[deprecated(since = "1.4.0", note = "use 'sync' instead")]
#[allow(deprecated)]
pub async fn dispatch_update_deprecated(
    aliases: Vec<String>,
    all: bool,
    metrics: PerformanceMetrics,
    quiet: bool,
) -> Result<()> {
    if !crate::utils::cli_args::deprecation_warnings_suppressed() {
        eprintln!(
            "{}",
            "Warning: 'update' is deprecated, use 'refresh' instead".yellow()
        );
    }

    // Update command doesn't support reindex, filter, or no_filter flags
    let config = SyncConfig::new().with_quiet(quiet);
    handle_refresh(aliases, all, &config, metrics).await
}

/// Handle refresh for one or more sources.
///
/// This is the core refresh logic that handles multiple aliases, with fallback
/// to refresh all sources when no aliases are specified and `all` is false.
pub async fn handle_refresh(
    aliases: Vec<String>,
    all: bool,
    config: &SyncConfig,
    metrics: PerformanceMetrics,
) -> Result<()> {
    let mut aliases = aliases;
    let mut filter = config.filter.clone();

    // Handle filter flag as implicit alias when no aliases provided
    if !all && aliases.is_empty() {
        if let Some(raw_value) = filter.take() {
            if crate::utils::filter_flags::is_known_filter_expression(&raw_value) {
                filter = Some(raw_value);
            } else {
                aliases.push(raw_value);
                filter = Some(String::from("all"));
            }
        }
    }

    // Create a config with potentially updated filter
    let resolved_config = SyncConfig {
        filter,
        ..config.clone()
    };

    if all || aliases.is_empty() {
        return execute_all(metrics, &resolved_config).await;
    }

    for alias in aliases {
        let metrics_clone = PerformanceMetrics {
            search_count: Arc::clone(&metrics.search_count),
            total_search_time: Arc::clone(&metrics.total_search_time),
            index_build_count: Arc::clone(&metrics.index_build_count),
            total_index_time: Arc::clone(&metrics.total_index_time),
            bytes_processed: Arc::clone(&metrics.bytes_processed),
            lines_searched: Arc::clone(&metrics.lines_searched),
        };
        execute(
            &alias,
            metrics_clone,
            resolved_config.quiet,
            resolved_config.reindex,
            resolved_config.filter.as_ref(),
            resolved_config.no_filter,
        )
        .await?;
    }

    Ok(())
}
