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

use crate::output::OutputFormat;
use crate::utils::heading_filter::HeadingLevelFilter;

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
