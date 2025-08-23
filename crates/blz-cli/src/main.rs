//! blz CLI - Fast local search for llms.txt documentation
//!
//! This is the main entry point for the blz command-line interface.
//! All command implementations are organized in separate modules for
//! better maintainability and single responsibility.

use anyhow::Result;
use blz_core::{PerformanceMetrics, ResourceMonitor};
use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod cli;
mod commands;
mod output;
mod utils;

use cli::{Cli, Commands};

#[cfg(feature = "flamegraph")]
use blz_core::profiling::{start_profiling, stop_profiling_and_report};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    initialize_logging(&cli)?;

    let metrics = PerformanceMetrics::default();
    let mut resource_monitor = create_resource_monitor(&cli);

    #[cfg(feature = "flamegraph")]
    let profiler_guard = start_flamegraph_if_requested(&cli);

    execute_command(cli.clone(), metrics.clone(), resource_monitor.as_mut()).await?;

    #[cfg(feature = "flamegraph")]
    stop_flamegraph_if_started(profiler_guard);

    print_diagnostics(&cli, &metrics, &mut resource_monitor);

    Ok(())
}

fn initialize_logging(cli: &Cli) -> Result<()> {
    let level = if cli.verbose || cli.debug {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn create_resource_monitor(cli: &Cli) -> Option<ResourceMonitor> {
    if cli.profile {
        Some(ResourceMonitor::new())
    } else {
        None
    }
}

#[cfg(feature = "flamegraph")]
fn start_flamegraph_if_requested(cli: &Cli) -> Option<pprof::ProfilerGuard<'static>> {
    if cli.flamegraph {
        match start_profiling() {
            Ok(guard) => {
                println!("🔥 CPU profiling started - flamegraph will be generated");
                Some(guard)
            },
            Err(e) => {
                eprintln!("Failed to start profiling: {}", e);
                None
            },
        }
    } else {
        None
    }
}

#[cfg(feature = "flamegraph")]
fn stop_flamegraph_if_started(guard: Option<pprof::ProfilerGuard<'static>>) {
    if let Some(guard) = guard {
        if let Err(e) = stop_profiling_and_report(guard) {
            eprintln!("Failed to generate flamegraph: {}", e);
        }
    }
}

async fn execute_command(
    cli: Cli,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) -> Result<()> {
    match cli.command {
        Some(Commands::Completions { shell }) => {
            commands::generate(shell);
        },

        Some(Commands::Add { alias, url, yes }) => {
            commands::add_source(&alias, &url, yes, metrics, resource_monitor).await?;
        },

        Some(Commands::Lookup { query }) => {
            commands::lookup_registry(&query, metrics, resource_monitor).await?;
        },

        Some(Commands::Search {
            query,
            alias,
            limit,
            all,
            page,
            top,
            output,
        }) => {
            let actual_limit = if all { 10000 } else { limit };
            commands::search(
                &query,
                alias.as_deref(),
                actual_limit,
                page,
                top,
                output,
                metrics,
                resource_monitor,
            )
            .await?;
        },

        Some(Commands::Get {
            alias,
            lines,
            context,
        }) => {
            commands::get_lines(&alias, &lines, context).await?;
        },

        Some(Commands::List { output }) => {
            commands::list_sources(output).await?;
        },

        Some(Commands::Update { alias, all }) => {
            if all || alias.is_none() {
                commands::update_all().await?;
            } else if let Some(alias) = alias {
                commands::update_source(&alias).await?;
            }
        },

        Some(Commands::Remove { alias }) => {
            commands::remove_source(&alias).await?;
        },

        Some(Commands::Diff { alias, since }) => {
            commands::show_diff(&alias, since.as_deref()).await?;
        },

        None => {
            // Default search command
            commands::handle_default_search(&cli.args, metrics, resource_monitor).await?;
        },
    }

    Ok(())
}

fn print_diagnostics(
    cli: &Cli,
    metrics: &PerformanceMetrics,
    resource_monitor: &mut Option<ResourceMonitor>,
) {
    if cli.debug {
        metrics.print_summary();
    }

    if cli.profile {
        if let Some(ref mut monitor) = resource_monitor {
            monitor.print_resource_usage();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::constants::RESERVED_KEYWORDS;
    use crate::utils::parsing::{parse_line_ranges, LineRange};
    use crate::utils::validation::validate_alias;
    use std::collections::HashSet;

    #[test]
    fn test_reserved_keywords_validation() {
        for &keyword in RESERVED_KEYWORDS {
            let result = validate_alias(keyword);
            assert!(
                result.is_err(),
                "Reserved keyword '{}' should be rejected",
                keyword
            );

            let error_msg = result.unwrap_err().to_string();
            assert!(
                error_msg.contains(keyword),
                "Error message should contain the reserved keyword '{}'",
                keyword
            );
        }
    }

    #[test]
    fn test_valid_aliases_allowed() {
        let valid_aliases = ["react", "nextjs", "python", "rust", "docs", "api", "guide"];

        for &alias in &valid_aliases {
            let result = validate_alias(alias);
            assert!(result.is_ok(), "Valid alias '{}' should be accepted", alias);
        }
    }

    #[test]
    fn test_language_names_are_not_reserved() {
        let language_names = [
            "node",
            "python",
            "rust",
            "go",
            "java",
            "javascript",
            "typescript",
        ];

        for &lang in &language_names {
            assert!(
                !RESERVED_KEYWORDS.contains(&lang),
                "Language name '{}' should not be reserved",
                lang
            );

            let result = validate_alias(lang);
            assert!(
                result.is_ok(),
                "Language name '{}' should be usable as alias",
                lang
            );
        }
    }

    #[test]
    fn test_reserved_keywords_case_insensitive() {
        let result = validate_alias("ADD");
        assert!(
            result.is_err(),
            "Reserved keyword 'ADD' (uppercase) should be rejected"
        );

        let result = validate_alias("Add");
        assert!(
            result.is_err(),
            "Reserved keyword 'Add' (mixed case) should be rejected"
        );
    }

    #[test]
    fn test_line_range_parsing() {
        // Single line
        let ranges = parse_line_ranges("42").expect("Should parse single line");
        assert_eq!(ranges.len(), 1);
        assert!(matches!(ranges[0], LineRange::Single(42)));

        // Colon range
        let ranges = parse_line_ranges("120:142").expect("Should parse colon range");
        assert_eq!(ranges.len(), 1);
        assert!(matches!(ranges[0], LineRange::Range(120, 142)));

        // Dash range
        let ranges = parse_line_ranges("120-142").expect("Should parse dash range");
        assert_eq!(ranges.len(), 1);
        assert!(matches!(ranges[0], LineRange::Range(120, 142)));

        // Plus syntax
        let ranges = parse_line_ranges("36+20").expect("Should parse plus syntax");
        assert_eq!(ranges.len(), 1);
        assert!(matches!(ranges[0], LineRange::PlusCount(36, 20)));

        // Multiple ranges
        let ranges =
            parse_line_ranges("36:43,120-142,200+10").expect("Should parse multiple ranges");
        assert_eq!(ranges.len(), 3);
    }

    #[test]
    fn test_line_range_parsing_errors() {
        assert!(parse_line_ranges("0").is_err(), "Line 0 should be invalid");
        assert!(
            parse_line_ranges("50:30").is_err(),
            "Backwards range should be invalid"
        );
        assert!(
            parse_line_ranges("50+0").is_err(),
            "Plus zero count should be invalid"
        );
        assert!(
            parse_line_ranges("abc").is_err(),
            "Invalid format should be rejected"
        );
    }

    #[test]
    fn test_reserved_keywords_no_duplicates() {
        let mut seen = HashSet::new();
        for &keyword in RESERVED_KEYWORDS {
            assert!(
                seen.insert(keyword),
                "Reserved keyword '{}' appears multiple times",
                keyword
            );
        }
    }
}
