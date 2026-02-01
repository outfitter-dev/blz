use std::convert::TryFrom;

use anyhow::{Context, Result, anyhow};
use blz_core::{AnchorsMap, HeadingLevel, LlmsJson, Storage};
use chrono::Utc;
use clap::{Args, Subcommand};
use colored::Colorize;

use crate::commands::RequestSpec;
use crate::output::OutputFormat;
use crate::utils::cli_args;
use crate::utils::cli_args::FormatArg;
use crate::utils::heading_filter::HeadingLevelFilter;
use crate::utils::parsing::{LineRange, parse_line_ranges};
use crate::utils::preferences::{self, TocHistoryEntry};

/// Validates that limit is at least 1
fn validate_limit(s: &str) -> Result<usize, String> {
    let value: usize = s
        .parse()
        .map_err(|_| format!("'{s}' is not a valid number"))?;
    if value == 0 {
        Err("limit must be at least 1".to_string())
    } else {
        Ok(value)
    }
}

/// Arguments for the deprecated `blz toc` command.
///
/// This command is deprecated in favor of `blz map`.
#[derive(Args, Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct TocArgs {
    /// Source alias (optional when using --source or --all)
    pub alias: Option<String>,
    /// Output format
    #[command(flatten)]
    pub format: FormatArg,
    /// Filter headings by boolean expression (use AND/OR/NOT; whitespace implies OR)
    #[arg(long = "filter", value_name = "EXPR")]
    pub filter: Option<String>,
    /// Limit results to headings at or above this level (1-6)
    #[arg(
        long = "max-depth",
        value_name = "DEPTH",
        value_parser = clap::value_parser!(u8).range(1..=6)
    )]
    pub max_depth: Option<u8>,
    /// Filter by heading level with comparison operators (e.g., <=2, >3, 1-3, 1,2,3)
    #[arg(short = 'H', long = "heading-level", value_name = "FILTER")]
    pub heading_level: Option<HeadingLevelFilter>,
    /// Search specific sources (comma-separated aliases)
    #[arg(
        short = 's',
        long = "source",
        value_name = "ALIASES",
        value_delimiter = ',',
        num_args = 1..,
        conflicts_with = "alias"
    )]
    pub sources: Vec<String>,
    /// Include all sources when no alias is provided, or bypass pagination limits
    #[arg(long)]
    pub all: bool,
    /// Display as hierarchical tree with box-drawing characters
    #[arg(long)]
    pub tree: bool,
    /// Show anchor metadata and remap history
    #[arg(long, alias = "mappings")]
    pub anchors: bool,
    /// Show anchor slugs in normal TOC output
    #[arg(short = 'a', long)]
    pub show_anchors: bool,
    /// Continue from previous toc (next page)
    #[arg(
        long,
        conflicts_with = "page",
        conflicts_with = "last",
        conflicts_with = "previous",
        conflicts_with = "all",
        display_order = 50
    )]
    pub next: bool,
    /// Go back to previous page
    #[arg(
        long,
        conflicts_with = "page",
        conflicts_with = "last",
        conflicts_with = "next",
        conflicts_with = "all",
        display_order = 51
    )]
    pub previous: bool,
    /// Jump to last page of results
    #[arg(
        long,
        conflicts_with = "next",
        conflicts_with = "page",
        conflicts_with = "previous",
        conflicts_with = "all",
        display_order = 52
    )]
    pub last: bool,
    /// Maximum number of headings per page (must be at least 1)
    #[arg(
        short = 'n',
        long,
        value_name = "COUNT",
        value_parser = validate_limit,
        display_order = 53
    )]
    pub limit: Option<usize>,
    /// Page number for pagination
    #[arg(
        long,
        default_value = "1",
        conflicts_with = "next",
        conflicts_with = "last",
        conflicts_with = "previous",
        conflicts_with = "all",
        display_order = 55
    )]
    pub page: usize,
}

/// Subcommands for `blz anchor` (legacy).
#[derive(Subcommand, Clone, Debug)]
pub enum AnchorCommands {
    /// List table-of-contents entries (headings) for a source.
    List {
        /// Source alias.
        alias: String,
        /// Output format.
        #[command(flatten)]
        format: FormatArg,
        /// Show anchor metadata and remap history.
        #[arg(long, alias = "mappings")]
        anchors: bool,
        /// Maximum number of headings to display.
        #[arg(short = 'n', long, value_name = "COUNT")]
        limit: Option<usize>,
        /// Limit results to headings at or above this level (1-6).
        #[arg(
            long = "max-depth",
            value_name = "DEPTH",
            value_parser = clap::value_parser!(u8).range(1..=6)
        )]
        max_depth: Option<u8>,
        /// Filter headings by boolean expression (use AND/OR/NOT; whitespace implies OR).
        #[arg(long = "filter", value_name = "EXPR")]
        filter: Option<String>,
    },
    /// Get content by anchor.
    Get {
        /// Source alias.
        alias: String,
        /// Anchor value (from list).
        anchor: String,
        /// Context lines around the section.
        #[arg(short = 'c', long)]
        context: Option<usize>,
        /// Output format.
        #[command(flatten)]
        format: FormatArg,
    },
}

