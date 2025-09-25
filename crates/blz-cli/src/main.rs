//! blz CLI - Fast local search for llms.txt documentation
//!
//! This is the main entry point for the blz command-line interface.
//! All command implementations are organized in separate modules for
//! better maintainability and single responsibility.
use anyhow::Result;
use blz_core::PerformanceMetrics;
use clap::{CommandFactory, Parser};
use colored::control as color_control;
use tracing::{Level, warn};
use tracing_subscriber::FmtSubscriber;

use std::collections::BTreeSet;
use std::sync::OnceLock;

mod cli;
mod commands;
mod output;
mod utils;
mod instruct_mod {
    pub fn print() {
        // Embed a simple, agent-friendly text with no special formatting.
        const INSTRUCT: &str = include_str!("../agent-instructions.txt");
        println!("{}", INSTRUCT.trim());
        println!(
            "\nNeed full command reference? Run `blz docs --format markdown` or `blz docs --format json`."
        );
    }
}

use crate::utils::preferences::{self, CliPreferences};
use cli::{AliasCommands, /* AnchorCommands, */ Cli, Commands};

#[cfg(feature = "flamegraph")]
use blz_core::profiling::{start_profiling, stop_profiling_and_report};

/// Preprocess command-line arguments so shorthand search syntax and format aliases work.
///
/// When search-only flags (for example `-s`, `--limit`, `--json`) are used without explicitly
/// writing the `search` subcommand, we inject it and normalise aliases so clap parses them
/// correctly.
fn preprocess_args() -> Vec<String> {
    let raw: Vec<String> = std::env::args().collect();
    preprocess_args_from(&raw)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FlagKind {
    Switch,
    TakesValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SearchFlagMatch {
    None,
    RequiresValue {
        flag: &'static str,
        attached: Option<String>,
    },
    NoValue(&'static str),
    FormatAlias(&'static str),
}

fn preprocess_args_from(raw: &[String]) -> Vec<String> {
    if raw.len() <= 1 {
        return raw.to_vec();
    }

    let mut first_non_global_idx = raw.len();
    let mut search_flag_found = false;
    let mut idx = 1;

    while idx < raw.len() {
        let arg = raw[idx].as_str();
        if arg == "--" {
            break;
        }

        if let Some(kind) = classify_global_flag(arg) {
            if kind == FlagKind::TakesValue && idx + 1 < raw.len() {
                idx += 1;
            }
            idx += 1;
            continue;
        }

        if first_non_global_idx == raw.len() {
            first_non_global_idx = idx;
        }

        if matches!(classify_search_flag(arg), SearchFlagMatch::None) {
            // keep scanning
        } else {
            search_flag_found = true;
        }

        idx += 1;
    }

    if first_non_global_idx == raw.len() && idx < raw.len() {
        first_non_global_idx = idx;
    }

    // Continue scanning from the first non-global argument for additional search flags
    for arg in raw.iter().skip(first_non_global_idx) {
        if arg == "--" {
            break;
        }
        if !matches!(classify_search_flag(arg), SearchFlagMatch::None) {
            search_flag_found = true;
        }
    }

    let explicit_subcommand =
        first_non_global_idx < raw.len() && is_known_subcommand(raw[first_non_global_idx].as_str());
    let mut result = Vec::with_capacity(raw.len() + 4);

    result.push(raw[0].clone());

    // Copy leading global flags so we can insert `search` after them if needed
    for arg in raw.iter().take(first_non_global_idx).skip(1) {
        result.push(arg.clone());
    }

    let should_inject_search = search_flag_found && !explicit_subcommand;
    if should_inject_search {
        result.push("search".to_string());
    }

    let mut idx = first_non_global_idx;
    let mut encountered_sentinel = false;

    while idx < raw.len() {
        let arg = raw[idx].as_str();
        if arg == "--" {
            result.push(raw[idx].clone());
            idx += 1;
            encountered_sentinel = true;
            break;
        }

        match classify_search_flag(arg) {
            SearchFlagMatch::None => {
                result.push(raw[idx].clone());
                idx += 1;
            },
            SearchFlagMatch::NoValue(flag) => {
                result.push(flag.to_string());
                idx += 1;
            },
            SearchFlagMatch::FormatAlias(format) => {
                result.push("--format".to_string());
                result.push(format.to_string());
                idx += 1;
            },
            SearchFlagMatch::RequiresValue { flag, attached } => {
                result.push(flag.to_string());
                if let Some(value) = attached {
                    result.push(value);
                    idx += 1;
                } else if idx + 1 < raw.len() {
                    result.push(raw[idx + 1].clone());
                    idx += 2;
                } else {
                    idx += 1;
                }
            },
        }
    }

    if encountered_sentinel {
        result.extend(raw.iter().skip(idx).cloned());
    }

    result
}

fn is_known_subcommand(value: &str) -> bool {
    known_subcommands().contains(value)
}

const RESERVED_SUBCOMMANDS: &[&str] = &["anchors", "anchor"];

fn known_subcommands() -> &'static BTreeSet<String> {
    static CACHE: OnceLock<BTreeSet<String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut names = BTreeSet::new();
        for sub in Cli::command().get_subcommands() {
            names.insert(sub.get_name().to_owned());
            for alias in sub.get_all_aliases() {
                names.insert(alias.to_owned());
            }
        }
        for extra in RESERVED_SUBCOMMANDS {
            names.insert((*extra).to_owned());
        }
        names
    })
}

fn classify_global_flag(arg: &str) -> Option<FlagKind> {
    match arg {
        "-v" | "--verbose" | "-q" | "--quiet" | "--debug" | "--profile" | "--no-color" | "-h"
        | "--help" | "-V" | "--version" | "--flamegraph" => Some(FlagKind::Switch),
        "--config" | "--config-dir" => Some(FlagKind::TakesValue),
        _ if arg.starts_with("--config=") || arg.starts_with("--config-dir=") => {
            Some(FlagKind::Switch)
        },
        _ => None,
    }
}

fn classify_search_flag(arg: &str) -> SearchFlagMatch {
    match arg {
        "--last" => return SearchFlagMatch::NoValue("--last"),
        "--all" => return SearchFlagMatch::NoValue("--all"),
        "--no-summary" => return SearchFlagMatch::NoValue("--no-summary"),
        "--json" => return SearchFlagMatch::FormatAlias("json"),
        "--jsonl" => return SearchFlagMatch::FormatAlias("jsonl"),
        "--text" => return SearchFlagMatch::FormatAlias("text"),
        _ => {},
    }

    if let Some(value) = arg.strip_prefix("--json=") {
        if !value.is_empty() {
            return SearchFlagMatch::FormatAlias("json");
        }
    }
    if let Some(value) = arg.strip_prefix("--jsonl=") {
        if !value.is_empty() {
            return SearchFlagMatch::FormatAlias("jsonl");
        }
    }
    if let Some(value) = arg.strip_prefix("--text=") {
        if !value.is_empty() {
            return SearchFlagMatch::FormatAlias("text");
        }
    }

    for (flag, canonical) in [
        ("--alias", "--alias"),
        ("--source", "--source"),
        ("--limit", "--limit"),
        ("--page", "--page"),
        ("--top", "--top"),
        ("--format", "--format"),
        ("--output", "--output"),
        ("--show", "--show"),
        ("--flavor", "--flavor"),
        ("--score-precision", "--score-precision"),
        ("--snippet-lines", "--snippet-lines"),
    ] {
        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            return SearchFlagMatch::RequiresValue {
                flag: canonical,
                attached: Some(value.to_string()),
            };
        }
        if arg == flag {
            return SearchFlagMatch::RequiresValue {
                flag: canonical,
                attached: None,
            };
        }
    }

    for (prefix, canonical) in [("-s", "-s"), ("-n", "-n"), ("-f", "-f"), ("-o", "-o")] {
        if arg == prefix {
            return SearchFlagMatch::RequiresValue {
                flag: canonical,
                attached: None,
            };
        }
        if arg.starts_with(prefix) && arg.len() > prefix.len() {
            return SearchFlagMatch::RequiresValue {
                flag: canonical,
                attached: Some(arg[prefix.len()..].to_string()),
            };
        }
    }

    SearchFlagMatch::None
}

