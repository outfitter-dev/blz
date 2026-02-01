//! Map command implementation - browse documentation structure
//!
//! This module provides the `blz map` command for exploring the heading structure
//! and table of contents of documentation sources. The "map" metaphor fits the
//! trail-blazing theme: map out the docs wilderness.
//!
//! # Examples
//!
//! ```bash
//! blz map bun                    # Show TOC for bun source
//! blz map bun --tree -H 1-2      # Tree view with H1-H2 only
//! blz map --all                  # Show TOC for all sources
//! ```

use anyhow::Result;
use clap::Args;

use crate::output::OutputFormat;
use crate::utils::cli_args::FormatArg;
use crate::utils::heading_filter::HeadingLevelFilter;

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

/// Arguments for `blz map` (browse documentation structure)
#[derive(Args, Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct MapArgs {
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

    /// Show anchor slugs in normal output
    #[arg(short = 'a', long)]
    pub show_anchors: bool,

    /// Continue from previous results (next page)
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

/// Dispatch the map command from CLI args.
///
/// This function takes the parsed `MapArgs` and quiet flag, resolves the output format,
/// and delegates to `execute`.
pub async fn dispatch(args: MapArgs, quiet: bool) -> Result<()> {
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

/// Execute the map command to browse documentation structure
///
/// This command displays the table of contents (heading structure) for documentation
/// sources. It delegates to the internal TOC implementation.
///
/// # Arguments
///
/// * `alias` - Optional source alias (can be omitted with --source or --all)
/// * `sources` - List of source aliases to show
/// * `all` - Include all sources when no alias provided
/// * `output` - Output format (text, json, jsonl)
/// * `anchors` - Show anchor metadata and remap history
/// * `show_anchors` - Show anchor slugs in normal TOC output
/// * `limit` - Maximum number of headings per page
/// * `max_depth` - Limit results to headings at or above this level
/// * `heading_level` - Filter by heading level with comparison operators
/// * `filter_expr` - Filter headings by boolean expression
/// * `tree` - Display as hierarchical tree
/// * `next` - Continue from previous (next page)
/// * `previous` - Go back to previous page
/// * `last` - Jump to last page
/// * `page` - Page number for pagination
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub async fn execute(
    alias: Option<&str>,
    sources: &[String],
    all: bool,
    output: OutputFormat,
    anchors: bool,
    show_anchors: bool,
    limit: Option<usize>,
    max_depth: Option<u8>,
    heading_level: Option<&HeadingLevelFilter>,
    filter_expr: Option<&str>,
    tree: bool,
    next: bool,
    previous: bool,
    last: bool,
    page: usize,
) -> Result<()> {
    // Delegate to internal TOC implementation
    super::toc::execute(
        alias,
        sources,
        all,
        output,
        anchors,
        show_anchors,
        limit,
        max_depth,
        heading_level,
        filter_expr,
        tree,
        next,
        previous,
        last,
        page,
    )
    .await
}