/// Dispatch the deprecated `toc` command.
///
/// This function handles the `blz toc` command, which is deprecated in favor of `blz map`.
/// It extracts arguments from `TocArgs`, shows a deprecation warning, and delegates to `execute`.
#[allow(deprecated)]
pub async fn dispatch(args: TocArgs, quiet: bool) -> Result<()> {
    if !cli_args::deprecation_warnings_suppressed() {
        eprintln!(
            "{}",
            "Warning: 'toc' is deprecated, use 'map' instead".yellow()
        );
    }

    execute(
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

/// Dispatch anchor subcommands.
///
/// This function handles the `blz anchor` subcommands (list, get).
pub async fn dispatch_anchor(command: AnchorCommands, quiet: bool) -> Result<()> {
    match command {
        AnchorCommands::List {
            alias,
            format,
            anchors,
            limit,
            max_depth,
            filter,
        } => {
            execute(
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
        } => get_by_anchor(&alias, &anchor, context, format.resolve(quiet)).await,
    }
}

/// Serialize a `HeadingLevelFilter` back to its string representation
fn serialize_heading_level_filter(
    filter: &crate::utils::heading_filter::HeadingLevelFilter,
) -> String {
    use crate::utils::heading_filter::HeadingLevelFilter;
    match filter {
        HeadingLevelFilter::Exact(n) => format!("={n}"),
        HeadingLevelFilter::LessThan(n) => format!("<{n}"),
        HeadingLevelFilter::LessThanOrEqual(n) => format!("<={n}"),
        HeadingLevelFilter::GreaterThan(n) => format!(">{n}"),
        HeadingLevelFilter::GreaterThanOrEqual(n) => format!(">={n}"),
        HeadingLevelFilter::List(levels) => levels
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(","),
        HeadingLevelFilter::Range(start, end) => format!("{start}-{end}"),
    }
}

/// Restore pagination state from history when using --next, --previous, or --last
#[allow(clippy::ref_option)]
fn restore_pagination_state(
    last_entry: &Option<TocHistoryEntry>,
    next: bool,
    previous: bool,
    last: bool,
    mut limit: Option<usize>,
    mut page: usize,
) -> Result<(Option<usize>, usize)> {
    if last_entry.is_none() && ((next || previous) || (last && limit.is_none())) {
        return Err(anyhow!(
            "No saved pagination state found. Run `blz toc <alias> --limit <COUNT>` first."
        ));
    }

    if (next || previous || last) && limit.is_none() {
        if let Some(entry) = last_entry {
            limit = entry.limit;
        }
        if limit.is_none() {
            return Err(anyhow!(
                "No saved page size found. Run `blz toc <alias> --limit <COUNT>` first."
            ));
        }
    }

    if matches!(limit, Some(0)) {
        anyhow::bail!("--limit must be at least 1; use --all to show everything.");
    }

    if next {
        page = last_entry
            .as_ref()
            .and_then(|entry| entry.page)
            .unwrap_or(1)
            .saturating_add(1);
    } else if previous {
        page = last_entry
            .as_ref()
            .and_then(|entry| entry.page)
            .unwrap_or(2)
            .saturating_sub(1)
            .max(1);
    } else if last {
        page = usize::MAX;
    }

    Ok((limit, page))
}

/// Restore filter parameters from history when navigating
#[allow(clippy::ref_option)]
fn restore_filter_params<'a>(
    last_entry: &'a Option<TocHistoryEntry>,
    filter_expr: Option<&'a str>,
    max_depth: Option<u8>,
    next: bool,
    previous: bool,
    last: bool,
) -> (Option<&'a str>, Option<u8>) {
    if next || previous || last {
        let saved_filter = last_entry.as_ref().and_then(|e| e.filter.as_deref());
        let saved_max_depth = last_entry.as_ref().and_then(|e| e.max_depth);
        (filter_expr.or(saved_filter), max_depth.or(saved_max_depth))
    } else {
        (filter_expr, max_depth)
    }
}

/// Resolve the list of sources to process
#[allow(
    clippy::ref_option,
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools
)]
fn resolve_source_list(
    storage: &Storage,
    alias: Option<&str>,
    sources: &[String],
    all_sources_mode: bool,
    last_entry: &Option<TocHistoryEntry>,
    next: bool,
    previous: bool,
    last: bool,
) -> Result<Vec<String>> {
    let mut source_list = if !sources.is_empty() {
        sources.to_vec()
    } else if let Some(single) = alias {
        vec![single.to_string()]
    } else if all_sources_mode {
        storage.list_sources()
    } else {
        Vec::new()
    };

    if source_list.is_empty() && (next || previous || last) {
        if let Some(entry) = last_entry {
            if let Some(saved_sources) = &entry.source {
                source_list = saved_sources
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }

    if source_list.is_empty() {
        if all_sources_mode {
            return Err(anyhow!(
                "No sources configured. Add one with `blz add <alias> <url>` before running `blz toc --all`."
            ));
        }
        return Err(anyhow!(
            "No source specified. Provide an alias, --source <alias>, or run with --all."
        ));
    }

    Ok(source_list)
}

/// Handle --anchors mode output
fn handle_anchors_mode(
    storage: &Storage,
    source_list: &[String],
    output: OutputFormat,
) -> Result<()> {
    if source_list.len() > 1 {
        return Err(anyhow!("--anchors can only be used with a single source"));
    }
    let canonical = crate::utils::resolver::resolve_source(storage, &source_list[0])?
        .unwrap_or_else(|| source_list[0].clone());
    let path = storage.anchors_map_path(&canonical)?;
    if !path.exists() {
        println!("No heading remap metadata found for '{canonical}'");
        return Ok(());
    }
    let txt = std::fs::read_to_string(&path)?;
    let map: AnchorsMap = serde_json::from_str(&txt)?;
    match output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&map)?);
        },
        OutputFormat::Jsonl => {
            for m in map.mappings {
                println!("{}", serde_json::to_string(&m)?);
            }
        },
        OutputFormat::Text => {
            println!(
                "Remap metadata for {} (updated {})\n",
                canonical.green(),
                map.updated_at
            );
            for m in map.mappings {
                let path_str = m.heading_path.join(" > ");
                println!(
                    "  {}\n    {} → {}\n    {}",
                    path_str,
                    m.old_lines,
                    m.new_lines,
                    m.anchor.bright_black()
                );
            }
        },
        OutputFormat::Raw => {
            return Err(anyhow!(
                "Raw output is not supported for toc listings. Use --format json, jsonl, or text instead."
            ));
        },
    }
    Ok(())
}