#[tokio::main]
async fn main() -> Result<()> {
    // Convert Broken pipe panics into a clean exit
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        if msg.contains("Broken pipe") || msg.contains("broken pipe") {
            // Exit silently for pipeline truncation
            std::process::exit(0);
        }
        // Default behavior: print to stderr
        eprintln!("{msg}");
    }));

    // Spawn process guard as early as possible to catch orphaned processes
    utils::process_guard::spawn_parent_exit_guard();

    // Preprocess arguments to handle shorthand search with flags
    let args = preprocess_args();
    let mut cli = Cli::parse_from(args);

    initialize_logging(&cli)?;

    let args: Vec<String> = std::env::args().collect();
    let mut cli_preferences = preferences::load();
    apply_preference_defaults(&mut cli, &cli_preferences, &args);

    let metrics = PerformanceMetrics::default();

    #[cfg(feature = "flamegraph")]
    let profiler_guard = start_flamegraph_if_requested(&cli);

    execute_command(cli.clone(), metrics.clone(), &mut cli_preferences).await?;

    #[cfg(feature = "flamegraph")]
    stop_flamegraph_if_started(profiler_guard);

    print_diagnostics(&cli, &metrics);

    if let Err(err) = preferences::save(&cli_preferences) {
        warn!("failed to persist CLI preferences: {err}");
    }

    Ok(())
}

