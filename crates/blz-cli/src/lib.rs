//! blz CLI - Fast local search for llms.txt documentation
//!
//! This is the main entry point for the blz command-line interface.
//! All command implementations are organized in separate modules for
//! better maintainability and single responsibility.
use anyhow::{Result, anyhow};
use blz_core::profiling::ResourceMonitor;
use blz_core::{PerformanceMetrics, Storage};
use clap::{CommandFactory, Parser};
use colored::Colorize;
use tracing::warn;

use std::sync::Arc;

pub mod args;
mod cli;
mod commands;
pub mod config;
pub mod error;
pub mod generate;
mod output;
mod prompt;
mod utils;

use crate::commands::{
    AddRequest, BUNDLED_ALIAS, DescriptorInput, DocsSyncStatus, RequestSpec, print_full_content,
    print_overview, sync_bundled_docs,
};

use crate::utils::initialize_logging;
use crate::utils::preferences::{self, CliPreferences};
#[cfg(feature = "flamegraph")]
use crate::utils::{start_flamegraph_if_requested, stop_flamegraph_if_started};
use cli::{
    AliasCommands, AnchorCommands, ClaudePluginCommands, Cli, Commands, DocsCommands,
    DocsSearchArgs, RegistryCommands,
};

/// Execute the blz CLI with the currently configured environment.
///
/// # Errors
///
/// Returns an error if CLI initialization, prompt emission, or command execution fails.
pub async fn run() -> Result<()> {
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

    let mut cli = Cli::parse();

    if let Some(target) = cli.prompt.clone() {
        prompt::emit(&target, cli.command.as_ref())?;
        return Ok(());
    }

    initialize_logging(&cli)?;

    let args: Vec<String> = std::env::args().collect();
    let mut cli_preferences = preferences::load();
    apply_preference_defaults(&mut cli, &cli_preferences, &args);

    let metrics = PerformanceMetrics::default();

    // Create resource monitor early to capture baseline memory if profiling requested
    let mut resource_monitor = if cli.profile {
        Some(ResourceMonitor::new())
    } else {
        None
    };

    #[cfg(feature = "flamegraph")]
    let profiler_guard = start_flamegraph_if_requested(&cli);

    execute_command(cli.clone(), metrics.clone(), &mut cli_preferences).await?;

    #[cfg(feature = "flamegraph")]
    stop_flamegraph_if_started(profiler_guard);

    print_diagnostics(&cli, &metrics, resource_monitor.as_mut());

    if let Err(err) = preferences::save(&cli_preferences) {
        warn!("failed to persist CLI preferences: {err}");
    }

    Ok(())
}

async fn execute_command(
    cli: Cli,
    metrics: PerformanceMetrics,
    prefs: &mut CliPreferences,
) -> Result<()> {
    let quiet = cli.quiet;
    match cli.command {
        Some(Commands::Instruct) => {
            prompt::emit("__global__", Some(&Commands::Instruct))?;
            eprintln!("`blz instruct` is deprecated. Use `blz --prompt` instead.");
        },
        Some(Commands::Completions {
            shell,
            list,
            format,
        }) => {
            handle_completions(shell, list, format.resolve(quiet));
        },
        Some(Commands::Docs { command }) => {
            handle_docs(command, quiet, metrics.clone(), prefs).await?;
        },
        Some(Commands::ClaudePlugin { command }) => handle_claude_plugin(command)?,
        Some(Commands::Alias { command }) => handle_alias(command).await?,
        Some(Commands::Add(args)) => handle_add(args, quiet, metrics).await?,
        Some(Commands::Lookup {
            query,
            format,
            limit,
        }) => {
            dispatch_lookup(query, format, limit, quiet, metrics).await?;
        },
        Some(Commands::Registry { command }) => handle_registry(command, quiet, metrics).await?,
        Some(cmd @ Commands::Search { .. }) => dispatch_search(cmd, quiet, metrics, prefs).await?,
        Some(Commands::History {
            limit,
            format,
            clear,
            clear_before,
        }) => {
            dispatch_history(limit, &format, clear, clear_before.as_deref(), quiet, prefs)?;
        },
        Some(cmd @ Commands::Get { .. }) => dispatch_get(cmd, quiet).await?,
        Some(Commands::Query(args)) => handle_query(args, quiet, prefs, metrics.clone()).await?,
        Some(Commands::Map(args)) => dispatch_map(args, quiet).await?,
        Some(Commands::Sync(args)) => dispatch_sync(args, quiet, metrics).await?,
        Some(Commands::Check(args)) => {
            commands::check_source(args.alias, args.all, args.format.resolve(quiet)).await?;
        },
        Some(Commands::Rm(args)) => commands::rm_source(vec![args.alias], args.yes).await?,
        #[allow(deprecated)]
        Some(cmd @ Commands::Find { .. }) => {
            dispatch_find(cmd, quiet, prefs, metrics.clone()).await?;
        },
        Some(Commands::Info { alias, format }) => {
            commands::execute_info(&alias, format.resolve(quiet)).await?;
        },
        Some(Commands::List {
            format,
            status,
            details,
            limit,
        }) => {
            dispatch_list(format, status, details, limit, quiet).await?;
        },
        Some(Commands::Stats { format, limit }) => {
            commands::show_stats(format.resolve(quiet), limit)?;
        },
        #[allow(deprecated)]
        Some(Commands::Validate { alias, all, format }) => {
            dispatch_validate(alias, all, format, quiet).await?;
        },
        Some(Commands::Doctor { format, fix }) => {
            commands::run_doctor(format.resolve(quiet), fix).await?;
        },
        #[allow(deprecated)]
        Some(cmd @ Commands::Refresh { .. }) => dispatch_refresh(cmd, quiet, metrics).await?,
        #[allow(deprecated)]
        Some(cmd @ Commands::Update { .. }) => dispatch_update(cmd, quiet, metrics).await?,
        #[allow(deprecated)]
        Some(Commands::Remove { alias, yes }) => dispatch_remove(alias, yes, quiet).await?,
        Some(Commands::Clear { force }) => commands::clear_cache(force)?,
        Some(Commands::Diff { alias, since }) => {
            commands::show_diff(&alias, since.as_deref()).await?;
        },
        Some(Commands::McpServer) => commands::mcp_server().await?,
        Some(Commands::Anchor { command }) => handle_anchor(command, quiet).await?,
        #[allow(deprecated)]
        Some(cmd @ Commands::Toc { .. }) => dispatch_toc(cmd, quiet).await?,
        None => {
            // No subcommand provided - show help
            Cli::command().print_help()?;
        },
    }
    Ok(())
}