/// Calculate pagination for TOC entries
#[allow(clippy::needless_pass_by_value)]
fn calculate_pagination(
    all_entries: Vec<serde_json::Value>,
    pagination_limit: Option<usize>,
    page: usize,
) -> (Vec<serde_json::Value>, usize, usize, usize) {
    let total_results = all_entries.len();
    let (page_entries, actual_page, total_pages) = pagination_limit.map_or_else(
        || (all_entries.clone(), 1, 1),
        |lim| {
            let total_pages = if total_results == 0 {
                0
            } else {
                total_results.div_ceil(lim)
            };

            let actual_page = if page == usize::MAX {
                total_pages.max(1)
            } else {
                page.clamp(1, total_pages.max(1))
            };

            let start = (actual_page - 1) * lim;
            let end = start.saturating_add(lim).min(total_results);

            let page_entries = if start < total_results {
                all_entries[start..end].to_vec()
            } else {
                Vec::new()
            };

            (page_entries, actual_page, total_pages)
        },
    );
    (page_entries, actual_page, total_pages, total_results)
}

/// Save TOC history entry for pagination state
#[allow(clippy::too_many_arguments)]
fn save_toc_history_entry(
    source_list: &[String],
    output: OutputFormat,
    actual_page: usize,
    limit: Option<usize>,
    total_pages: usize,
    total_results: usize,
    filter_expr: Option<&str>,
    max_depth: Option<u8>,
    heading_level: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
) {
    let source_str = if source_list.len() == 1 {
        Some(source_list[0].clone())
    } else if !source_list.is_empty() {
        Some(source_list.join(","))
    } else {
        None
    };

    let history_entry = TocHistoryEntry {
        timestamp: Utc::now().to_rfc3339(),
        source: source_str,
        format: preferences::format_to_string(output),
        page: Some(actual_page),
        limit,
        total_pages: Some(total_pages),
        total_results: Some(total_results),
        filter: filter_expr.map(str::to_string),
        max_depth,
        heading_level: heading_level.map(serialize_heading_level_filter),
    };

    if let Err(err) = preferences::save_toc_history(&history_entry) {
        tracing::warn!("failed to save TOC history: {err}");
    }
}

/// Format and print JSON output
fn format_json_output(
    page_entries: &[serde_json::Value],
    actual_page: usize,
    total_pages: usize,
    total_results: usize,
    pagination_limit: Option<usize>,
) -> Result<()> {
    let payload = serde_json::json!({
        "entries": page_entries,
        "page": actual_page,
        "total_pages": total_pages.max(1),
        "total_results": total_results,
        "page_size": pagination_limit,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&payload)
            .context("Failed to serialize table of contents to JSON")?
    );
    Ok(())
}

/// Format and print JSONL output
fn format_jsonl_output(
    page_entries: &[serde_json::Value],
    actual_page: usize,
    total_pages: usize,
    total_results: usize,
    pagination_limit: Option<usize>,
) -> Result<()> {
    let metadata = serde_json::json!({
        "page": actual_page,
        "total_pages": total_pages.max(1),
        "total_results": total_results,
        "page_size": pagination_limit,
    });
    println!(
        "{}",
        serde_json::to_string(&metadata)
            .context("Failed to serialize table of contents metadata to JSONL")?
    );

    for e in page_entries {
        println!(
            "{}",
            serde_json::to_string(e).context("Failed to serialize table of contents to JSONL")?
        );
    }
    Ok(())
}

/// Print paginated flat list text output
fn print_paginated_flat_list(
    storage: &Storage,
    source_list: &[String],
    page_entries: &[serde_json::Value],
    actual_page: usize,
    total_pages: usize,
    total_results: usize,
    show_anchors: bool,
) -> Result<()> {
    // Print header
    if source_list.len() > 1 {
        println!("Table of contents (showing {} sources)", source_list.len());
    } else if let Some(first) = source_list.first() {
        let canonical = crate::utils::resolver::resolve_source(storage, first)?
            .unwrap_or_else(|| first.clone());
        println!("Table of contents for {}\n", canonical.green());
    }

    // Print entries from page_entries (which already has pagination applied)
    for entry in page_entries {
        let heading_path = entry["headingPath"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();
        let name = heading_path.last().unwrap_or(&"");
        let lines = entry["lines"].as_str().unwrap_or("");
        let heading_level = entry["headingLevel"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .unwrap_or(1);

        let indent = "  ".repeat(heading_level.saturating_sub(1));
        let lines_display = format!("[{lines}]").dimmed();

        if show_anchors {
            let anchor = entry["anchor"].as_str().unwrap_or("");
            println!("{indent}- {name} {lines_display} {}", anchor.bright_black());
        } else {
            println!("{indent}- {name} {lines_display}");
        }
    }

    // Print footer with pagination info
    println!(
        "\nPage {} of {} ({} total results)",
        actual_page,
        total_pages.max(1),
        total_results
    );
    Ok(())
}

/// Restore heading level filter from history when navigating
#[allow(clippy::ref_option)]
fn restore_heading_level<'a>(
    heading_level: Option<&'a crate::utils::heading_filter::HeadingLevelFilter>,
    last_entry: &Option<TocHistoryEntry>,
    heading_level_owner: &'a mut Option<crate::utils::heading_filter::HeadingLevelFilter>,
    is_navigating: bool,
) -> Option<&'a crate::utils::heading_filter::HeadingLevelFilter> {
    heading_level.or_else(|| {
        if is_navigating {
            *heading_level_owner = last_entry
                .as_ref()
                .and_then(|e| e.heading_level.as_deref())
                .and_then(|saved_str| saved_str.parse().ok());
            heading_level_owner.as_ref()
        } else {
            None
        }
    })
}