fn initialize_logging(cli: &Cli) -> Result<()> {
    // Base level from global flags
    let mut level = if cli.verbose || cli.debug {
        Level::DEBUG
    } else if cli.quiet {
        Level::ERROR
    } else {
        Level::WARN
    };

    // If the selected command is emitting machine-readable output, suppress info logs
    // to keep stdout/stderr clean unless verbose/debug was explicitly requested.
    let mut machine_output = false;
    if !(cli.verbose || cli.debug) {
        let command_format = match &cli.command {
            Some(
                Commands::Search { format, .. }
                | Commands::List { format, .. }
                | Commands::History { format, .. }
                | Commands::Lookup { format, .. }
                | Commands::Get { format, .. }
                | Commands::Completions { format, .. },
            ) => Some(format.resolve(cli.quiet)),
            _ => None,
        };

        if let Some(fmt) = command_format {
            if matches!(
                fmt,
                crate::output::OutputFormat::Json | crate::output::OutputFormat::Jsonl
            ) {
                level = Level::ERROR;
                machine_output = true;
            }
        }
    }

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // Color control: disable when requested, NO_COLOR is set, or when emitting machine output
    let env_no_color = std::env::var("NO_COLOR").ok().is_some();
    if cli.no_color || env_no_color || machine_output {
        color_control::set_override(false);
    }
    Ok(())
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
        if let Err(e) = stop_profiling_and_report(&guard) {
            eprintln!("Failed to generate flamegraph: {e}");
        }
    }
}

