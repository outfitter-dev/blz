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
                eprintln!("Failed to start profiling: {e}");
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
            eprintln!("Failed to generate flamegraph: {e}");
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
                commands::update_all(metrics, resource_monitor).await?;
            } else if let Some(alias) = alias {
                commands::update_source(&alias, metrics, resource_monitor).await?;
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
                "Reserved keyword '{keyword}' should be rejected"
            );

            let error_msg = result.unwrap_err().to_string();
            assert!(
                error_msg.contains(keyword),
                "Error message should contain the reserved keyword '{keyword}'"
            );
        }
    }

    #[test]
    fn test_valid_aliases_allowed() {
        let valid_aliases = ["react", "nextjs", "python", "rust", "docs", "api", "guide"];

        for &alias in &valid_aliases {
            let result = validate_alias(alias);
            assert!(result.is_ok(), "Valid alias '{alias}' should be accepted");
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
                "Language name '{lang}' should not be reserved"
            );

            let result = validate_alias(lang);
            assert!(
                result.is_ok(),
                "Language name '{lang}' should be usable as alias"
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
                "Reserved keyword '{keyword}' appears multiple times"
            );
        }
    }

    // CLI flag combination and validation tests

    #[test]
    fn test_cli_flag_combinations() {
        use clap::Parser;

        // Test valid flag combinations
        let valid_combinations = vec![
            vec!["blz", "search", "rust", "--limit", "20"],
            vec!["blz", "search", "rust", "--alias", "node", "--limit", "10"],
            vec!["blz", "search", "rust", "--all"],
            vec!["blz", "add", "test", "https://example.com/llms.txt"],
            vec!["blz", "list", "--output", "json"],
            vec!["blz", "update", "--all"],
            vec!["blz", "remove", "test"],
            vec!["blz", "get", "test", "--lines", "1-10"],
            vec!["blz", "lookup", "react"],
        ];

        for combination in valid_combinations {
            let result = Cli::try_parse_from(combination.clone());
            assert!(
                result.is_ok(),
                "Valid combination should parse: {combination:?}"
            );
        }
    }

    #[test]
    fn test_cli_invalid_flag_combinations() {
        use clap::Parser;

        // Test invalid flag combinations that should fail
        let invalid_combinations = vec![
            // Missing required arguments
            vec!["blz", "add", "alias"], // Missing URL
            vec!["blz", "get", "alias"], // Missing --lines argument
            vec!["blz", "search"],       // Missing query
            vec!["blz", "lookup"],       // Missing query
            // Invalid flag values
            vec!["blz", "search", "rust", "--limit", "-5"], // Negative limit
            vec!["blz", "search", "rust", "--page", "-1"],  // Negative page
            vec!["blz", "list", "--output", "invalid"],     // Invalid output format

                                                            // Note: --all with --limit is actually valid (--all sets limit to 10000)
                                                            // Note: update with alias and --all is also valid

                                                            // Add actual invalid combinations here as needed
        ];

        for combination in invalid_combinations {
            let result = Cli::try_parse_from(combination.clone());
            assert!(
                result.is_err(),
                "Invalid combination should fail: {combination:?}"
            );
        }
    }

    #[test]
    fn test_cli_help_generation() {
        use clap::Parser;

        // Test that help can be generated without errors
        let help_commands = vec![
            vec!["blz", "--help"],
            vec!["blz", "search", "--help"],
            vec!["blz", "add", "--help"],
            vec!["blz", "list", "--help"],
            vec!["blz", "get", "--help"],
            vec!["blz", "update", "--help"],
            vec!["blz", "remove", "--help"],
            vec!["blz", "lookup", "--help"],
            vec!["blz", "completions", "--help"],
        ];

        for help_cmd in help_commands {
            let result = Cli::try_parse_from(help_cmd.clone());
            // Help commands should fail parsing but with a specific help error
            if let Err(error) = result {
                assert!(
                    error.kind() == clap::error::ErrorKind::DisplayHelp,
                    "Help command should display help: {help_cmd:?}"
                );
            } else {
                panic!("Help command should not succeed: {help_cmd:?}");
            }
        }
    }

    #[test]
    fn test_cli_version_flag() {
        use clap::Parser;

        let version_commands = vec![vec!["blz", "--version"], vec!["blz", "-V"]];

        for version_cmd in version_commands {
            let result = Cli::try_parse_from(version_cmd.clone());
            // Version commands should fail parsing but with a specific version error
            if let Err(error) = result {
                assert!(
                    error.kind() == clap::error::ErrorKind::DisplayVersion,
                    "Version command should display version: {version_cmd:?}"
                );
            } else {
                panic!("Version command should not succeed: {version_cmd:?}");
            }
        }
    }

    #[test]
    fn test_cli_default_values() {
        use clap::Parser;

        // Test that default values are set correctly
        let cli = Cli::try_parse_from(vec!["blz", "search", "test"]).unwrap();

        if let Some(Commands::Search {
            limit,
            page,
            all,
            output,
            ..
        }) = cli.command
        {
            assert_eq!(limit, 50, "Default limit should be 50");
            assert_eq!(page, 1, "Default page should be 1");
            assert!(!all, "Default all should be false");
            assert_eq!(
                output,
                crate::output::OutputFormat::Text,
                "Default output should be text"
            );
        } else {
            panic!("Expected search command");
        }
    }

    #[test]
    fn test_cli_flag_validation_edge_cases() {
        use clap::Parser;

        // Test edge cases for numeric values
        let edge_cases = vec![
            // Very large values
            vec!["blz", "search", "rust", "--limit", "999999"],
            vec!["blz", "search", "rust", "--page", "999999"],
            // Boundary values
            vec!["blz", "search", "rust", "--limit", "1"],
            vec!["blz", "search", "rust", "--page", "1"],
            // Maximum reasonable values
            vec!["blz", "search", "rust", "--limit", "10000"],
            vec!["blz", "search", "rust", "--page", "1000"],
        ];

        for edge_case in edge_cases {
            let result = Cli::try_parse_from(edge_case.clone());

            // All these should parse successfully (validation happens at runtime)
            assert!(result.is_ok(), "Edge case should parse: {edge_case:?}");
        }
    }

    #[test]
    fn test_cli_string_argument_validation() {
        use clap::Parser;

        // Test various string inputs
        let string_cases = vec![
            // Normal cases
            vec!["blz", "search", "normal query"],
            vec!["blz", "add", "test-alias", "https://example.com/llms.txt"],
            vec!["blz", "lookup", "react"],
            // Edge cases
            vec!["blz", "search", ""], // Empty query (should be handled at runtime)
            vec![
                "blz",
                "search",
                "very-long-query-with-lots-of-words-to-test-limits",
            ],
            vec!["blz", "add", "alias", "file:///local/path.txt"], // File URL
            // Special characters
            vec!["blz", "search", "query with spaces"],
            vec!["blz", "search", "query-with-dashes"],
            vec!["blz", "search", "query_with_underscores"],
            vec!["blz", "add", "test", "https://example.com/path?query=value"],
        ];

        for string_case in string_cases {
            let result = Cli::try_parse_from(string_case.clone());

            // Most string cases should parse (validation happens at runtime)
            assert!(result.is_ok(), "String case should parse: {string_case:?}");
        }
    }

    #[test]
    fn test_cli_output_format_validation() {
        use clap::Parser;

        // Test all valid output formats
        let output_formats = vec![
            ("text", crate::output::OutputFormat::Text),
            ("json", crate::output::OutputFormat::Json),
        ];

        for (format_str, expected_format) in output_formats {
            let cli = Cli::try_parse_from(vec!["blz", "list", "--output", format_str]).unwrap();

            if let Some(Commands::List { output }) = cli.command {
                assert_eq!(
                    output, expected_format,
                    "Output format should match: {format_str}"
                );
            } else {
                panic!("Expected list command");
            }
        }

        // Test invalid output format
        let result = Cli::try_parse_from(vec!["blz", "list", "--output", "invalid"]);
        assert!(result.is_err(), "Invalid output format should fail");
    }

    #[test]
    fn test_cli_boolean_flags() {
        use clap::Parser;

        // Test boolean flags
        let bool_flag_cases = vec![
            // Verbose flag
            (
                vec!["blz", "--verbose", "search", "test"],
                true,
                false,
                false,
            ),
            (vec!["blz", "-v", "search", "test"], true, false, false),
            // Debug flag
            (vec!["blz", "--debug", "search", "test"], false, true, false),
            // Profile flag
            (
                vec!["blz", "--profile", "search", "test"],
                false,
                false,
                true,
            ),
            // Multiple flags
            (
                vec!["blz", "--verbose", "--debug", "--profile", "search", "test"],
                true,
                true,
                true,
            ),
            (
                vec!["blz", "-v", "--debug", "--profile", "search", "test"],
                true,
                true,
                true,
            ),
        ];

        for (args, expected_verbose, expected_debug, expected_profile) in bool_flag_cases {
            let cli = Cli::try_parse_from(args.clone()).unwrap();

            assert_eq!(
                cli.verbose, expected_verbose,
                "Verbose flag mismatch for: {args:?}"
            );
            assert_eq!(
                cli.debug, expected_debug,
                "Debug flag mismatch for: {args:?}"
            );
            assert_eq!(
                cli.profile, expected_profile,
                "Profile flag mismatch for: {args:?}"
            );
        }
    }

    #[test]
    fn test_cli_subcommand_specific_flags() {
        use clap::Parser;

        // Test search-specific flags
        let cli = Cli::try_parse_from(vec![
            "blz", "search", "rust", "--alias", "node", "--limit", "20", "--page", "2", "--top",
            "10", "--output", "json",
        ])
        .unwrap();

        if let Some(Commands::Search {
            alias,
            limit,
            page,
            top,
            output,
            ..
        }) = cli.command
        {
            assert_eq!(alias, Some("node".to_string()));
            assert_eq!(limit, 20);
            assert_eq!(page, 2);
            assert!(top.is_some());
            assert_eq!(output, crate::output::OutputFormat::Json);
        } else {
            panic!("Expected search command");
        }

        // Test add-specific flags
        let cli = Cli::try_parse_from(vec![
            "blz",
            "add",
            "test",
            "https://example.com/llms.txt",
            "--yes",
        ])
        .unwrap();

        if let Some(Commands::Add { alias, url, yes }) = cli.command {
            assert_eq!(alias, "test");
            assert_eq!(url, "https://example.com/llms.txt");
            assert!(yes);
        } else {
            panic!("Expected add command");
        }

        // Test get-specific flags
        let cli = Cli::try_parse_from(vec![
            "blz",
            "get",
            "test",
            "--lines",
            "1-10",
            "--context",
            "5",
        ])
        .unwrap();

        if let Some(Commands::Get {
            alias,
            lines,
            context,
        }) = cli.command
        {
            assert_eq!(alias, "test");
            assert_eq!(lines, "1-10");
            assert_eq!(context, Some(5));
        } else {
            panic!("Expected get command");
        }
    }

    #[test]
    fn test_cli_special_argument_parsing() {
        use clap::Parser;

        // Test line range parsing edge cases
        let line_range_cases = vec![
            "1",
            "1-10",
            "1:10",
            "1+5",
            "10,20,30",
            "1-5,10-15,20+5",
            "100:200",
        ];

        for line_range in line_range_cases {
            let result = Cli::try_parse_from(vec!["blz", "get", "test", "--lines", line_range]);
            assert!(result.is_ok(), "Line range should parse: {line_range}");
        }

        // Test URL parsing for add command
        let url_cases = vec![
            "https://example.com/llms.txt",
            "http://localhost:3000/llms.txt",
            "https://api.example.com/v1/docs/llms.txt",
            "https://example.com/llms.txt?version=1",
            "https://raw.githubusercontent.com/user/repo/main/llms.txt",
        ];

        for url in url_cases {
            let result = Cli::try_parse_from(vec!["blz", "add", "test", url]);
            assert!(result.is_ok(), "URL should parse: {url}");
        }
    }

    #[test]
    fn test_cli_error_messages() {
        use clap::Parser;

        // Test that error messages are informative
        let error_cases = vec![
            // Missing required arguments
            (vec!["blz", "add"], "missing"),
            (vec!["blz", "search"], "required"),
            (vec!["blz", "get", "alias"], "required"),
            // Invalid values
            (vec!["blz", "list", "--output", "invalid"], "invalid"),
        ];

        for (args, expected_error_content) in error_cases {
            let result = Cli::try_parse_from(args.clone());

            assert!(result.is_err(), "Should error for: {args:?}");

            let error_msg = format!("{:?}", result.unwrap_err()).to_lowercase();
            assert!(
                error_msg.contains(expected_error_content),
                "Error message should contain '{expected_error_content}' for args {args:?}, got: {error_msg}"
            );
        }
    }

    #[test]
    fn test_cli_argument_order_independence() {
        use clap::Parser;

        // Test that global flags can appear in different positions
        let equivalent_commands = vec![
            vec![
                vec!["blz", "--verbose", "search", "rust"],
                vec!["blz", "search", "--verbose", "rust"],
            ],
            vec![
                vec!["blz", "--debug", "--profile", "search", "rust"],
                vec!["blz", "search", "rust", "--debug", "--profile"],
                vec!["blz", "--debug", "search", "--profile", "rust"],
            ],
        ];

        for command_group in equivalent_commands {
            let mut parsed_commands = Vec::new();

            for args in &command_group {
                let result = Cli::try_parse_from(args.clone());
                assert!(result.is_ok(), "Should parse: {args:?}");
                parsed_commands.push(result.unwrap());
            }

            // All commands in the group should parse to equivalent structures
            let first = &parsed_commands[0];
            for other in &parsed_commands[1..] {
                assert_eq!(first.verbose, other.verbose, "Verbose flags should match");
                assert_eq!(first.debug, other.debug, "Debug flags should match");
                assert_eq!(first.profile, other.profile, "Profile flags should match");
            }
        }
    }

    // Shell completion generation and accuracy tests

    #[test]
    fn test_completion_generation_for_all_shells() {
        use clap_complete::Shell;

        // Test that completions can be generated for all supported shells without panicking
        let shells = vec![
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Elvish,
        ];

        for shell in shells {
            // Should not panic - this is the main test
            let result = std::panic::catch_unwind(|| {
                crate::commands::generate(shell);
            });

            assert!(
                result.is_ok(),
                "Completion generation should not panic for {shell:?}"
            );
        }
    }

    #[test]
    fn test_completion_cli_structure_contains_all_subcommands() {
        use crate::cli::Cli;
        use clap::CommandFactory;

        // Test that our CLI structure has all expected subcommands (which completions will include)
        let cmd = Cli::command();

        let subcommands: Vec<&str> = cmd.get_subcommands().map(clap::Command::get_name).collect();

        // Verify all main subcommands are present in CLI structure
        let expected_commands = vec![
            "search",
            "add",
            "list",
            "get",
            "update",
            "remove",
            "lookup",
            "diff",
            "completions",
        ];

        for expected_command in expected_commands {
            assert!(
                subcommands.contains(&expected_command),
                "CLI should have '{expected_command}' subcommand for completions"
            );
        }

        // Verify command aliases are configured in CLI structure
        let list_cmd = cmd
            .get_subcommands()
            .find(|sub| sub.get_name() == "list")
            .expect("Should have list command");

        let aliases: Vec<&str> = list_cmd.get_all_aliases().collect();
        assert!(
            aliases.contains(&"sources"),
            "List command should have 'sources' alias"
        );

        let remove_cmd = cmd
            .get_subcommands()
            .find(|sub| sub.get_name() == "remove")
            .expect("Should have remove command");

        let remove_aliases: Vec<&str> = remove_cmd.get_all_aliases().collect();
        assert!(
            remove_aliases.contains(&"rm"),
            "Remove command should have 'rm' alias"
        );
        assert!(
            remove_aliases.contains(&"delete"),
            "Remove command should have 'delete' alias"
        );
    }

    #[test]
    fn test_completion_cli_structure_contains_global_flags() {
        use crate::cli::Cli;
        use clap::CommandFactory;

        // Test that our CLI structure has all expected global flags (which completions will include)
        let cmd = Cli::command();

        let global_args: Vec<&str> = cmd
            .get_arguments()
            .filter(|arg| arg.is_global_set())
            .map(|arg| arg.get_id().as_str())
            .collect();

        // Verify global flags are present in CLI structure
        let expected_global_flags = vec!["verbose", "debug", "profile"];

        for expected_flag in expected_global_flags {
            assert!(
                global_args.contains(&expected_flag),
                "CLI should have global flag '{expected_flag}' for completions"
            );
        }

        // Verify verbose flag properties
        let verbose_arg = cmd
            .get_arguments()
            .find(|arg| arg.get_id().as_str() == "verbose")
            .expect("Should have verbose argument");

        assert!(
            verbose_arg.get_long().is_some(),
            "Verbose should have long form --verbose"
        );
        assert_eq!(
            verbose_arg.get_long(),
            Some("verbose"),
            "Verbose long form should be --verbose"
        );
        assert!(verbose_arg.is_global_set(), "Verbose should be global");
    }

    #[test]
    fn test_completion_cli_structure_contains_subcommand_flags() {
        use crate::cli::Cli;
        use clap::CommandFactory;

        let cmd = Cli::command();

        // Check search command flags
        let search_cmd = cmd
            .get_subcommands()
            .find(|sub| sub.get_name() == "search")
            .expect("Should have search command");

        let search_args: Vec<&str> = search_cmd
            .get_arguments()
            .map(|arg| arg.get_id().as_str())
            .collect();

        let expected_search_flags = vec!["alias", "limit", "all", "page", "top", "output"];
        for expected_flag in expected_search_flags {
            assert!(
                search_args.contains(&expected_flag),
                "Search command should have '{expected_flag}' flag for completions"
            );
        }

        // Check add command flags
        let add_cmd = cmd
            .get_subcommands()
            .find(|sub| sub.get_name() == "add")
            .expect("Should have add command");

        let add_args: Vec<&str> = add_cmd
            .get_arguments()
            .map(|arg| arg.get_id().as_str())
            .collect();

        assert!(
            add_args.contains(&"yes"),
            "Add command should have 'yes' flag"
        );

        // Check get command flags
        let get_cmd = cmd
            .get_subcommands()
            .find(|sub| sub.get_name() == "get")
            .expect("Should have get command");

        let get_args: Vec<&str> = get_cmd
            .get_arguments()
            .map(|arg| arg.get_id().as_str())
            .collect();

        assert!(
            get_args.contains(&"lines"),
            "Get command should have 'lines' flag"
        );
        assert!(
            get_args.contains(&"context"),
            "Get command should have 'context' flag"
        );

        // Check that output argument has value_enum (which provides completion values)
        let output_arg = search_cmd
            .get_arguments()
            .find(|arg| arg.get_id().as_str() == "output")
            .expect("Search should have output argument");

        assert!(
            !output_arg.get_possible_values().is_empty(),
            "Output argument should have possible values for completion"
        );
    }

    #[test]
    fn test_completion_generation_consistency() {
        use clap_complete::Shell;

        // Generate completions multiple times to ensure consistency (no panics)
        let shells_to_test = vec![Shell::Bash, Shell::Zsh, Shell::Fish];

        for shell in shells_to_test {
            // Should not panic on multiple generations
            for _ in 0..3 {
                let result = std::panic::catch_unwind(|| {
                    crate::commands::generate(shell);
                });
                assert!(
                    result.is_ok(),
                    "Completion generation should be consistent for {shell:?}"
                );
            }
        }
    }

    #[test]
    fn test_completion_command_parsing() {
        use clap::Parser;

        // Test that the completions command parses correctly for all shells
        let shell_completions = vec![
            vec!["blz", "completions", "bash"],
            vec!["blz", "completions", "zsh"],
            vec!["blz", "completions", "fish"],
            vec!["blz", "completions", "powershell"],
            vec!["blz", "completions", "elvish"],
        ];

        for args in shell_completions {
            let result = Cli::try_parse_from(args.clone());
            assert!(result.is_ok(), "Completions command should parse: {args:?}");

            if let Ok(cli) = result {
                match cli.command {
                    Some(Commands::Completions { shell: _ }) => {
                        // Expected - completions command parsed successfully
                    },
                    other => {
                        panic!("Expected Completions command, got: {other:?} for args: {args:?}");
                    },
                }
            }
        }
    }

    #[test]
    fn test_completion_invalid_shell_handling() {
        use clap::Parser;

        // Test that invalid shell names are rejected
        let invalid_shells = vec![
            vec!["blz", "completions", "invalid"],
            vec!["blz", "completions", "cmd"],
            vec!["blz", "completions", ""],
            vec!["blz", "completions", "bash_typo"],
            vec!["blz", "completions", "ZSH"], // Wrong case
        ];

        for args in invalid_shells {
            let result = Cli::try_parse_from(args.clone());
            assert!(
                result.is_err(),
                "Invalid shell should be rejected: {args:?}"
            );
        }
    }

    #[test]
    fn test_completion_help_generation() {
        use clap::Parser;

        // Test that help for completions command works
        let help_commands = vec![
            vec!["blz", "completions", "--help"],
            vec!["blz", "completions", "-h"],
        ];

        for help_cmd in help_commands {
            let result = Cli::try_parse_from(help_cmd.clone());

            if let Err(error) = result {
                assert_eq!(
                    error.kind(),
                    clap::error::ErrorKind::DisplayHelp,
                    "Completions help should display help: {help_cmd:?}"
                );

                let help_text = error.to_string();
                assert!(
                    help_text.contains("completions"),
                    "Help text should mention completions"
                );
                assert!(
                    help_text.contains("shell") || help_text.contains("Shell"),
                    "Help text should mention shell parameter"
                );
            } else {
                panic!("Help command should not succeed: {help_cmd:?}");
            }
        }
    }

    #[test]
    fn test_completion_integration_with_clap() {
        use crate::cli::Cli;
        use clap::CommandFactory;

        // Test that our CLI structure is compatible with clap_complete
        let cmd = Cli::command();

        // Verify basic command structure that completion depends on
        assert_eq!(cmd.get_name(), "blz", "Command name should be 'blz'");

        // Verify subcommands are properly configured
        let subcommands: Vec<&str> = cmd.get_subcommands().map(clap::Command::get_name).collect();

        let expected_subcommands = vec![
            "completions",
            "add",
            "lookup",
            "search",
            "get",
            "list",
            "update",
            "remove",
            "diff",
        ];

        for expected in expected_subcommands {
            assert!(
                subcommands.contains(&expected),
                "Command should have subcommand '{expected}', found: {subcommands:?}"
            );
        }

        // Verify completions subcommand has proper shell argument
        let completions_cmd = cmd
            .get_subcommands()
            .find(|sub| sub.get_name() == "completions")
            .expect("Should have completions subcommand");

        let shell_arg = completions_cmd
            .get_arguments()
            .find(|arg| arg.get_id() == "shell")
            .expect("Completions should have shell argument");

        assert!(
            shell_arg.is_positional(),
            "Shell argument should be positional"
        );
    }
}