/// Parse filter expression and compute level filter
fn parse_filters(
    filter_expr: Option<&str>,
    heading_level: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
    max_depth: Option<u8>,
) -> Result<(
    Option<HeadingFilter>,
    Option<crate::utils::heading_filter::HeadingLevelFilter>,
)> {
    let filter = filter_expr
        .map(HeadingFilter::parse)
        .transpose()
        .context("Failed to parse filter expression")?;

    let level_filter = heading_level.cloned().or_else(|| {
        max_depth.map(crate::utils::heading_filter::HeadingLevelFilter::LessThanOrEqual)
    });

    Ok((filter, level_filter))
}

/// Collect TOC entries from all sources
fn collect_all_entries(
    storage: &Storage,
    source_list: &[String],
    max_depth: Option<u8>,
    filter: Option<&HeadingFilter>,
    level_filter: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
) -> Result<Vec<serde_json::Value>> {
    let mut all_entries = Vec::new();
    for source_alias in source_list {
        let canonical = crate::utils::resolver::resolve_source(storage, source_alias)?
            .unwrap_or_else(|| source_alias.clone());

        let llms: LlmsJson = storage
            .load_llms_json(&canonical)
            .with_context(|| format!("Failed to load TOC for '{canonical}'"))?;

        collect_entries(
            &mut all_entries,
            &llms.toc,
            max_depth.map(usize::from),
            0,
            filter,
            level_filter,
            source_alias,
            &canonical,
        );
    }
    Ok(all_entries)
}

/// Format and display TOC output based on output format
#[allow(clippy::too_many_arguments)]
fn format_toc_output(
    storage: &Storage,
    output: OutputFormat,
    source_list: &[String],
    page_entries: &[serde_json::Value],
    actual_page: usize,
    total_pages: usize,
    total_results: usize,
    pagination_limit: Option<usize>,
    tree: bool,
    show_anchors: bool,
    max_depth: Option<u8>,
    filter: Option<&HeadingFilter>,
    level_filter: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
) -> Result<()> {
    match output {
        OutputFormat::Json => {
            format_json_output(
                page_entries,
                actual_page,
                total_pages,
                total_results,
                pagination_limit,
            )?;
        },
        OutputFormat::Jsonl => {
            format_jsonl_output(
                page_entries,
                actual_page,
                total_pages,
                total_results,
                pagination_limit,
            )?;
        },
        OutputFormat::Text => {
            if pagination_limit.is_some() && !tree {
                print_paginated_flat_list(
                    storage,
                    source_list,
                    page_entries,
                    actual_page,
                    total_pages,
                    total_results,
                    show_anchors,
                )?;
            } else {
                print_tree_or_hierarchical(
                    storage,
                    source_list,
                    max_depth,
                    filter,
                    level_filter,
                    tree,
                    show_anchors,
                )?;
            }
        },
        OutputFormat::Raw => {
            return Err(anyhow!(
                "Raw output is not supported for toc listings. Use --format json, jsonl, or text instead."
            ));
        },
    }
    Ok(())
}

/// Print tree or hierarchical text output
fn print_tree_or_hierarchical(
    storage: &Storage,
    source_list: &[String],
    max_depth: Option<u8>,
    filter: Option<&HeadingFilter>,
    level_filter: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
    tree: bool,
    show_anchors: bool,
) -> Result<()> {
    for source_alias in source_list {
        let canonical = crate::utils::resolver::resolve_source(storage, source_alias)?
            .unwrap_or_else(|| source_alias.clone());

        let llms: LlmsJson = storage
            .load_llms_json(&canonical)
            .with_context(|| format!("Failed to load TOC for '{canonical}'"))?;

        if source_list.len() > 1 {
            println!("\n{}:", canonical.green());
        } else {
            println!("Table of contents for {}\n", canonical.green());
        }

        if tree {
            let mut count = 0;
            let mut prev_depth: Option<usize> = None;
            let mut prev_h1_had_children = false;
            for (i, e) in llms.toc.iter().enumerate() {
                let is_last = i == llms.toc.len() - 1;
                print_tree(
                    e,
                    0,
                    is_last,
                    "",
                    max_depth.map(usize::from),
                    filter,
                    level_filter,
                    &mut count,
                    None, // No limit for tree mode
                    show_anchors,
                    &mut prev_depth,
                    &mut prev_h1_had_children,
                );
            }
        } else {
            for e in &llms.toc {
                print_text(
                    e,
                    0,
                    max_depth.map(usize::from),
                    filter,
                    level_filter,
                    show_anchors,
                );
            }
        }
    }
    Ok(())
}