async fn execute_command(
    cli: Cli,
    metrics: PerformanceMetrics,
    prefs: &mut CliPreferences,
) -> Result<()> {
    match cli.command {
        Some(Commands::Completions {
            shell,
            list,
            format,
        }) => {
            let resolved_format = format.resolve(cli.quiet);
            if list {
                commands::list_supported(resolved_format);
            } else if let Some(shell) = shell {
                commands::generate(shell);
            } else {
                commands::list_supported(resolved_format);
            }
        },
        Some(Commands::Docs { format }) => handle_docs(format)?,
        Some(Commands::Alias { command }) => handle_alias(command).await?,
        Some(Commands::Instruct) => instruct_mod::print(),
        Some(Commands::Add { alias, url, yes }) => {
            commands::add_source(&alias, &url, yes, cli.quiet, metrics).await?;
        },
        Some(Commands::Lookup { query, format }) => {
            commands::lookup_registry(&query, metrics, cli.quiet, format.resolve(cli.quiet))
                .await?;
        },
        Some(Commands::Search {
            query,
            source,
            last,
            limit,
            all,
            page,
            top,
            format,
            flavor,
            show,
            no_summary,
            score_precision,
            snippet_lines,
        }) => {
            let resolved_format = format.resolve(cli.quiet);
            handle_search(
                query,
                source,
                last,
                limit,
                all,
                page,
                top,
                resolved_format,
                flavor,
                show,
                no_summary,
                score_precision,
                snippet_lines,
                metrics,
                prefs,
            )
            .await?;
        },
        Some(Commands::History { limit, format }) => {
            commands::show_history(prefs, limit, format.resolve(cli.quiet))?;
        },
        Some(Commands::Config { command }) => {
            commands::run_config(command)?;
        },
        Some(Commands::Get {
            alias,
            lines,
            context,
            format,
        }) => commands::get_lines(&alias, &lines, context, format.resolve(cli.quiet)).await?,
        Some(Commands::List { format, status }) => {
            commands::list_sources(format.resolve(cli.quiet), status, cli.quiet).await?;
        },
        Some(Commands::Update {
            alias,
            all,
            flavor,
            yes,
        }) => {
            handle_update(alias, all, metrics, cli.quiet, flavor, yes).await?;
        },
        Some(Commands::Remove { alias, yes }) => {
            commands::remove_source(&alias, yes, cli.quiet).await?;
        },
        Some(Commands::Diff { alias, since }) => {
            commands::show_diff(&alias, since.as_deref()).await?;
        },
        None => commands::handle_default_search(&cli.query, metrics, None, prefs).await?,
    }

    Ok(())
}

fn handle_docs(format: crate::commands::DocsFormat) -> Result<()> {
    // If BLZ_OUTPUT_FORMAT=json and no explicit format set (markdown default), prefer JSON
    let effective = match (std::env::var("BLZ_OUTPUT_FORMAT").ok(), format) {
        (Some(v), crate::commands::DocsFormat::Markdown) if v.eq_ignore_ascii_case("json") => {
            crate::commands::DocsFormat::Json
        },
        _ => format,
    };
    commands::generate_docs(effective)
}

// Anchor commands disabled for v0.2 release
// async fn handle_anchor(command: AnchorCommands) -> Result<()> {
//     match command {
//         AnchorCommands::List {
//             alias,
//             output,
//             mappings,
//         } => commands::show_anchors(&alias, output, mappings).await,
//         AnchorCommands::Get {
//             alias,
//             anchor,
//             context,
//             output,
//         } => commands::get_by_anchor(&alias, &anchor, context, output).await,
//     }
// }