/// Dispatch a Search command variant, handling destructuring internally.
///
/// This function extracts all fields from the `Commands::Search` variant and
/// forwards them to the underlying `handle_search` implementation.
#[allow(clippy::too_many_lines)]
async fn dispatch_search(
    cmd: Commands,
    quiet: bool,
    metrics: PerformanceMetrics,
    prefs: &mut CliPreferences,
) -> Result<()> {
    let Commands::Search(args) = cmd else {
        unreachable!("dispatch_search called with non-Search command");
    };

    let resolved_format = args.format.resolve(quiet);
    let merged_context = crate::cli::merge_context_flags(
        args.context,
        args.context_deprecated,
        args.after_context,
        args.before_context,
    );

    handle_search(
        args.query,
        args.sources,
        args.next,
        args.previous,
        args.last,
        args.limit,
        args.all,
        args.page,
        args.top,
        args.heading_level,
        resolved_format,
        args.show,
        args.no_summary,
        args.score_precision,
        args.snippet_lines,
        args.max_chars,
        merged_context,
        args.block,
        args.max_lines,
        args.headings_only,
        args.no_history,
        args.copy,
        args.timing,
        quiet,
        metrics,
        prefs,
    )
    .await
}

/// Dispatch a Get command variant, handling destructuring internally.
#[allow(clippy::too_many_lines)]
async fn dispatch_get(cmd: Commands, quiet: bool) -> Result<()> {
    let Commands::Get {
        targets,
        lines,
        source,
        context,
        context_deprecated,
        after_context,
        before_context,
        block,
        max_lines,
        format,
        copy,
    } = cmd
    else {
        unreachable!("dispatch_get called with non-Get command");
    };

    handle_get(
        targets,
        lines,
        source,
        context,
        context_deprecated,
        after_context,
        before_context,
        block,
        max_lines,
        format.resolve(quiet),
        copy,
    )
    .await
}

/// Dispatch a Find command variant, handling destructuring internally.
#[allow(clippy::too_many_lines)]
async fn dispatch_find(
    cmd: Commands,
    quiet: bool,
    prefs: &mut CliPreferences,
    metrics: PerformanceMetrics,
) -> Result<()> {
    #[allow(deprecated)]
    let Commands::Find(args) = cmd else {
        unreachable!("dispatch_find called with non-Find command");
    };

    handle_find(
        args.inputs,
        args.sources,
        args.limit,
        args.all,
        args.page,
        args.top,
        args.heading_level,
        args.format.resolve(quiet),
        args.show,
        args.no_summary,
        args.score_precision,
        args.snippet_lines,
        args.max_chars,
        args.context,
        args.context_deprecated,
        args.after_context,
        args.before_context,
        args.block,
        args.max_lines,
        args.headings_only,
        args.no_history,
        args.copy,
        args.timing,
        quiet,
        prefs,
        metrics,
    )
    .await
}

/// Dispatch a Toc command variant, handling destructuring internally.
#[allow(deprecated)]
async fn dispatch_toc(cmd: Commands, quiet: bool) -> Result<()> {
    let Commands::Toc(args) = cmd else {
        unreachable!("dispatch_toc called with non-Toc command");
    };

    handle_toc(
        args.alias,
        args.sources,
        args.all,
        args.format.resolve(quiet),
        args.anchors,
        args.show_anchors,
        args.limit,
        args.max_depth,
        args.heading_level,
        args.filter,
        args.tree,
        args.next,
        args.previous,
        args.last,
        args.page,
    )
    .await
}

/// Dispatch a Refresh command variant, handling destructuring internally.
#[allow(deprecated)]
async fn dispatch_refresh(cmd: Commands, quiet: bool, metrics: PerformanceMetrics) -> Result<()> {
    let Commands::Refresh {
        aliases,
        all,
        yes: _,
        reindex,
        filter,
        no_filter,
    } = cmd
    else {
        unreachable!("dispatch_refresh called with non-Refresh command");
    };

    if !utils::cli_args::deprecation_warnings_suppressed() {
        eprintln!(
            "{}",
            "Warning: 'refresh' is deprecated, use 'sync' instead".yellow()
        );
    }

    handle_refresh(aliases, all, reindex, filter, no_filter, metrics, quiet).await
}

/// Dispatch a Map command.
async fn dispatch_map(args: crate::cli::MapArgs, quiet: bool) -> Result<()> {
    commands::show_map(
        args.alias.as_deref(),
        &args.sources,
        args.all,
        args.format.resolve(quiet),
        args.anchors,
        args.show_anchors,
        args.limit,
        args.max_depth,
        args.heading_level.as_ref(),
        args.filter.as_deref(),
        args.tree,
        args.next,
        args.previous,
        args.last,
        args.page,
    )
    .await
}

/// Dispatch a Sync command.
async fn dispatch_sync(
    args: crate::cli::SyncArgs,
    quiet: bool,
    metrics: PerformanceMetrics,
) -> Result<()> {
    commands::sync_source(
        args.aliases,
        args.all,
        args.yes,
        args.reindex,
        args.filter,
        args.no_filter,
        metrics,
        quiet,
    )
    .await
}

/// Dispatch a History command.
fn dispatch_history(
    limit: usize,
    format: &crate::utils::cli_args::FormatArg,
    clear: bool,
    clear_before: Option<&str>,
    quiet: bool,
    prefs: &CliPreferences,
) -> Result<()> {
    commands::show_history(prefs, limit, format.resolve(quiet), clear, clear_before)
}