#[allow(
    dead_code,
    clippy::unused_async,
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools
)]
pub async fn execute(
    alias: Option<&str>,
    sources: &[String],
    all: bool,
    output: OutputFormat,
    anchors: bool,
    show_anchors: bool,
    limit: Option<usize>,
    max_depth: Option<u8>,
    heading_level: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
    filter_expr: Option<&str>,
    tree: bool,
    next: bool,
    previous: bool,
    last: bool,
    page: usize,
) -> Result<()> {
    let storage = Storage::new()?;
    let all_sources_mode = all && alias.is_none() && sources.is_empty();
    let is_navigating = next || previous || last;

    let last_entry = if is_navigating {
        preferences::load_last_toc_entry()
    } else {
        None
    };

    let (limit, page) = restore_pagination_state(&last_entry, next, previous, last, limit, page)?;
    let (filter_expr, max_depth) =
        restore_filter_params(&last_entry, filter_expr, max_depth, next, previous, last);

    let mut heading_level_owner: Option<crate::utils::heading_filter::HeadingLevelFilter> = None;
    let heading_level = restore_heading_level(
        heading_level,
        &last_entry,
        &mut heading_level_owner,
        is_navigating,
    );

    if anchors && filter_expr.is_some() {
        return Err(anyhow!("--filter cannot be combined with --anchors"));
    }

    let source_list = resolve_source_list(
        &storage,
        alias,
        sources,
        all_sources_mode,
        &last_entry,
        next,
        previous,
        last,
    )?;

    if anchors {
        return handle_anchors_mode(&storage, &source_list, output);
    }

    let (filter, level_filter) = parse_filters(filter_expr, heading_level, max_depth)?;
    let all_entries = collect_all_entries(
        &storage,
        &source_list,
        max_depth,
        filter.as_ref(),
        level_filter.as_ref(),
    )?;

    let pagination_limit = if all && !all_sources_mode {
        None
    } else {
        limit
    };
    let (page_entries, actual_page, total_pages, total_results) =
        calculate_pagination(all_entries, pagination_limit, page);

    if limit.is_some() {
        save_toc_history_entry(
            &source_list,
            output,
            actual_page,
            limit,
            total_pages,
            total_results,
            filter_expr,
            max_depth,
            heading_level,
        );
    }

    format_toc_output(
        &storage,
        output,
        &source_list,
        &page_entries,
        actual_page,
        total_pages,
        total_results,
        pagination_limit,
        tree,
        show_anchors,
        max_depth,
        filter.as_ref(),
        level_filter.as_ref(),
    )
}

#[allow(dead_code)]
fn print_text_with_limit(
    e: &blz_core::TocEntry,
    depth: usize,
    remaining: usize,
    max_depth: Option<usize>,
    filter: Option<&HeadingFilter>,
    level_filter: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
    show_anchors: bool,
) -> usize {
    if remaining == 0 || exceeds_depth(depth, max_depth) {
        return 0;
    }

    let display_path = display_path(e);
    let name = display_path.last().cloned().unwrap_or_default();
    let level_matches =
        level_filter.is_none_or(|f| f.matches(HeadingLevel::from_depth(depth).as_u8()));
    let text_matches = filter.is_none_or(|f| f.matches(&display_path, e.anchor.as_deref()));

    let mut printed = if text_matches && level_matches {
        let indent = "  ".repeat(depth);
        let lines_display = format!("[{}]", e.lines).dimmed();

        if show_anchors {
            let anchor = e.anchor.clone().unwrap_or_default();
            println!(
                "{}- {} {} {}",
                indent,
                name,
                lines_display,
                anchor.bright_black()
            );
        } else {
            println!("{indent}- {name} {lines_display}");
        }

        1
    } else {
        0
    };
    if can_descend(depth, max_depth) {
        for c in &e.children {
            if printed >= remaining {
                break;
            }
            printed += print_text_with_limit(
                c,
                depth + 1,
                remaining - printed,
                max_depth,
                filter,
                level_filter,
                show_anchors,
            );
        }
    }
    printed
}

#[allow(dead_code, clippy::items_after_statements, clippy::too_many_arguments)]
fn collect_entries(
    entries: &mut Vec<serde_json::Value>,
    list: &[blz_core::TocEntry],
    max_depth: Option<usize>,
    depth: usize,
    filter: Option<&HeadingFilter>,
    level_filter: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
    alias: &str,
    canonical: &str,
) {
    for e in list {
        if exceeds_depth(depth, max_depth) {
            continue;
        }
        let display_path = display_path(e);
        let level_matches =
            level_filter.is_none_or(|f| f.matches(HeadingLevel::from_depth(depth).as_u8()));
        let text_matches = filter.is_none_or(|f| f.matches(&display_path, e.anchor.as_deref()));

        if text_matches && level_matches {
            entries.push(serde_json::json!({
                "alias": alias,
                "source": canonical,
                "headingPath": display_path,
                "rawHeadingPath": e.heading_path,
                "headingPathNormalized": e.heading_path_normalized,
                "headingLevel": depth + 1,
                "lines": e.lines,
                "anchor": e.anchor,
            }));
        }
        if !e.children.is_empty() && can_descend(depth, max_depth) {
            collect_entries(
                entries,
                &e.children,
                max_depth,
                depth + 1,
                filter,
                level_filter,
                alias,
                canonical,
            );
        }
    }
}

#[allow(dead_code)]
fn print_text(
    e: &blz_core::TocEntry,
    depth: usize,
    max_depth: Option<usize>,
    filter: Option<&HeadingFilter>,
    level_filter: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
    show_anchors: bool,
) {
    if exceeds_depth(depth, max_depth) {
        return;
    }
    let display_path = display_path(e);
    let name = display_path.last().cloned().unwrap_or_default();
    let level_matches =
        level_filter.is_none_or(|f| f.matches(HeadingLevel::from_depth(depth).as_u8()));
    let text_matches = filter.is_none_or(|f| f.matches(&display_path, e.anchor.as_deref()));
    if text_matches && level_matches {
        let indent = "  ".repeat(depth);
        let lines_display = format!("[{}]", e.lines).dimmed();

        if show_anchors {
            let anchor = e.anchor.clone().unwrap_or_default();
            println!(
                "{}- {} {} {}",
                indent,
                name,
                lines_display,
                anchor.bright_black()
            );
        } else {
            println!("{indent}- {name} {lines_display}");
        }
    }
    if can_descend(depth, max_depth) {
        for c in &e.children {
            print_text(c, depth + 1, max_depth, filter, level_filter, show_anchors);
        }
    }
}