async fn handle_alias(command: AliasCommands) -> Result<()> {
    match command {
        AliasCommands::Add { source, alias } => {
            commands::manage_alias(commands::AliasCommand::Add { source, alias }).await
        },
        AliasCommands::Rm { source, alias } => {
            commands::manage_alias(commands::AliasCommand::Rm { source, alias }).await
        },
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_search(
    query: String,
    source: Option<String>,
    last: bool,
    limit: usize,
    all: bool,
    page: usize,
    top: Option<u8>,
    format: crate::output::OutputFormat,
    flavor: crate::commands::FlavorMode,
    show: Vec<crate::cli::ShowComponent>,
    no_summary: bool,
    score_precision: Option<u8>,
    snippet_lines: u8,
    metrics: PerformanceMetrics,
    prefs: &mut CliPreferences,
) -> Result<()> {
    let actual_limit = if all { 10_000 } else { limit };
    commands::search(
        &query,
        source.as_deref(),
        last,
        actual_limit,
        page,
        top,
        format,
        flavor,
        &show,
        no_summary,
        score_precision,
        snippet_lines,
        Some(prefs),
        metrics,
        None,
    )
    .await
}

async fn handle_update(
    alias: Option<String>,
    all: bool,
    metrics: PerformanceMetrics,
    quiet: bool,
    flavor: crate::commands::FlavorMode,
    yes: bool,
) -> Result<()> {
    if all || alias.is_none() {
        commands::update_all(metrics, quiet, flavor, yes).await
    } else if let Some(alias) = alias {
        commands::update_source(&alias, metrics, quiet, flavor, yes).await
    } else {
        Ok(())
    }
}

fn print_diagnostics(cli: &Cli, metrics: &PerformanceMetrics) {
    if cli.debug {
        metrics.print_summary();
    }
}

fn apply_preference_defaults(cli: &mut Cli, prefs: &CliPreferences, args: &[String]) {
    if let Some(Commands::Search {
        show,
        score_precision,
        snippet_lines,
        ..
    }) = cli.command.as_mut()
    {
        let show_env = std::env::var("BLZ_SHOW").is_ok();
        if show.is_empty() && !flag_present(args, "--show") && !show_env {
            *show = prefs.default_show_components();
        }

        if score_precision.is_none()
            && !flag_present(args, "--score-precision")
            && std::env::var("BLZ_SCORE_PRECISION").is_err()
        {
            *score_precision = Some(prefs.default_score_precision());
        }

        if !flag_present(args, "--snippet-lines") && std::env::var("BLZ_SNIPPET_LINES").is_err() {
            *snippet_lines = prefs.default_snippet_lines();
        }
    }
}

fn flag_present(args: &[String], flag: &str) -> bool {
    let flag_eq = flag;
    let flag_eq_with_equal = format!("{flag}=");
    args.iter()
        .any(|arg| arg == flag_eq || arg.starts_with(&flag_eq_with_equal))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_macros,
    clippy::needless_collect,
    clippy::unnecessary_wraps,
    clippy::deref_addrof
)]
mod tests {
    use super::*;
    use crate::utils::constants::RESERVED_KEYWORDS;
    use crate::utils::parsing::{LineRange, parse_line_ranges};
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
            format,
            flavor,
            ..
        }) = cli.command
        {
            assert_eq!(limit, 50, "Default limit should be 50");
            assert_eq!(page, 1, "Default page should be 1");
            assert!(!all, "Default all should be false");
            assert_eq!(
                format.resolve(false),
                crate::output::OutputFormat::Text,
                "Default format should be text"
            );
            assert!(
                matches!(flavor, crate::commands::FlavorMode::Current),
                "Default flavor should be current"
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
        let format_options = vec![
            ("text", crate::output::OutputFormat::Text),
            ("json", crate::output::OutputFormat::Json),
            ("jsonl", crate::output::OutputFormat::Jsonl),
        ];

        for (format_str, expected_format) in &format_options {
            let cli = Cli::try_parse_from(vec!["blz", "list", "--format", *format_str]).unwrap();

            if let Some(Commands::List { format, .. }) = cli.command {
                assert_eq!(
                    format.resolve(false),
                    *expected_format,
                    "Format should match: {format_str}"
                );
            } else {
                panic!("Expected list command");
            }
        }

        // Alias --output should continue to work for compatibility
        for (format_str, expected_format) in &format_options {
            let cli = Cli::try_parse_from(vec!["blz", "list", "--output", *format_str]).unwrap();

            if let Some(Commands::List { format, .. }) = cli.command {
                assert_eq!(
                    format.resolve(false),
                    *expected_format,
                    "Alias --output should map to {format_str}"
                );
            } else {
                panic!("Expected list command");
            }
        }

        // Test invalid format value
        let result = Cli::try_parse_from(vec!["blz", "list", "--format", "invalid"]);
        assert!(result.is_err(), "Invalid output format should fail");
    }

    fn to_string_vec(items: &[&str]) -> Vec<String> {
        items.iter().copied().map(str::to_owned).collect()
    }

    #[test]
    fn preprocess_injects_search_for_shorthand_flags() {
        use clap::Parser;

        let raw = to_string_vec(&["blz", "query", "-s", "react"]);
        let processed = preprocess_args_from(&raw);

        let expected = to_string_vec(&["blz", "search", "query", "-s", "react"]);
        assert_eq!(processed, expected);

        let cli = Cli::try_parse_from(processed).unwrap();
        match cli.command {
            Some(Commands::Search { source, .. }) => {
                assert_eq!(source.as_deref(), Some("react"));
            },
            _ => panic!("expected search command"),
        }
    }

    #[test]
    fn preprocess_preserves_global_flags_order() {
        let raw = to_string_vec(&["blz", "--quiet", "query", "-s", "docs"]);
        let processed = preprocess_args_from(&raw);
        let expected = to_string_vec(&["blz", "--quiet", "search", "query", "-s", "docs"]);
        assert_eq!(processed, expected);
    }

    #[test]
    fn preprocess_converts_json_aliases() {
        use clap::Parser;

        let raw = to_string_vec(&["blz", "query", "--json"]);
        let processed = preprocess_args_from(&raw);
        let expected = to_string_vec(&["blz", "search", "query", "--format", "json"]);
        assert_eq!(processed, expected);

        let cli = Cli::try_parse_from(processed).unwrap();
        match cli.command {
            Some(Commands::Search { format, .. }) => {
                assert_eq!(format.resolve(false), crate::output::OutputFormat::Json);
            },
            _ => panic!("expected search command"),
        }
    }

    #[test]
    fn preprocess_handles_list_subcommand_without_injection() {
        use clap::Parser;

        let raw = to_string_vec(&["blz", "list", "--jsonl"]);
        let processed = preprocess_args_from(&raw);
        let expected = to_string_vec(&["blz", "list", "--format", "jsonl"]);
        assert_eq!(processed, expected);

        let cli = Cli::try_parse_from(processed).unwrap();
        match cli.command {
            Some(Commands::List { format, .. }) => {
                assert_eq!(format.resolve(false), crate::output::OutputFormat::Jsonl);
            },
            _ => panic!("expected list command"),
        }
    }

    #[test]
    fn preprocess_respects_sentinel() {
        let raw = to_string_vec(&["blz", "query", "--", "-s", "react"]);
        let processed = preprocess_args_from(&raw);
        assert_eq!(processed, raw);
    }

    #[test]
    fn preprocess_does_not_inject_hidden_subcommands() {
        let raw = to_string_vec(&["blz", "anchors", "e2e", "-f", "json"]);
        let processed = preprocess_args_from(&raw);
        assert_eq!(processed, raw);
    }

    #[test]
    fn preprocess_retains_hidden_subcommand_with_search_flags() {
        let raw = to_string_vec(&["blz", "anchors", "e2e", "--limit", "5", "--json"]);
        let processed = preprocess_args_from(&raw);
        let expected =
            to_string_vec(&["blz", "anchors", "e2e", "--limit", "5", "--format", "json"]);
        assert_eq!(
            processed, expected,
            "hidden subcommands must not trigger shorthand injection"
        );
    }

    #[test]
    fn known_subcommands_cover_clap_definitions() {
        use clap::CommandFactory;

        let command = Cli::command();
        for sub in command.get_subcommands() {
            let name = sub.get_name();
            assert!(
                is_known_subcommand(name),
                "expected known subcommand to include {name}"
            );

            for alias in sub.get_all_aliases() {
                assert!(
                    is_known_subcommand(alias),
                    "expected alias {alias} to be recognized"
                );
            }
        }
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
            "10", "--format", "json",
        ])
        .unwrap();

        if let Some(Commands::Search {
            source,
            limit,
            page,
            top,
            format,
            ..
        }) = cli.command
        {
            assert_eq!(source, Some("node".to_string()));
            assert_eq!(limit, 20);
            assert_eq!(page, 2);
            assert!(top.is_some());
            assert_eq!(format.resolve(false), crate::output::OutputFormat::Json);
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
            format,
        }) = cli.command
        {
            assert_eq!(alias, "test");
            assert_eq!(lines, "1-10");
            assert_eq!(context, Some(5));
            let _ = format; // ignore
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
            (vec!["blz", "list", "--format", "invalid"], "invalid"),
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

        let expected_search_flags = vec![
            "source",
            "limit",
            "all",
            "page",
            "top",
            "format",
            "show",
            "no_summary",
        ];
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
        let format_arg = search_cmd
            .get_arguments()
            .find(|arg| arg.get_id().as_str() == "format")
            .expect("Search should have format argument");

        assert!(
            !format_arg.get_possible_values().is_empty(),
            "Format argument should have possible values for completion"
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
                    Some(Commands::Completions { shell: _, .. }) => {
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