/// Dispatch a List command.
async fn dispatch_list(
    format: crate::utils::cli_args::FormatArg,
    status: bool,
    details: bool,
    limit: Option<usize>,
    quiet: bool,
) -> Result<()> {
    commands::list_sources(format.resolve(quiet), status, details, limit).await
}

/// Dispatch a Validate command (deprecated).
#[allow(deprecated)]
async fn dispatch_validate(
    alias: Option<String>,
    all: bool,
    format: crate::utils::cli_args::FormatArg,
    quiet: bool,
) -> Result<()> {
    if !utils::cli_args::deprecation_warnings_suppressed() {
        eprintln!(
            "{}",
            "Warning: 'validate' is deprecated, use 'check' instead".yellow()
        );
    }
    commands::validate_source(alias, all, format.resolve(quiet)).await
}

/// Dispatch an Update command (deprecated).
#[allow(deprecated)]
async fn dispatch_update(cmd: Commands, quiet: bool, metrics: PerformanceMetrics) -> Result<()> {
    let Commands::Update {
        aliases,
        all,
        yes: _,
    } = cmd
    else {
        unreachable!("dispatch_update called with non-Update command");
    };

    if !utils::cli_args::deprecation_warnings_suppressed() {
        eprintln!(
            "{}",
            "Warning: 'update' is deprecated, use 'refresh' instead".yellow()
        );
    }
    handle_refresh(aliases, all, false, None, false, metrics, quiet).await
}

/// Dispatch a Remove command (deprecated).
#[allow(deprecated)]
async fn dispatch_remove(alias: String, yes: bool, quiet: bool) -> Result<()> {
    if !utils::cli_args::deprecation_warnings_suppressed() {
        eprintln!(
            "{}",
            "Warning: 'remove' is deprecated, use 'rm' instead".yellow()
        );
    }
    commands::remove_source(&alias, yes, quiet).await
}

/// Dispatch a Lookup command.
async fn dispatch_lookup(
    query: String,
    format: crate::utils::cli_args::FormatArg,
    limit: Option<usize>,
    quiet: bool,
    metrics: PerformanceMetrics,
) -> Result<()> {
    commands::lookup_registry(&query, metrics, quiet, format.resolve(quiet), limit).await
}

fn handle_completions(
    shell: Option<clap_complete::Shell>,
    list: bool,
    format: crate::output::OutputFormat,
) {
    if list {
        commands::list_supported(format);
    } else if let Some(shell) = shell {
        commands::generate(shell);
    } else {
        commands::list_supported(format);
    }
}

async fn handle_add(
    args: crate::cli::AddArgs,
    quiet: bool,
    metrics: PerformanceMetrics,
) -> Result<()> {
    if let Some(manifest) = &args.manifest {
        commands::add_manifest(
            manifest,
            &args.only,
            metrics,
            commands::AddFlowOptions::new(args.dry_run, quiet, args.no_language_filter),
        )
        .await
    } else {
        let alias = args
            .alias
            .as_deref()
            .ok_or_else(|| anyhow!("alias is required when manifest is not provided"))?;
        let url = args
            .url
            .as_deref()
            .ok_or_else(|| anyhow!("url is required when manifest is not provided"))?;

        let descriptor = DescriptorInput::from_cli_inputs(
            &args.aliases,
            args.name.as_deref(),
            args.description.as_deref(),
            args.category.as_deref(),
            &args.tags,
        );

        let request = AddRequest::new(
            alias.to_string(),
            url.to_string(),
            descriptor,
            args.dry_run,
            quiet,
            metrics,
            args.no_language_filter,
        );

        commands::add_source(request).await
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_get(
    targets: Vec<String>,
    lines: Option<String>,
    source: Option<String>,
    context: Option<crate::cli::ContextMode>,
    context_deprecated: Option<crate::cli::ContextMode>,
    after_context: Option<usize>,
    before_context: Option<usize>,
    block: bool,
    max_lines: Option<usize>,
    format: crate::output::OutputFormat,
    copy: bool,
) -> Result<()> {
    let request_specs = parse_get_targets(&targets, lines.as_deref(), source)?;

    let merged_context =
        crate::cli::merge_context_flags(context, context_deprecated, after_context, before_context);

    commands::get_lines(
        &request_specs,
        merged_context.as_ref(),
        block,
        max_lines,
        format,
        copy,
    )
    .await
}

/// Parse get command targets into request specifications.
fn parse_get_targets(
    targets: &[String],
    lines: Option<&str>,
    source: Option<String>,
) -> Result<Vec<RequestSpec>> {
    if targets.is_empty() {
        anyhow::bail!("At least one target is required. Use format: alias[:ranges]");
    }

    if lines.is_some() && targets.len() > 1 {
        anyhow::bail!(
            "--lines can only be combined with a single alias. \
             Provide explicit ranges via colon syntax for each additional target."
        );
    }

    let mut request_specs = Vec::with_capacity(targets.len());
    for (idx, target) in targets.iter().enumerate() {
        let spec = parse_single_target(target, idx, lines)?;
        request_specs.push(spec);
    }

    apply_source_override(&mut request_specs, source)?;
    Ok(request_specs)
}

/// Parse a single target string into a `RequestSpec`.
fn parse_single_target(target: &str, idx: usize, lines: Option<&str>) -> Result<RequestSpec> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Alias at position {} cannot be empty.", idx + 1);
    }

    if let Some((alias_part, range_part)) = trimmed.split_once(':') {
        let trimmed_alias = alias_part.trim();
        if trimmed_alias.is_empty() {
            anyhow::bail!(
                "Alias at position {} cannot be empty. Use syntax like 'bun:120-142'.",
                idx + 1
            );
        }
        if range_part.is_empty() {
            anyhow::bail!(
                "Alias '{trimmed_alias}' is missing a range. \
                 Use syntax like '{trimmed_alias}:120-142'."
            );
        }
        Ok(RequestSpec {
            alias: trimmed_alias.to_string(),
            line_expression: range_part.trim().to_string(),
        })
    } else {
        let Some(line_expr) = lines else {
            anyhow::bail!(
                "Missing line specification for alias '{trimmed}'. \
                 Use '{trimmed}:1-3' or provide --lines."
            );
        };
        Ok(RequestSpec {
            alias: trimmed.to_string(),
            line_expression: line_expr.to_string(),
        })
    }
}