#[allow(dead_code, clippy::too_many_arguments)]
fn print_tree(
    e: &blz_core::TocEntry,
    depth: usize,
    is_last: bool,
    prefix: &str,
    max_depth: Option<usize>,
    filter: Option<&HeadingFilter>,
    level_filter: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
    count: &mut usize,
    limit: Option<usize>,
    show_anchors: bool,
    prev_depth: &mut Option<usize>,
    prev_h1_had_children: &mut bool,
) -> bool {
    if let Some(limit_count) = limit {
        if *count >= limit_count {
            return false;
        }
    }

    if exceeds_depth(depth, max_depth) {
        return false;
    }

    let display_path = display_path(e);
    let name = display_path.last().cloned().unwrap_or_default();
    let level_matches =
        level_filter.is_none_or(|f| f.matches(HeadingLevel::from_depth(depth).as_u8()));
    let text_matches = filter.is_none_or(|f| f.matches(&display_path, e.anchor.as_deref()));

    if text_matches && level_matches {
        // Add blank line when jumping up levels (but not to H1 - H1 handles its own spacing)
        if let Some(prev) = *prev_depth {
            if depth < prev && depth > 0 {
                // Jumping up levels within H2+
                if depth > 1 {
                    // H3+ has continuation pipes
                    let pipe_prefix = prefix.trim_end();
                    println!("{pipe_prefix}");
                } else if depth == 1 {
                    // H2 level: show pipe if not last sibling
                    if is_last {
                        println!();
                    } else {
                        println!("│");
                    }
                }
            }
        }

        let lines_display = format!("[{}]", e.lines).dimmed();

        // H1s (depth 0) are left-aligned with no branch characters
        if depth == 0 {
            // Add blank line before H1 if previous H1 had visible children
            if *prev_h1_had_children {
                println!();
            }
            if show_anchors {
                let anchor = e.anchor.clone().unwrap_or_default();
                println!("{name} {lines_display} {}", anchor.bright_black());
            } else {
                println!("{name} {lines_display}");
            }
        } else {
            // H2+ use tree structure
            let branch = if is_last { "└─ " } else { "├─ " };
            if show_anchors {
                let anchor = e.anchor.clone().unwrap_or_default();
                println!(
                    "{prefix}{branch}{name} {lines_display} {}",
                    anchor.bright_black()
                );
            } else {
                println!("{prefix}{branch}{name} {lines_display}");
            }
        }
        *count += 1;
        *prev_depth = Some(depth);
    }

    let mut had_visible_children = false;

    if can_descend(depth, max_depth) {
        let new_prefix = if depth == 0 {
            // For H1s, children don't get additional prefix since H1 is left-aligned
            String::new()
        } else {
            format!("{}{}  ", prefix, if is_last { " " } else { "│" })
        };

        for (i, c) in e.children.iter().enumerate() {
            if let Some(limit_count) = limit {
                if *count >= limit_count {
                    break;
                }
            }
            let child_is_last = i == e.children.len() - 1;
            let child_printed = print_tree(
                c,
                depth + 1,
                child_is_last,
                &new_prefix,
                max_depth,
                filter,
                level_filter,
                count,
                limit,
                show_anchors,
                prev_depth,
                prev_h1_had_children,
            );
            if child_printed {
                had_visible_children = true;
            }
        }
    }

    // If this is an H1, update the flag for next H1
    if depth == 0 && text_matches && level_matches {
        *prev_h1_had_children = had_visible_children;
    }

    text_matches && level_matches
}

fn display_path(entry: &blz_core::TocEntry) -> Vec<String> {
    entry
        .heading_path_display
        .clone()
        .unwrap_or_else(|| entry.heading_path.clone())
}

fn exceeds_depth(depth: usize, max_depth: Option<usize>) -> bool {
    max_depth.is_some_and(|max| depth + 1 > max)
}

fn can_descend(depth: usize, max_depth: Option<usize>) -> bool {
    max_depth.is_none_or(|max| depth + 1 < max)
}

#[derive(Debug)]
struct HeadingFilter {
    expr: HeadingExpr,
}

impl HeadingFilter {
    fn parse(expr: &str) -> Result<Self> {
        let tokens = tokenize_filter(expr)?;
        let mut parser = FilterParser::new(tokens);
        let parsed = parser.parse_expression()?;
        Ok(Self { expr: parsed })
    }

    fn matches(&self, display_path: &[String], anchor: Option<&str>) -> bool {
        let mut haystack = display_path.join(" ").to_ascii_lowercase();
        if let Some(anchor) = anchor {
            if !anchor.is_empty() {
                haystack.push(' ');
                haystack.push_str(&anchor.to_ascii_lowercase());
            }
        }
        self.expr.matches(&haystack)
    }
}

#[derive(Debug, Clone)]
enum HeadingExpr {
    Term(String),
    And(Vec<Self>),
    Or(Vec<Self>),
    Not(Box<Self>),
}

impl HeadingExpr {
    fn matches(&self, haystack: &str) -> bool {
        match self {
            Self::Term(term) => haystack.contains(term),
            Self::And(terms) => terms.iter().all(|expr| expr.matches(haystack)),
            Self::Or(terms) => terms.iter().any(|expr| expr.matches(haystack)),
            Self::Not(expr) => !expr.matches(haystack),
        }
    }

