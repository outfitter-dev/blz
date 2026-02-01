//! Refresh command implementation

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Result, anyhow};
use blz_core::numeric::safe_percentage;
use blz_core::refresh::{
    DefaultRefreshIndexer, RefreshOutcome, RefreshStorage, RefreshUrlResolution,
    refresh_source_with_metadata, reindex_source, resolve_refresh_url,
};
use blz_core::{Fetcher, PerformanceMetrics, Storage};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

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
    let outcome = refresh_source_with_metadata(
        &storage,
        &fetcher,
        &canonical_alias,
        existing_metadata,
        existing_aliases,
        &resolution,
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
pub async fn execute_all(
    metrics: PerformanceMetrics,
    quiet: bool,
    reindex: bool,
    filter: Option<&String>,
    no_filter: bool,
) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources();

    if sources.is_empty() {
        anyhow::bail!("No sources configured. Use 'blz add' to add sources.");
    }

    if reindex {
        let mut updated_count = 0;
        let mut error_count = 0;

        for alias in sources {
            match execute_reindex(&storage, &alias, metrics.clone(), quiet, filter, no_filter) {
                Ok(()) => {
                    updated_count += 1;
                },
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
    let filter_flags = filter_flags::parse_filter_flags(filter);

    for alias in sources {
        let spinner = if quiet {
            ProgressBar::hidden()
        } else {
            create_spinner(format!("Checking {alias}...").as_str())
        };

        let metadata = storage.load_metadata(&alias)?;
        let aliases = storage.load_llms_aliases(&alias)?;

        let filter_preference = if no_filter {
            false
        } else if filter_flags.any_enabled() {
            filter_flags.language
        } else {
            metadata.filter_non_english.unwrap_or(true)
        };

        let resolution = resolve_refresh_url(&fetcher, &metadata).await?;
        spinner.finish_and_clear();
        announce_upgrade(&resolution, &alias, quiet);

        match refresh_source_with_metadata(
            &storage,
            &fetcher,
            &alias,
            metadata,
            aliases,
            &resolution,
            metrics.clone(),
            &indexer,
            filter_preference,
        )
        .await
        {
            Ok(RefreshOutcome::Refreshed { .. }) => {
                refreshed_count += 1;
                if !quiet {
                    println!("{} {}", "✓ Refreshed".green(), alias.green());
                }
            },
            Ok(RefreshOutcome::Unchanged { .. }) => {
                skipped_count += 1;
                if !quiet {
                    println!("{} {} (unchanged)", "✓".green(), alias.green());
                }
            },
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

/// Dispatch a deprecated Refresh command.
///
/// This function handles the deprecated `refresh` command, printing a deprecation
/// warning and delegating to `handle_refresh`.
#[deprecated(since = "1.5.0", note = "use 'sync' instead")]
#[allow(deprecated, clippy::fn_params_excessive_bools)]
pub async fn dispatch_deprecated(
    aliases: Vec<String>,
    all: bool,
    reindex: bool,
    filter: Option<String>,
    no_filter: bool,
    metrics: PerformanceMetrics,
    quiet: bool,
) -> Result<()> {
    if !crate::utils::cli_args::deprecation_warnings_suppressed() {
        eprintln!(
            "{}",
            "Warning: 'refresh' is deprecated, use 'sync' instead".yellow()
        );
    }

    handle_refresh(aliases, all, reindex, filter, no_filter, metrics, quiet).await
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
    handle_refresh(aliases, all, false, None, false, metrics, quiet).await
}

/// Handle refresh for one or more sources.
///
/// This is the core refresh logic that handles multiple aliases, with fallback
/// to refresh all sources when no aliases are specified and `all` is false.
#[allow(clippy::fn_params_excessive_bools)]
pub async fn handle_refresh(
    aliases: Vec<String>,
    all: bool,
    reindex: bool,
    filter: Option<String>,
    no_filter: bool,
    metrics: PerformanceMetrics,
    quiet: bool,
) -> Result<()> {
    let mut aliases = aliases;
    let mut filter = filter;

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

    if all || aliases.is_empty() {
        return execute_all(metrics, quiet, reindex, filter.as_ref(), no_filter).await;
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
            quiet,
            reindex,
            filter.as_ref(),
            no_filter,
        )
        .await?;
    }

    Ok(())
}