/// Apply source override to request specs if provided.
fn apply_source_override(request_specs: &mut [RequestSpec], source: Option<String>) -> Result<()> {
    if let Some(explicit_source) = source {
        if request_specs.len() > 1 {
            anyhow::bail!("--source cannot be combined with multiple alias targets.");
        }
        if let Some(first) = request_specs.first_mut() {
            first.alias = explicit_source;
        }
    }
    Ok(())
}

async fn handle_query(
    args: crate::cli::QueryArgs,
    quiet: bool,
    prefs: &mut CliPreferences,
    metrics: PerformanceMetrics,
) -> Result<()> {
    let resolved_format = args.format.resolve(quiet);
    let merged_context = crate::cli::merge_context_flags(
        args.context,
        args.context_deprecated,
        args.after_context,
        args.before_context,
    );

    commands::query(
        &args.inputs,
        &args.sources,
        args.limit,
        args.all,
        args.page,
        false, // last - query command doesn't support --last flag
        args.top,
        args.heading_level.clone(),
        resolved_format,
        &args.show,
        args.no_summary,
        args.score_precision,
        args.snippet_lines,
        args.max_chars,
        merged_context.as_ref(),
        args.block,
        args.max_lines,
        args.no_history,
        args.copy,
        quiet,
        args.headings_only,
        args.timing,
        Some(prefs),
        metrics,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
async fn handle_find(
    inputs: Vec<String>,
    sources: Vec<String>,
    limit: Option<usize>,
    all: bool,
    page: usize,
    top: Option<u8>,
    heading_level: Option<String>,
    format: crate::output::OutputFormat,
    show: Vec<crate::cli::ShowComponent>,
    no_summary: bool,
    score_precision: Option<u8>,
    snippet_lines: u8,
    max_chars: Option<usize>,
    context: Option<crate::cli::ContextMode>,
    context_deprecated: Option<crate::cli::ContextMode>,
    after_context: Option<usize>,
    before_context: Option<usize>,
    block: bool,
    max_lines: Option<usize>,
    headings_only: bool,
    no_history: bool,
    copy: bool,
    timing: bool,
    quiet: bool,
    prefs: &mut CliPreferences,
    metrics: PerformanceMetrics,
) -> Result<()> {
    if !utils::cli_args::deprecation_warnings_suppressed() {
        eprintln!(
            "{}",
            "Warning: 'find' is deprecated, use 'query' or 'get' instead".yellow()
        );
    }

    let merged_context =
        crate::cli::merge_context_flags(context, context_deprecated, after_context, before_context);

    commands::find(
        &inputs,
        &sources,
        limit,
        all,
        page,
        false, // last - find command doesn't support --last flag
        top,
        heading_level,
        format,
        &show,
        no_summary,
        score_precision,
        snippet_lines,
        max_chars,
        merged_context.as_ref(),
        block,
        max_lines,
        no_history,
        copy,
        quiet,
        headings_only,
        timing,
        Some(prefs),
        metrics,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
async fn handle_toc(
    alias: Option<String>,
    sources: Vec<String>,
    all: bool,
    format: crate::output::OutputFormat,
    anchors: bool,
    show_anchors: bool,
    limit: Option<usize>,
    max_depth: Option<u8>,
    heading_level: Option<crate::utils::heading_filter::HeadingLevelFilter>,
    filter: Option<String>,
    tree: bool,
    next: bool,
    previous: bool,
    last: bool,
    page: usize,
) -> Result<()> {
    if !utils::cli_args::deprecation_warnings_suppressed() {
        eprintln!(
            "{}",
            "Warning: 'toc' is deprecated, use 'map' instead".yellow()
        );
    }

    commands::show_toc(
        alias.as_deref(),
        &sources,
        all,
        format,
        anchors,
        show_anchors,
        limit,
        max_depth,
        heading_level.as_ref(),
        filter.as_deref(),
        tree,
        next,
        previous,
        last,
        page,
    )
    .await
}

async fn handle_docs(
    command: Option<DocsCommands>,
    quiet: bool,
    metrics: PerformanceMetrics,
    _prefs: &mut CliPreferences,
) -> Result<()> {
    match command {
        Some(DocsCommands::Search(args)) => docs_search(args, quiet, metrics.clone()).await?,
        Some(DocsCommands::Sync {
            force,
            quiet: sync_quiet,
        }) => docs_sync(force, sync_quiet, metrics.clone())?,
        Some(DocsCommands::Overview) => {
            docs_overview(quiet, metrics.clone())?;
        },
        Some(DocsCommands::Cat) => {
            docs_cat(metrics.clone())?;
        },
        Some(DocsCommands::Export { format }) => {
            docs_export(Some(format))?;
        },
        None => {
            // When no subcommand is provided, show overview
            docs_overview(quiet, metrics.clone())?;
        },
    }

    Ok(())
}

fn handle_claude_plugin(command: ClaudePluginCommands) -> Result<()> {
    match command {
        ClaudePluginCommands::Install { scope } => {
            commands::install_local_plugin(scope)?;
        },
    }

    Ok(())
}

async fn docs_search(args: DocsSearchArgs, quiet: bool, metrics: PerformanceMetrics) -> Result<()> {
    sync_and_report(false, quiet, metrics.clone())?;
    let query = args.query.join(" ").trim().to_string();
    if query.is_empty() {
        anyhow::bail!("Search query cannot be empty");
    }

    // Resolve format once before checking placeholder content
    let format = args.format.resolve(quiet);

    // Check if the bundled docs contain placeholder content
    let storage = Storage::new()?;
    if let Ok(content_path) = storage.llms_txt_path(BUNDLED_ALIAS) {
        if let Ok(content) = std::fs::read_to_string(&content_path) {
            if content.contains("# BLZ bundled docs (placeholder)") {
                let error_msg = if matches!(format, crate::output::OutputFormat::Json) {
                    // JSON output: structured error message
                    let error_json = serde_json::json!({
                        "error": "Bundled documentation content not yet available",
                        "reason": "The blz-docs source currently contains placeholder content",
                        "suggestions": [
                            "Use 'blz docs overview' for quick-start information",
                            "Use 'blz docs export' to view CLI documentation",
                            "Full bundled documentation will be included in a future release"
                        ]
                    });
                    return Err(anyhow!(serde_json::to_string_pretty(&error_json)?));
                } else {
                    // Text output: user-friendly message
                    "Bundled documentation content not yet available.\n\
                     \n\
                     The blz-docs source currently contains placeholder content.\n\
                     Full documentation will be included in a future release.\n\
                     \n\
                     Available alternatives:\n\
                     • Run 'blz docs overview' for quick-start information\n\
                     • Run 'blz docs export' to view CLI documentation\n\
                     • Run 'blz docs cat' to view the current placeholder content"
                };
                anyhow::bail!("{error_msg}");
            }
        }
    }

    let sources = vec![BUNDLED_ALIAS.to_string()];

    // Convert docs search args context to ContextMode
    let context_mode = args.context.map(crate::cli::ContextMode::Symmetric);

    commands::search(
        &query,
        &sources,
        false,
        args.limit,
        1,
        args.top,
        None, // heading_level - not supported in bare command mode
        format,
        &args.show,
        args.no_summary,
        args.score_precision,
        args.snippet_lines,
        args.max_chars.unwrap_or(commands::DEFAULT_MAX_CHARS),
        context_mode.as_ref(),
        args.block,
        args.max_block_lines,
        false,
        true,
        args.copy,
        false, // timing
        quiet,
        None,
        metrics,
        None,
        false, // no deprecation warning - `blz docs search` is the intended command
    )
    .await
}

fn docs_sync(force: bool, quiet: bool, metrics: PerformanceMetrics) -> Result<()> {
    let status = sync_and_report(force, quiet, metrics)?;
    if !quiet && matches!(status, DocsSyncStatus::Installed | DocsSyncStatus::Updated) {
        let storage = Storage::new()?;
        let llms_path = storage.llms_txt_path(BUNDLED_ALIAS)?;
        println!("Bundled docs stored at {}", llms_path.display());
    }
    Ok(())
}

fn docs_overview(quiet: bool, metrics: PerformanceMetrics) -> Result<()> {
    let status = sync_and_report(false, quiet, metrics)?;
    if !quiet {
        let storage = Storage::new()?;
        let llms_path = storage.llms_txt_path(BUNDLED_ALIAS)?;
        println!("Bundled docs status: {status:?}");
        println!("Alias: {BUNDLED_ALIAS} (also @blz)");
        println!("Stored at: {}", llms_path.display());
    }
    print_overview();
    Ok(())
}

fn docs_cat(metrics: PerformanceMetrics) -> Result<()> {
    sync_and_report(false, true, metrics)?;
    print_full_content();
    Ok(())
}

fn docs_export(format: Option<crate::commands::DocsFormat>) -> Result<()> {
    let requested = format.unwrap_or(crate::commands::DocsFormat::Markdown);
    let effective = match (std::env::var("BLZ_OUTPUT_FORMAT").ok(), requested) {
        (Some(v), crate::commands::DocsFormat::Markdown) if v.eq_ignore_ascii_case("json") => {
            crate::commands::DocsFormat::Json
        },
        _ => requested,
    };
    commands::generate_docs(effective)
}

fn sync_and_report(
    force: bool,
    quiet: bool,
    metrics: PerformanceMetrics,
) -> Result<DocsSyncStatus> {
    let status = sync_bundled_docs(force, metrics)?;
    if !quiet {
        match status {
            DocsSyncStatus::UpToDate => {
                println!("Bundled docs already up to date");
            },
            DocsSyncStatus::Installed => {
                println!("Installed bundled docs source: {BUNDLED_ALIAS}");
            },
            DocsSyncStatus::Updated => {
                println!("Updated bundled docs source: {BUNDLED_ALIAS}");
            },
        }
    }
    Ok(status)
}

async fn handle_anchor(command: AnchorCommands, quiet: bool) -> Result<()> {
    match command {
        AnchorCommands::List {
            alias,
            format,
            anchors,
            limit,
            max_depth,
            filter,
        } => {
            commands::show_toc(
                Some(&alias),
                &[],
                false,
                format.resolve(quiet),
                anchors,
                false, // show_anchors - not applicable in anchor list mode
                limit,
                max_depth,
                None,
                filter.as_deref(),
                false,
                false, // next
                false, // previous
                false, // last
                1,     // page
            )
            .await
        },
        AnchorCommands::Get {
            alias,
            anchor,
            context,
            format,
        } => commands::get_by_anchor(&alias, &anchor, context, format.resolve(quiet)).await,
    }
}

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

async fn handle_registry(
    command: RegistryCommands,
    quiet: bool,
    metrics: PerformanceMetrics,
) -> Result<()> {
    match command {
        RegistryCommands::CreateSource {
            name,
            url,
            description,
            category,
            tags,
            npm,
            github,
            add,
            yes,
        } => {
            commands::create_registry_source(
                &name,
                &url,
                description,
                category,
                tags,
                npm,
                github,
                add,
                yes,
                quiet,
                metrics,
            )
            .await
        },
    }
}

#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
async fn handle_search(
    query: Option<String>,
    sources: Vec<String>,
    next: bool,
    previous: bool,
    last: bool,
    limit: Option<usize>,
    all: bool,
    page: usize,
    top: Option<u8>,
    heading_level: Option<String>,
    format: crate::output::OutputFormat,
    show: Vec<crate::cli::ShowComponent>,
    no_summary: bool,
    score_precision: Option<u8>,
    snippet_lines: u8,
    max_chars: Option<usize>,
    context: Option<crate::cli::ContextMode>,
    block: bool,
    max_lines: Option<usize>,
    headings_only: bool,
    no_history: bool,
    copy: bool,
    timing: bool,
    quiet: bool,
    metrics: PerformanceMetrics,
    prefs: &mut CliPreferences,
) -> Result<()> {
    const DEFAULT_LIMIT: usize = 50;
    const ALL_RESULTS_LIMIT: usize = 10_000;
    const DEFAULT_SNIPPET_LINES: u8 = 3;

    let provided_query = query.is_some();
    let limit_was_explicit = all || limit.is_some();
    let mut use_headings_only = headings_only;

    // Emit deprecation warning if --snippet-lines was explicitly set
    if snippet_lines != DEFAULT_SNIPPET_LINES {
        let args: Vec<String> = std::env::args().collect();
        if flag_present(&args, "--snippet-lines") || std::env::var("BLZ_SNIPPET_LINES").is_ok() {
            // Pass false for quiet - the deprecation function handles quiet mode internally
            utils::cli_args::emit_snippet_lines_deprecation(false);
        }
    }

    if next {
        validate_continuation_flag("--next", provided_query, &sources, page, last)?;
    }

    if previous {
        validate_continuation_flag("--previous", provided_query, &sources, page, last)?;
    }

    let history_entry = if next || previous || !provided_query {
        let mut records = utils::history_log::recent_for_active_scope(1);
        if records.is_empty() {
            anyhow::bail!("No previous search found. Use 'blz search <query>' first.");
        }
        Some(records.remove(0))
    } else {
        None
    };

    if let Some(entry) = history_entry.as_ref() {
        if (next || previous) && headings_only != entry.headings_only {
            anyhow::bail!(
                "Cannot change --headings-only while using --next/--previous. Rerun without continuation flags."
            );
        }
        if !headings_only {
            use_headings_only = entry.headings_only;
        }
    }

    let actual_query = resolve_query(query, history_entry.as_ref())?;
    let actual_sources = resolve_sources(sources, history_entry.as_ref());

    let base_limit = if all {
        ALL_RESULTS_LIMIT
    } else {
        limit.unwrap_or(DEFAULT_LIMIT)
    };
    let actual_max_chars = max_chars.map_or(commands::DEFAULT_MAX_CHARS, commands::clamp_max_chars);

    let (actual_page, actual_limit) = if let Some(entry) = history_entry.as_ref() {
        let adj = apply_history_pagination(
            entry,
            next,
            previous,
            provided_query,
            all,
            limit,
            limit_was_explicit,
            page,
            base_limit,
            ALL_RESULTS_LIMIT,
        )?;
        (adj.page, adj.limit)
    } else {
        (page, base_limit)
    };

    commands::search(
        &actual_query,
        &actual_sources,
        last,
        actual_limit,
        actual_page,
        top,
        heading_level.clone(),
        format,
        &show,
        no_summary,
        score_precision,
        snippet_lines,
        actual_max_chars,
        context.as_ref(),
        block,
        max_lines,
        use_headings_only,
        no_history,
        copy,
        timing,
        quiet,
        Some(prefs),
        metrics,
        None,
        true, // emit deprecation warning - this is the deprecated `blz search` command
    )
    .await
}

#[allow(clippy::fn_params_excessive_bools)]
async fn handle_refresh(
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
        return commands::refresh_all(metrics, quiet, reindex, filter.as_ref(), no_filter).await;
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
        commands::refresh_source(
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

fn print_diagnostics(
    cli: &Cli,
    metrics: &PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>,
) {
    if cli.debug {
        metrics.print_summary();
    }
    if let Some(monitor) = resource_monitor {
        monitor.print_resource_usage();
    }
}

fn apply_preference_defaults(cli: &mut Cli, prefs: &CliPreferences, args: &[String]) {
    if let Some(Commands::Search(search_args)) = cli.command.as_mut() {
        let show_env = std::env::var("BLZ_SHOW").is_ok();
        if search_args.show.is_empty() && !flag_present(args, "--show") && !show_env {
            search_args.show = prefs.default_show_components();
        }

        if search_args.score_precision.is_none()
            && !flag_present(args, "--score-precision")
            && std::env::var("BLZ_SCORE_PRECISION").is_err()
        {
            search_args.score_precision = Some(prefs.default_score_precision());
        }

        if !flag_present(args, "--snippet-lines") && std::env::var("BLZ_SNIPPET_LINES").is_err() {
            search_args.snippet_lines = prefs.default_snippet_lines();
        }
    }
}

fn flag_present(args: &[String], flag: &str) -> bool {
    let flag_eq = flag;
    let flag_eq_with_equal = format!("{flag}=");
    args.iter()
        .any(|arg| arg == flag_eq || arg.starts_with(&flag_eq_with_equal))
}

/// Pagination adjustments computed from history and continuation flags.
struct PaginationAdjustment {
    page: usize,
    limit: usize,
}

/// Apply history-based pagination adjustments for --next/--previous flags.
///
/// Returns adjusted page and limit values based on history entry and continuation mode.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
fn apply_history_pagination(
    entry: &utils::preferences::SearchHistoryEntry,
    next: bool,
    previous: bool,
    provided_query: bool,
    all: bool,
    limit: Option<usize>,
    limit_was_explicit: bool,
    current_page: usize,
    current_limit: usize,
    all_results_limit: usize,
) -> Result<PaginationAdjustment> {
    let mut actual_page = current_page;
    let mut actual_limit = current_limit;

    if next {
        validate_history_has_results(entry)?;
        validate_pagination_limit_consistency(
            "--next",
            all,
            limit_was_explicit,
            limit,
            entry.limit,
            all_results_limit,
        )?;

        if let (Some(prev_page), Some(total_pages)) = (entry.page, entry.total_pages) {
            if prev_page >= total_pages {
                anyhow::bail!("Already at the last page (page {prev_page} of {total_pages})");
            }
            actual_page = prev_page + 1;
        } else {
            actual_page = entry.page.unwrap_or(1) + 1;
        }

        if !limit_was_explicit {
            actual_limit = entry.limit.unwrap_or(actual_limit);
        }
    } else if previous {
        validate_history_has_results(entry)?;
        validate_pagination_limit_consistency(
            "--previous",
            all,
            limit_was_explicit,
            limit,
            entry.limit,
            all_results_limit,
        )?;

        if let Some(prev_page) = entry.page {
            if prev_page <= 1 {
                anyhow::bail!("Already on first page");
            }
            actual_page = prev_page - 1;
        } else {
            anyhow::bail!("No previous page found in search history");
        }

        if !limit_was_explicit {
            actual_limit = entry.limit.unwrap_or(actual_limit);
        }
    } else if !provided_query && !limit_was_explicit {
        actual_limit = entry.limit.unwrap_or(actual_limit);
    }

    Ok(PaginationAdjustment {
        page: actual_page,
        limit: actual_limit,
    })
}

/// Validate that history entry has non-zero results.
fn validate_history_has_results(entry: &utils::preferences::SearchHistoryEntry) -> Result<()> {
    if matches!(entry.total_pages, Some(0)) || matches!(entry.total_results, Some(0)) {
        anyhow::bail!(
            "Previous search returned 0 results. Rerun with a different query or source."
        );
    }
    Ok(())
}

/// Resolve the search query from explicit input or history.
fn resolve_query(
    mut query: Option<String>,
    history: Option<&utils::preferences::SearchHistoryEntry>,
) -> Result<String> {
    if let Some(value) = query.take() {
        Ok(value)
    } else if let Some(entry) = history {
        Ok(entry.query.clone())
    } else {
        anyhow::bail!("No previous search found. Use 'blz search <query>' first.");
    }
}

/// Resolve source filters from explicit input or history.
fn resolve_sources(
    sources: Vec<String>,
    history: Option<&utils::preferences::SearchHistoryEntry>,
) -> Vec<String> {
    if !sources.is_empty() {
        sources
    } else if let Some(entry) = history {
        entry.source.as_ref().map_or_else(Vec::new, |source_str| {
            source_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        })
    } else {
        Vec::new()
    }
}

/// Validate that pagination limit hasn't changed when using continuation flags.
fn validate_pagination_limit_consistency(
    flag: &str,
    all: bool,
    limit_was_explicit: bool,
    limit: Option<usize>,
    history_limit: Option<usize>,
    all_results_limit: usize,
) -> Result<()> {
    let history_all = history_limit.is_some_and(|value| value >= all_results_limit);
    if all != history_all {
        anyhow::bail!(
            "Cannot use {flag} when changing page size or --all; rerun without {flag} or reuse the previous pagination flags."
        );
    }
    if limit_was_explicit {
        if let Some(requested_limit) = limit {
            if history_limit != Some(requested_limit) {
                anyhow::bail!(
                    "Cannot use {flag} when changing page size; rerun without {flag} or reuse the previous limit."
                );
            }
        }
    }
    Ok(())
}

/// Validate that a continuation flag (--next/--previous) is not combined with incompatible options.
fn validate_continuation_flag(
    flag: &str,
    provided_query: bool,
    sources: &[String],
    page: usize,
    last: bool,
) -> Result<()> {
    if provided_query {
        anyhow::bail!(
            "Cannot combine {flag} with an explicit query. Remove the query to continue from the previous search."
        );
    }
    if !sources.is_empty() {
        anyhow::bail!(
            "Cannot combine {flag} with --source. Omit --source to reuse the last search context."
        );
    }
    if page != 1 {
        anyhow::bail!("Cannot combine {flag} with --page. Use one pagination option at a time.");
    }
    if last {
        anyhow::bail!("Cannot combine {flag} with --last. Choose a single continuation flag.");
    }
    Ok(())
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
    #[allow(deprecated)]
    fn test_cli_parse_refresh_multiple_aliases() {
        use clap::Parser;

        let cli = Cli::try_parse_from(vec!["blz", "refresh", "bun", "react"]).unwrap();
        match cli.command {
            Some(Commands::Refresh { aliases, all, .. }) => {
                assert_eq!(aliases, vec!["bun", "react"]);
                assert!(!all);
            },
            other => panic!("Expected refresh command, got {other:?}"),
        }
    }

    #[test]
    fn test_cli_refresh_all_conflict_with_aliases() {
        use clap::Parser;

        let result = Cli::try_parse_from(vec!["blz", "refresh", "bun", "--all"]);
        assert!(
            result.is_err(),
            "Should error when both --all and aliases are provided"
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_cli_parse_update_multiple_aliases() {
        use clap::Parser;

        // Test deprecated 'update' command for backward compatibility
        let cli = Cli::try_parse_from(vec!["blz", "update", "bun", "react"]).unwrap();
        match cli.command {
            Some(Commands::Update { aliases, all, .. }) => {
                assert_eq!(aliases, vec!["bun", "react"]);
                assert!(!all);
            },
            other => panic!("Expected update command, got {other:?}"),
        }
    }

    #[test]
    #[allow(deprecated)]
    fn test_cli_update_all_conflict_with_aliases() {
        use clap::Parser;

        // Test deprecated 'update' command for backward compatibility
        let result = Cli::try_parse_from(vec!["blz", "update", "bun", "--all"]);
        assert!(
            result.is_err(),
            "--all should conflict with explicit aliases"
        );
    }

    #[test]
    fn test_cli_invalid_flag_combinations() {
        use clap::Parser;

        // Test invalid flag combinations that should fail
        let invalid_combinations = vec![
            // Missing required arguments
            vec!["blz", "add", "alias"], // Missing URL
            // Note: "blz get alias" is now valid (supports colon syntax like "alias:1-3")
            vec!["blz", "search"], // Missing query
            vec!["blz", "lookup"], // Missing query
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

        if let Some(Commands::Search(args)) = cli.command {
            assert_eq!(
                args.limit, None,
                "Default limit should be unset (defaults to 50)"
            );
            assert_eq!(args.page, 1, "Default page should be 1");
            assert!(!args.all, "Default all should be false");
            // When running tests, stdout is not a terminal, so default is JSON when piped
            let expected_format = if is_terminal::IsTerminal::is_terminal(&std::io::stdout()) {
                crate::output::OutputFormat::Text
            } else {
                crate::output::OutputFormat::Json
            };
            assert_eq!(
                args.format.resolve(false),
                expected_format,
                "Default format should be JSON when piped, Text when terminal"
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
    #[allow(deprecated)]
    fn anchors_alias_still_parses_to_toc() {
        use clap::Parser;

        let raw = to_string_vec(&["blz", "anchors", "react"]);
        let cli = Cli::try_parse_from(raw).expect("anchors alias should parse");
        match cli.command {
            Some(Commands::Toc(args)) => assert_eq!(args.alias, Some("react".to_string())),
            other => panic!("expected toc command, got {other:?}"),
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

        if let Some(Commands::Search(args)) = cli.command {
            assert_eq!(args.sources, vec!["node"]);
            assert_eq!(args.limit, Some(20));
            assert_eq!(args.page, 2);
            assert!(args.top.is_some());
            assert_eq!(
                args.format.resolve(false),
                crate::output::OutputFormat::Json
            );
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

        if let Some(Commands::Add(args)) = cli.command {
            assert_eq!(args.alias.as_deref(), Some("test"));
            assert_eq!(args.url.as_deref(), Some("https://example.com/llms.txt"));
            assert!(args.aliases.is_empty());
            assert!(args.tags.is_empty());
            assert!(args.name.is_none());
            assert!(args.description.is_none());
            assert!(args.category.is_none());
            assert!(args.yes);
            assert!(!args.dry_run);
            assert!(args.manifest.is_none());
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
            targets,
            lines,
            source,
            context,
            block,
            max_lines,
            format,
            copy: _,
            ..
        }) = cli.command
        {
            assert_eq!(targets, vec!["test".to_string()]);
            assert_eq!(lines, Some("1-10".to_string()));
            assert!(source.is_none());
            assert_eq!(context, Some(crate::cli::ContextMode::Symmetric(5)));
            assert!(!block);
            assert_eq!(max_lines, None);
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
            // Note: "blz get alias" is now valid (supports colon syntax like "alias:1-3")
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
            "refresh",
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

        // `rm` is now its own command (not an alias of `remove`)
        let rm_cmd = cmd
            .get_subcommands()
            .find(|sub| sub.get_name() == "rm")
            .expect("Should have rm command");
        assert_eq!(rm_cmd.get_name(), "rm");

        // `remove` is deprecated but still exists
        let remove_cmd = cmd.get_subcommands().find(|sub| sub.get_name() == "remove");
        assert!(
            remove_cmd.is_some(),
            "Remove command should still exist (deprecated)"
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
            "sources",
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
            "refresh",
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

    #[test]
    fn test_multi_source_search_parsing() {
        use clap::Parser;

        // Test comma-separated sources
        let cli = Cli::try_parse_from(vec![
            "blz",
            "search",
            "hooks",
            "--source",
            "react,vue,svelte",
        ])
        .unwrap();

        if let Some(Commands::Search(args)) = cli.command {
            assert_eq!(args.sources, vec!["react", "vue", "svelte"]);
        } else {
            panic!("Expected search command");
        }
    }

    #[test]
    fn test_single_source_search_parsing() {
        use clap::Parser;

        // Test single source (backward compatibility)
        let cli = Cli::try_parse_from(vec!["blz", "search", "hooks", "--source", "react"]).unwrap();

        if let Some(Commands::Search(args)) = cli.command {
            assert_eq!(args.sources, vec!["react"]);
        } else {
            panic!("Expected search command");
        }
    }

    #[test]
    fn test_no_source_search_parsing() {
        use clap::Parser;

        // Test no source (searches all)
        let cli = Cli::try_parse_from(vec!["blz", "search", "hooks"]).unwrap();

        if let Some(Commands::Search(args)) = cli.command {
            assert!(args.sources.is_empty());
        } else {
            panic!("Expected search command");
        }
    }

    #[test]
    fn test_multi_source_shorthand_parsing() {
        use clap::Parser;

        // Test with -s shorthand
        let cli = Cli::try_parse_from(vec!["blz", "search", "api", "-s", "bun,node,deno"]).unwrap();

        if let Some(Commands::Search(args)) = cli.command {
            assert_eq!(args.sources, vec!["bun", "node", "deno"]);
        } else {
            panic!("Expected search command");
        }
    }

    #[test]
    fn test_get_command_with_source_flag() {
        use clap::Parser;

        let cli = Cli::try_parse_from(vec![
            "blz", "get", "meta", "--lines", "1-3", "--source", "bun",
        ])
        .unwrap();

        if let Some(Commands::Get {
            targets,
            source,
            lines,
            ..
        }) = cli.command
        {
            assert_eq!(targets, vec!["meta".to_string()]);
            assert_eq!(source.as_deref(), Some("bun"));
            assert_eq!(lines.as_deref(), Some("1-3"));
        } else {
            panic!("Expected get command");
        }
    }

    #[test]
    fn test_get_command_with_source_shorthand() {
        use clap::Parser;

        let cli = Cli::try_parse_from(vec!["blz", "get", "meta:4-6", "-s", "canonical"]).unwrap();

        if let Some(Commands::Get {
            targets, source, ..
        }) = cli.command
        {
            assert_eq!(targets, vec!["meta:4-6".to_string()]);
            assert_eq!(source.as_deref(), Some("canonical"));
        } else {
            panic!("Expected get command");
        }
    }
}