    fn and(terms: Vec<Self>) -> Self {
        if terms.len() == 1 {
            return terms
                .into_iter()
                .next()
                .unwrap_or_else(|| Self::And(Vec::new()));
        }
        let mut flattened = Vec::new();
        for term in terms {
            match term {
                Self::And(mut inner) => flattened.append(&mut inner),
                _ => flattened.push(term),
            }
        }
        Self::And(flattened)
    }

    fn or(terms: Vec<Self>) -> Self {
        if terms.len() == 1 {
            return terms
                .into_iter()
                .next()
                .unwrap_or_else(|| Self::Or(Vec::new()));
        }
        let mut flattened = Vec::new();
        for term in terms {
            match term {
                Self::Or(mut inner) => flattened.append(&mut inner),
                _ => flattened.push(term),
            }
        }
        Self::Or(flattened)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FilterToken {
    Term(String),
    And,
    Or,
    Not,
    LParen,
    RParen,
}

struct FilterParser {
    tokens: Vec<FilterToken>,
    position: usize,
}

impl FilterParser {
    const fn new(tokens: Vec<FilterToken>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn parse_expression(&mut self) -> Result<HeadingExpr> {
        let expr = self.parse_or()?;
        if let Some(token) = self.peek() {
            return Err(anyhow!(
                "Unexpected token {} in filter expression",
                token.describe()
            ));
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<HeadingExpr> {
        let mut terms = vec![self.parse_and()?];
        loop {
            match self.peek() {
                Some(FilterToken::Or) => {
                    self.advance();
                    terms.push(self.parse_and()?);
                },
                Some(FilterToken::RParen) | None => break,
                Some(token) if Self::starts_expression(token) => {
                    terms.push(self.parse_and()?);
                },
                Some(token) => {
                    return Err(anyhow!(
                        "Unexpected token {} in filter expression",
                        token.describe()
                    ));
                },
            }
        }
        Ok(HeadingExpr::or(terms))
    }

    fn parse_and(&mut self) -> Result<HeadingExpr> {
        let mut terms = vec![self.parse_unary()?];
        while matches!(self.peek(), Some(FilterToken::And)) {
            self.advance();
            terms.push(self.parse_unary()?);
        }
        Ok(HeadingExpr::and(terms))
    }

    fn parse_unary(&mut self) -> Result<HeadingExpr> {
        match self.next() {
            Some(FilterToken::Not) => Ok(HeadingExpr::Not(Box::new(self.parse_unary()?))),
            Some(FilterToken::LParen) => {
                let expr = self.parse_or()?;
                self.expect_rparen()?;
                Ok(expr)
            },
            Some(FilterToken::Term(term)) => Ok(HeadingExpr::Term(term)),
            Some(token) => Err(anyhow!(
                "Unexpected token {} in filter expression",
                token.describe()
            )),
            None => Err(anyhow!("Unexpected end of filter expression")),
        }
    }

    fn expect_rparen(&mut self) -> Result<()> {
        match self.next() {
            Some(FilterToken::RParen) => Ok(()),
            Some(token) => Err(anyhow!(
                "Expected ')' but found {} in filter expression",
                token.describe()
            )),
            None => Err(anyhow!("Unclosed '(' in filter expression")),
        }
    }

    const fn starts_expression(token: &FilterToken) -> bool {
        matches!(
            token,
            FilterToken::Term(_) | FilterToken::Not | FilterToken::LParen
        )
    }

    fn peek(&self) -> Option<&FilterToken> {
        self.tokens.get(self.position)
    }

    fn next(&mut self) -> Option<FilterToken> {
        let token = self.tokens.get(self.position).cloned();
        if token.is_some() {
            self.position += 1;
        }
        token
    }

    const fn advance(&mut self) {
        self.position += 1;
    }
}

impl FilterToken {
    const fn describe(&self) -> &'static str {
        match self {
            Self::Term(_) => "term",
            Self::And => "AND",
            Self::Or => "OR",
            Self::Not => "NOT",
            Self::LParen => "(",
            Self::RParen => ")",
        }
    }
}

fn tokenize_filter(expr: &str) -> Result<Vec<FilterToken>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_quoted = false;
    let mut in_quote = false;
    let mut quote_char = '\0';

    let flush_token = |tokens: &mut Vec<FilterToken>,
                       current: &mut String,
                       current_quoted: &mut bool|
     -> Result<()> {
        if current.is_empty() {
            return Ok(());
        }
        let token = parse_filter_token(current, *current_quoted)?;
        tokens.push(token);
        current.clear();
        *current_quoted = false;
        Ok(())
    };

    for ch in expr.chars() {
        match ch {
            '"' | '\'' => {
                if in_quote {
                    if ch == quote_char {
                        in_quote = false;
                    } else {
                        current.push(ch);
                    }
                } else {
                    in_quote = true;
                    quote_char = ch;
                    current_quoted = true;
                }
            },
            '(' | ')' if !in_quote => {
                flush_token(&mut tokens, &mut current, &mut current_quoted)?;
                tokens.push(if ch == '(' {
                    FilterToken::LParen
                } else {
                    FilterToken::RParen
                });
            },
            c if c.is_whitespace() && !in_quote => {
                flush_token(&mut tokens, &mut current, &mut current_quoted)?;
            },
            _ => current.push(ch),
        }
    }

    if in_quote {
        return Err(anyhow!("Unterminated quote in filter expression"));
    }

    flush_token(&mut tokens, &mut current, &mut current_quoted)?;

    if tokens.is_empty() {
        return Err(anyhow!("Filter expression must include at least one term"));
    }

    Ok(tokens)
}

fn parse_filter_token(raw: &str, quoted: bool) -> Result<FilterToken> {
    if quoted {
        return Ok(FilterToken::Term(raw.to_ascii_lowercase()));
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Encountered empty term in filter expression"));
    }
    let lower = trimmed.to_ascii_lowercase();
    match lower.as_str() {
        "and" | "&&" => Ok(FilterToken::And),
        "or" | "||" => Ok(FilterToken::Or),
        "not" => Ok(FilterToken::Not),
        _ => {
            if trimmed.starts_with('+') {
                return Err(anyhow!(
                    "Filter term '{trimmed}' uses '+'. Use AND instead."
                ));
            }
            if trimmed.starts_with('-') || trimmed.starts_with('!') {
                return Err(anyhow!(
                    "Filter term '{trimmed}' uses a prefix operator. Use NOT instead."
                ));
            }
            Ok(FilterToken::Term(lower))
        },
    }
}

/// Get lines by anchor
#[allow(dead_code, clippy::unused_async)]
pub async fn get_by_anchor(
    alias: &str,
    anchor: &str,
    context: Option<usize>,
    output: OutputFormat,
) -> Result<()> {
    let storage = Storage::new()?;
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .unwrap_or_else(|| alias.to_string());
    // Load JSON metadata (Phase 3: always llms.txt)
    let llms: LlmsJson = storage
        .load_llms_json(&canonical)
        .with_context(|| format!("Failed to load TOC for '{canonical}'"))?;

    #[allow(clippy::items_after_statements)]
    fn find<'a>(list: &'a [blz_core::TocEntry], a: &str) -> Option<&'a blz_core::TocEntry> {
        for e in list {
            if e.anchor.as_deref() == Some(a) {
                return Some(e);
            }
            if let Some(f) = find(&e.children, a) {
                return Some(f);
            }
        }
        None
    }

    let Some(entry) = find(&llms.toc, anchor) else {
        println!("Anchor not found for '{anchor}' in '{canonical}'");
        println!("Hint: run 'blz toc {canonical}' to inspect available headings");
        return Ok(());
    };

    match output {
        OutputFormat::Text => {
            // Convert context to ContextMode
            let context_mode = context.map(crate::cli::ContextMode::Symmetric);
            let requests = vec![RequestSpec {
                alias: alias.to_string(),
                line_expression: entry.lines.clone(),
            }];
            crate::commands::get_lines(
                &requests,
                context_mode.as_ref(),
                false,
                None,
                OutputFormat::Text,
                false,
            )
            .await
        },
        OutputFormat::Json | OutputFormat::Jsonl => {
            // Build content string for the range +/- context
            let file_path = storage.llms_txt_path(&canonical)?;
            let file_content = std::fs::read_to_string(&file_path).with_context(|| {
                format!(
                    "Failed to read llms.txt content from {}",
                    file_path.display()
                )
            })?;
            let all_lines: Vec<&str> = file_content.lines().collect();
            let (body, line_numbers) = extract_content(&entry.lines, context, &all_lines)?;
            let display_path = display_path(entry);
            let obj = serde_json::json!({
                "alias": alias,
                "source": canonical,
                "anchor": anchor,
                "headingPath": display_path,
                "rawHeadingPath": entry.heading_path,
                "headingPathNormalized": entry.heading_path_normalized,
                "lines": entry.lines,
                "lineNumbers": line_numbers,
                "content": body,
            });
            if matches!(output, OutputFormat::Json) {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&obj)
                        .context("Failed to serialize anchor content to JSON")?
                );
            } else {
                println!(
                    "{}",
                    serde_json::to_string(&obj)
                        .context("Failed to serialize anchor content to JSONL")?
                );
            }
            Ok(())
        },
        OutputFormat::Raw => Err(anyhow!(
            "Raw output is not supported for toc listings. Use --format json, jsonl, or text instead."
        )),
    }
}

#[allow(dead_code)]
fn extract_content(
    lines_spec: &str,
    context: Option<usize>,
    all_lines: &[&str],
) -> Result<(String, Vec<usize>)> {
    let ranges = parse_line_ranges(lines_spec)
        .map_err(|_| anyhow::anyhow!("Invalid lines format in anchor entry: {lines_spec}"))?;
    let ctx = context.unwrap_or(0);
    let mut selected: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for r in ranges {
        match r {
            LineRange::Single(line) => add_with_context(&mut selected, line, ctx, all_lines.len()),
            LineRange::Range(start, end) => {
                add_range_with_context(&mut selected, start, end, ctx, all_lines.len());
            },
            LineRange::PlusCount(start, count) => {
                let end = start + count - 1;
                add_range_with_context(&mut selected, start, end, ctx, all_lines.len());
            },
        }
    }
    let mut out = String::new();
    for (i, &ln) in selected.iter().enumerate() {
        if ln == 0 || ln > all_lines.len() {
            continue;
        }
        if i > 0 {
            out.push('\n');
        }
        out.push_str(all_lines[ln - 1]);
    }
    Ok((out, selected.into_iter().collect()))
}

#[allow(dead_code)]
fn add_with_context(
    set: &mut std::collections::BTreeSet<usize>,
    line: usize,
    ctx: usize,
    total: usize,
) {
    let start = line.saturating_sub(ctx + 1);
    let end = (line + ctx).min(total);
    for i in start..end {
        set.insert(i + 1);
    }
}

#[allow(dead_code)]
fn add_range_with_context(
    set: &mut std::collections::BTreeSet<usize>,
    start: usize,
    end: usize,
    ctx: usize,
    total: usize,
) {
    let s = start.saturating_sub(ctx + 1);
    let e = (end + ctx).min(total);
    for i in s..e {
        set.insert(i + 1);
    }
}
