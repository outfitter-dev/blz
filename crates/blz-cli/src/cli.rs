//! # CLI Structure and Argument Parsing
//!
//! This module defines the command-line interface for `blz`, a fast local search
//! tool for llms.txt documentation. The CLI is built using `clap` with derive macros
//! for automatic help generation and argument validation.
//!
//! ## Architecture
//!
//! The CLI follows a standard command-subcommand pattern:
//!
//! - **Global options**: Apply to all commands (`--verbose`, `--debug`, `--profile`)
//! - **Subcommands**: Specific operations like `query`, `get`, `add`, `list`, etc.
//!
//! ## Usage Patterns
//!
//! ```bash
//! # Search documentation
//! blz query "React hooks"
//! blz query "async/await" --limit 10
//!
//! # Source management
//! blz add react https://react.dev/llms.txt
//! blz list
//! blz sync --all
//!
//! # Content retrieval
//! blz get react:120-142
//! ```
//!
//! ## Performance Options
//!
//! The CLI includes several performance monitoring options:
//!
//! - `--debug`: Shows detailed performance metrics after operation
//! - `--profile`: Displays memory and CPU usage statistics
//! - `--flamegraph`: Generates CPU flamegraph for performance analysis (requires feature)
//!
//! ## Output Formats
//!
//! Most commands support multiple output formats:
//!
//! - **text**: Human-readable formatted output (default)
//! - **json**: Machine-readable JSON for scripting
//!
//! ## Error Handling
//!
//! The CLI handles errors gracefully with:
//!
//! - User-friendly error messages
//! - Proper exit codes for shell integration
//! - Validation of inputs before execution
//! - Helpful suggestions for common mistakes

use clap::{Args, Parser, Subcommand};

use crate::utils::cli_args::FormatArg;
use std::path::PathBuf;

// Re-export shared types from args module for backward compatibility
pub use crate::args::{ContextMode, ShowComponent, merge_context_flags};

/// Custom help template with grouped command sections
static HELP_TEMPLATE: &str = "\
{before-help}{name} {version} - {about}

{usage-heading} {usage}

Querying:
  query          Full-text search across cached documentation
  get            Retrieve exact lines from a source by citation
  map            Browse documentation structure (headings and sections)

Source Management:
  add            Add a new source
  list           List all cached sources [aliases: sources]
  sync           Fetch latest documentation from sources
  rm             Remove a source and its cached content
  info           Show detailed information about a source
  check          Validate source integrity and availability
  lookup         Search registries for documentation to add

Configuration:
  stats          Show cache statistics and overview
  history        Show recent search history and defaults
  doctor         Run health checks on cache and sources
  clear          Clear the entire cache (removes all sources)
  docs           Bundled documentation hub and CLI reference
  completions    Generate shell completions
  alias          Manage aliases for a source
  registry       Manage the registry
  claude-plugin  Manage the BLZ Claude plugin

{options}{after-help}";

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

/// Main CLI structure for the `blz` command
///
/// This structure defines the top-level CLI interface using clap's derive macros.
/// A subcommand is required for all operations.
///
/// # Global Options
///
/// Global options can be used with any command:
///
/// - `--verbose`: Enable verbose logging output
/// - `--debug`: Show detailed performance metrics after execution
/// - `--profile`: Display resource usage statistics
/// - `--flamegraph`: Generate CPU flamegraph (requires flamegraph feature)
///
/// # Examples
///
/// ```bash
/// # Search documentation
/// blz query "React hooks"
/// blz --debug query "async patterns"
///
/// # Add source with profiling
/// blz --profile add react https://react.dev/llms.txt
///
/// # Retrieve content by citation
/// blz get bun:120-142
/// ```
#[derive(Parser, Clone, Debug)]
#[command(name = "blz")]
#[command(version)]
#[command(about = "Fast local search for llms.txt documentation", long_about = None)]
#[command(override_usage = "blz <COMMAND> [ARGS]... [OPTIONS]")]
#[command(help_template = HELP_TEMPLATE)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Emit agent-focused JSON prompt for the CLI or a specific command
    ///
    /// Example usages:
    /// - `blz --prompt` (general guidance)
    /// - `blz --prompt search` (command-specific guidance)
    #[arg(
        long,
        global = true,
        value_name = "TARGET",
        num_args = 0..=1,
        default_missing_value = "__global__"
    )]
    pub prompt: Option<String>,

    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    /// Suppress informational messages (only show errors)
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    /// Show detailed performance metrics
    #[arg(long, global = true)]
    pub debug: bool,

    /// Show resource usage (memory, CPU)
    #[arg(long, global = true)]
    pub profile: bool,

    /// Disable all ANSI colors in output (also respects `NO_COLOR` env)
    #[arg(long = "no-color", global = true)]
    pub no_color: bool,

    /// Generate CPU flamegraph (requires flamegraph feature)
    #[cfg(feature = "flamegraph")]
    #[arg(long, global = true)]
    pub flamegraph: bool,

    /// Path to configuration file (overrides autodiscovery). Also via `BLZ_CONFIG`.
    #[arg(long, global = true, value_name = "FILE", env = "BLZ_CONFIG")]
    pub config: Option<PathBuf>,
    /// Directory containing config.toml (overrides autodiscovery). Also via `BLZ_CONFIG_DIR`.
    #[arg(
        long = "config-dir",
        global = true,
        value_name = "DIR",
        env = "BLZ_CONFIG_DIR"
    )]
    pub config_dir: Option<PathBuf>,
}

/// Available subcommands for the `blz` CLI
///
/// Each variant represents a distinct operation that can be performed by the CLI.
/// Commands are organized by functionality:
///
/// ## Source Management
/// - [`Add`]: Add a new documentation source
/// - [`Lookup`]: Search registries for documentation to add
/// - [`List`]: List all cached sources
/// - [`Update`]: Update cached content from sources
/// - [`Remove`]: Remove a source and its cached content
///
/// ## Content Access
/// - [`Search`]: Full-text search across cached documentation
/// - [`Get`]: Retrieve specific lines from a source
/// - [`Diff`]: View changes between document versions
///
/// ## Utility
/// - [`Completions`]: Generate shell completion scripts
///
/// # Command Aliases
///
/// Some commands provide convenient aliases:
/// - `list` → `sources`
/// - `remove` → `rm` → `delete`
///
/// # Examples
///
/// ```bash
/// # Source management
/// blz add react https://react.dev/llms.txt
/// blz sources --format json
/// blz refresh react
/// blz rm react
///
/// # Content access
/// blz search "useEffect" --limit 5
/// blz get react --lines 120-142 -C 3
/// blz diff react --since "2024-01-01"
///
/// # Utility
/// blz completions bash > ~/.bash_completion.d/blz
/// ```
#[derive(Subcommand, Clone, Debug)]
pub enum Commands {
    /// Deprecated: use `blz --prompt`
    #[command(hide = true, display_order = 100)]
    Instruct,
    /// Generate shell completions
    #[command(display_order = 51, hide = true)]
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Option<clap_complete::Shell>,
        /// List supported shells instead of generating a script
        #[arg(long)]
        list: bool,
        /// Output format for listing
        #[command(flatten)]
        format: FormatArg,
    },

    /// Manage aliases for a source
    #[command(display_order = 52, hide = true)]
    Alias {
        #[command(subcommand)]
        command: AliasCommands,
    },

    /// Bundled documentation hub and CLI reference export
    #[command(display_order = 50, hide = true)]
    Docs {
        #[command(subcommand)]
        command: Option<DocsCommands>,
    },

    /// Manage the BLZ Claude plugin
    #[command(name = "claude-plugin", display_order = 56, hide = true)]
    ClaudePlugin {
        #[command(subcommand)]
        command: ClaudePluginCommands,
    },

    /// Legacy anchor utilities (use `toc` instead)
    #[command(display_order = 53, hide = true)]
    Anchor {
        #[command(subcommand)]
        command: AnchorCommands,
    },

    /// Show table of contents (deprecated: use `map` instead)
    #[command(display_order = 154, alias = "anchors", hide = true)]
    #[deprecated(since = "1.5.0", note = "use 'map' instead")]
    Toc {
        /// Source alias (optional when using --source or --all)
        alias: Option<String>,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Filter headings by boolean expression (use AND/OR/NOT; whitespace implies OR)
        #[arg(long = "filter", value_name = "EXPR")]
        filter: Option<String>,
        /// Limit results to headings at or above this level (1-6)
        #[arg(
            long = "max-depth",
            value_name = "DEPTH",
            value_parser = clap::value_parser!(u8).range(1..=6)
        )]
        max_depth: Option<u8>,
        /// Filter by heading level with comparison operators (e.g., <=2, >3, 1-3, 1,2,3)
        #[arg(short = 'H', long = "heading-level", value_name = "FILTER")]
        heading_level: Option<crate::utils::heading_filter::HeadingLevelFilter>,
        /// Search specific sources (comma-separated aliases)
        #[arg(
            short = 's',
            long = "source",
            value_name = "ALIASES",
            value_delimiter = ',',
            num_args = 1..,
            conflicts_with = "alias"
        )]
        sources: Vec<String>,
        /// Include all sources when no alias is provided, or bypass pagination limits
        #[arg(long)]
        all: bool,
        /// Display as hierarchical tree with box-drawing characters
        #[arg(long)]
        tree: bool,
        /// Show anchor metadata and remap history
        #[arg(long, alias = "mappings")]
        anchors: bool,
        /// Show anchor slugs in normal TOC output
        #[arg(short = 'a', long)]
        show_anchors: bool,
        /// Continue from previous toc (next page)
        #[arg(
            long,
            conflicts_with = "page",
            conflicts_with = "last",
            conflicts_with = "previous",
            conflicts_with = "all",
            display_order = 50
        )]
        next: bool,
        /// Go back to previous page
        #[arg(
            long,
            conflicts_with = "page",
            conflicts_with = "last",
            conflicts_with = "next",
            conflicts_with = "all",
            display_order = 51
        )]
        previous: bool,
        /// Jump to last page of results
        #[arg(
            long,
            conflicts_with = "next",
            conflicts_with = "page",
            conflicts_with = "previous",
            conflicts_with = "all",
            display_order = 52
        )]
        last: bool,
        /// Maximum number of headings per page (must be at least 1)
        #[arg(
            short = 'n',
            long,
            value_name = "COUNT",
            value_parser = validate_limit,
            display_order = 53
        )]
        limit: Option<usize>,
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
        page: usize,
    },
    /// Add a new source
    #[command(display_order = 1, hide = true)]
    Add(AddArgs),

    /// Full-text search across cached documentation (rejects citations)
    ///
    /// Query Syntax:
    ///   "exact phrase"      Match exact phrase (use single quotes: blz '"exact phrase"')
    ///   +term               Require term (AND)
    ///   term1 term2         Match any term (OR - default)
    ///   +api +key           Require both terms
    ///
    /// For retrieving specific lines by citation, use `blz get` instead.
    ///
    /// Examples:
    ///   blz query "react hooks"         # Search for phrase
    ///   blz query useEffect cleanup     # Search for terms (OR)
    ///   blz query +async +await         # Require both terms (AND)
    #[command(display_order = 5, hide = true)]
    Query(QueryArgs),

    /// Browse documentation structure (headings and sections)
    ///
    /// Navigate the table of contents for indexed sources.
    ///
    /// Examples:
    ///   blz map bun                     # Browse bun docs structure
    ///   blz map bun --tree              # Hierarchical tree view
    ///   blz map --all -H 1-2            # All sources, H1/H2 only
    #[command(display_order = 7, hide = true)]
    Map(MapArgs),

    /// Fetch latest documentation from sources
    ///
    /// Syncs cached documentation with upstream llms.txt files.
    ///
    /// Examples:
    ///   blz sync                        # Sync all sources
    ///   blz sync bun react              # Sync specific sources
    ///   blz sync --reindex              # Force re-index even if unchanged
    #[command(display_order = 10, hide = true)]
    Sync(SyncArgs),

    /// Remove a source and its cached content
    ///
    /// Examples:
    ///   blz rm react                    # Remove with confirmation
    ///   blz rm react -y                 # Remove without prompting
    #[command(display_order = 11, hide = true)]
    Rm(RmArgs),

    /// Validate source integrity and availability
    ///
    /// Check that sources are properly configured and accessible.
    ///
    /// Examples:
    ///   blz check bun                   # Validate single source
    ///   blz check --all                 # Validate all sources
    #[command(display_order = 15, hide = true)]
    Check(CheckArgs),

    /// Search registries for documentation to add
    #[command(display_order = 30, hide = true)]
    Lookup {
        /// Search query (tool name, partial name, etc.)
        query: String,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Maximum number of results to display
        #[arg(short = 'n', long, value_name = "COUNT")]
        limit: Option<usize>,
    },

    /// Manage the registry (create sources, validate, etc.)
    #[command(display_order = 55, hide = true)]
    Registry {
        #[command(subcommand)]
        command: RegistryCommands,
    },

    /// Search across cached docs (deprecated: use `find` instead)
    ///
    /// Query Syntax:
    ///   "exact phrase"      Match exact phrase (use single quotes: blz '"exact phrase"')
    ///   +term               Require term (AND)
    ///   term1 term2         Match any term (OR - default)
    ///   +api +key           Require both terms
    ///
    /// Examples:
    ///   blz find "react hooks"         # Preferred: use find
    ///   blz "react hooks"              # Default command still works
    #[command(display_order = 2, hide = true)]
    Search {
        /// Search query (required unless --next, --previous, or --last)
        #[arg(required_unless_present_any = ["next", "previous", "last"])]
        query: Option<String>,
        /// Filter by source(s) - comma-separated or repeated (-s a -s b)
        #[arg(
            long = "source",
            short = 's',
            visible_alias = "alias",
            visible_alias = "sources",
            value_name = "SOURCE",
            value_delimiter = ','
        )]
        sources: Vec<String>,
        /// Continue from previous search (next page)
        #[arg(
            long,
            conflicts_with = "page",
            conflicts_with = "last",
            conflicts_with = "previous",
            display_order = 50
        )]
        next: bool,
        /// Go back to previous page
        #[arg(
            long,
            conflicts_with = "page",
            conflicts_with = "last",
            conflicts_with = "next",
            display_order = 51
        )]
        previous: bool,
        /// Jump to last page of results
        #[arg(
            long,
            conflicts_with = "next",
            conflicts_with = "page",
            conflicts_with = "previous",
            display_order = 52
        )]
        last: bool,
        /// Maximum number of results per page (default 50; internally fetches up to 3x this value for scoring stability)
        #[arg(
            short = 'n',
            long,
            value_name = "COUNT",
            conflicts_with = "all",
            display_order = 53
        )]
        limit: Option<usize>,
        /// Show all results (no limit)
        #[arg(long, conflicts_with = "limit", display_order = 54)]
        all: bool,
        /// Page number for pagination
        #[arg(
            long,
            default_value = "1",
            conflicts_with = "next",
            conflicts_with = "last",
            display_order = 55
        )]
        page: usize,
        /// Show only top N percentile of results (1-100). Applied after paging is calculated.
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
        top: Option<u8>,
        /// Filter results by heading level
        ///
        /// Supports comparison operators (<=2, >2, >=3, <4, =2), lists (1,2,3), and ranges (1-3).
        ///
        /// Examples:
        ///   -H <=2       # Level 1 and 2 headings only
        ///   -H >2        # Level 3+ headings only
        ///   -H 1,2,3     # Levels 1, 2, and 3 only
        ///   -H 2-4       # Levels 2, 3, and 4 only
        #[arg(short = 'H', long = "heading-level", value_name = "FILTER")]
        heading_level: Option<String>,
        /// Output format (text, json, jsonl)
        #[command(flatten)]
        format: FormatArg,
        /// Additional columns to include in text output
        #[arg(long = "show", value_enum, value_delimiter = ',', env = "BLZ_SHOW")]
        show: Vec<ShowComponent>,
        /// Hide the summary/footer line
        #[arg(long = "no-summary")]
        no_summary: bool,
        /// Number of decimal places to show for scores (0-4)
        #[arg(
            long = "score-precision",
            value_name = "PLACES",
            value_parser = clap::value_parser!(u8).range(0..=4),
            env = "BLZ_SCORE_PRECISION"
        )]
        score_precision: Option<u8>,
        /// Maximum snippet lines to display around a hit (1-10)
        #[arg(
            long = "snippet-lines",
            value_name = "LINES",
            value_parser = clap::value_parser!(u8).range(1..=10),
            env = "BLZ_SNIPPET_LINES",
            default_value_t = 3,
            hide = true
        )]
        snippet_lines: u8,
        /// Maximum total characters in snippet (including newlines). Range: 50-1000, default: 200.
        #[arg(
            long = "max-chars",
            value_name = "CHARS",
            env = "BLZ_MAX_CHARS",
            value_parser = clap::value_parser!(usize)
        )]
        max_chars: Option<usize>,
        /// Print LINES lines of context (both before and after match). Same as -C.
        ///
        /// Use "all" to expand to the full heading section containing the match.
        /// If no heading encompasses the match, returns only the matched lines.
        ///
        /// Examples:
        ///   -C 10              # 10 lines before and after
        ///   -C all             # Expand to containing heading section
        ///   --context 5        # Long form (also valid)
        #[arg(
            short = 'C',
            long = "context",
            value_name = "LINES",
            num_args = 0..=1,
            default_missing_value = "5",
            allow_hyphen_values = false,
            conflicts_with_all = ["block", "context_deprecated"],
            display_order = 30
        )]
        context: Option<ContextMode>,
        /// Deprecated: use -C or --context instead (hidden for backward compatibility)
        #[arg(
            short = 'c',
            value_name = "LINES",
            num_args = 0..=1,
            default_missing_value = "5",
            allow_hyphen_values = false,
            conflicts_with_all = ["block", "context"],
            hide = true,
            display_order = 100
        )]
        context_deprecated: Option<ContextMode>,
        /// Print LINES lines of context after each match
        ///
        /// Examples:
        ///   -A3                # 3 lines after match
        ///   --after-context 5  # 5 lines after match
        #[arg(
            short = 'A',
            long = "after-context",
            value_name = "LINES",
            num_args = 0..=1,
            default_missing_value = "5",
            allow_hyphen_values = false,
            conflicts_with = "block",
            display_order = 31
        )]
        after_context: Option<usize>,
        /// Print LINES lines of context before each match
        ///
        /// Examples:
        ///   -B3                # 3 lines before match
        ///   --before-context 5 # 5 lines before match
        #[arg(
            short = 'B',
            long = "before-context",
            value_name = "LINES",
            num_args = 0..=1,
            default_missing_value = "5",
            allow_hyphen_values = false,
            conflicts_with = "block",
            display_order = 32
        )]
        before_context: Option<usize>,
        /// Expand to the full heading section containing each hit.
        ///
        /// If no heading encompasses the range, returns only the requested lines.
        /// Legacy alias for --context all.
        #[arg(long, conflicts_with_all = ["context", "context_deprecated", "after_context", "before_context"], display_order = 33)]
        block: bool,
        /// Maximum number of lines to include when using block expansion (--block or --context all)
        #[arg(
            long = "max-lines",
            value_name = "LINES",
            value_parser = clap::value_parser!(usize),
            display_order = 34
        )]
        max_lines: Option<usize>,
        /// Restrict matches to heading text only
        #[arg(long = "headings-only", display_order = 35)]
        headings_only: bool,
        /// Don't save this search to history
        #[arg(long = "no-history")]
        no_history: bool,
        /// Copy results to clipboard using OSC 52 escape sequence
        #[arg(long)]
        copy: bool,
        /// Show detailed timing breakdown for performance analysis
        #[arg(long)]
        timing: bool,
    },

    /// Show recent search history and defaults (last 20 entries by default)
    ///
    /// Displays the last 20 searches unless `--limit` is provided to override the count.
    #[command(display_order = 14, hide = true)]
    History {
        /// Maximum number of entries to display
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Clear all search history
        #[arg(long, conflicts_with = "clear_before")]
        clear: bool,
        /// Clear search history before the specified date (format: YYYY-MM-DD or ISO 8601)
        #[arg(long = "clear-before", value_name = "DATE", conflicts_with = "clear")]
        clear_before: Option<String>,
    },
    // Config command removed in v1.0.0-beta.1 - flavor preferences eliminated
    /// Retrieve exact lines from a source by citation
    ///
    /// Syntax: `blz get alias:start-end` or `blz get alias --lines start-end`
    ///
    /// Multiple spans can be comma-separated:
    /// `blz get bun:120-142,200-210`
    ///
    /// Examples:
    ///   blz get bun:120-142             # Single range
    ///   blz get bun:120-142 -C 5        # With context
    ///   blz get bun:120-142,200-210     # Multiple ranges
    ///   blz get bun deno:5-10           # Multiple sources
    #[command(display_order = 6, hide = true)]
    Get {
        /// One or more `alias[:ranges]` targets (preferred: matches search output, e.g., "bun:1-3")
        ///
        /// The --lines flag remains available for single-target compatibility.
        #[arg(value_name = "ALIAS[:RANGES]", num_args = 1..)]
        targets: Vec<String>,
        /// Explicit source alias (use when positional alias is ambiguous)
        #[arg(long = "source", short = 's', value_name = "SOURCE")]
        source: Option<String>,
        /// Line range(s) to retrieve
        ///
        /// Format: "120-142", "36:43,320:350", "36+20", "1,5,10-15"
        ///
        /// Can be omitted if using colon syntax (e.g., "bun:1-3")
        #[arg(short = 'l', long, value_name = "RANGE")]
        lines: Option<String>,
        /// Print LINES lines of context (both before and after). Same as -C.
        ///
        /// Use "all" to expand to the full heading section containing the range.
        /// If no heading encompasses the range, returns only the requested lines.
        ///
        /// Examples:
        ///   -C 10              # 10 lines before and after
        ///   -C all             # Expand to containing heading section
        ///   --context 5        # Long form (also valid)
        #[arg(
            short = 'C',
            long = "context",
            value_name = "LINES",
            num_args = 0..=1,
            default_missing_value = "5",
            allow_hyphen_values = false,
            conflicts_with_all = ["block", "context_deprecated"],
            display_order = 30
        )]
        context: Option<ContextMode>,
        /// Deprecated: use -C or --context instead (hidden for backward compatibility)
        #[arg(
            short = 'c',
            value_name = "LINES",
            num_args = 0..=1,
            default_missing_value = "5",
            allow_hyphen_values = false,
            conflicts_with_all = ["block", "context"],
            hide = true,
            display_order = 100
        )]
        context_deprecated: Option<ContextMode>,
        /// Print LINES lines of context after each line/range
        ///
        /// Examples:
        ///   -A3                # 3 lines after
        ///   --after-context 5  # 5 lines after
        #[arg(
            short = 'A',
            long = "after-context",
            value_name = "LINES",
            num_args = 0..=1,
            default_missing_value = "5",
            allow_hyphen_values = false,
            conflicts_with = "block",
            display_order = 31
        )]
        after_context: Option<usize>,
        /// Print LINES lines of context before each line/range
        ///
        /// Examples:
        ///   -B3                # 3 lines before
        ///   --before-context 5 # 5 lines before
        #[arg(
            short = 'B',
            long = "before-context",
            value_name = "LINES",
            num_args = 0..=1,
            default_missing_value = "5",
            allow_hyphen_values = false,
            conflicts_with = "block",
            display_order = 32
        )]
        before_context: Option<usize>,
        /// Expand to the full heading section containing the range.
        ///
        /// If no heading encompasses the range, returns only the requested lines.
        /// Legacy alias for --context all.
        #[arg(long, conflicts_with_all = ["context", "context_deprecated", "after_context", "before_context"], display_order = 33)]
        block: bool,
        /// Maximum number of lines to include when using block expansion (--block or --context all)
        #[arg(
            long = "max-lines",
            value_name = "LINES",
            value_parser = clap::value_parser!(usize),
            display_order = 34
        )]
        max_lines: Option<usize>,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Copy output to clipboard using OSC 52 escape sequence
        #[arg(long)]
        copy: bool,
    },

    /// Show detailed information about a source
    #[command(display_order = 12, hide = true)]
    Info {
        /// Source to inspect
        alias: String,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
    },

    /// List all cached sources
    #[command(visible_alias = "sources", display_order = 4, hide = true)]
    List {
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Include status/health information (etag, lastModified, checksum)
        #[arg(long)]
        status: bool,
        /// Show descriptor metadata (description, category, tags, origin)
        #[arg(long)]
        details: bool,
        /// Maximum number of sources to display
        #[arg(short = 'n', long, value_name = "COUNT")]
        limit: Option<usize>,
    },

    /// Show cache statistics and overview
    #[command(display_order = 13, hide = true)]
    Stats {
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Maximum number of sources to display in statistics
        #[arg(short = 'n', long, value_name = "COUNT")]
        limit: Option<usize>,
    },

    /// Validate source integrity (deprecated: use `check` instead)
    #[command(display_order = 115, hide = true)]
    #[deprecated(since = "1.5.0", note = "use 'check' instead")]
    Validate {
        /// Source to validate (validates all if not specified)
        alias: Option<String>,
        /// Validate all sources
        #[arg(long)]
        all: bool,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
    },

    /// Run health checks on cache and sources
    #[command(display_order = 16, hide = true)]
    Doctor {
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Fix issues automatically where possible
        #[arg(long)]
        fix: bool,
    },

    /// Refresh sources (deprecated: use `sync` instead)
    #[command(display_order = 110, hide = true)]
    #[deprecated(since = "1.5.0", note = "use 'sync' instead")]
    Refresh {
        /// Source aliases to refresh (refreshes all if omitted)
        #[arg(
            value_name = "ALIAS",
            num_args = 0..,
            conflicts_with = "all"
        )]
        aliases: Vec<String>,
        /// Refresh all sources
        #[arg(long, conflicts_with = "aliases")]
        all: bool,
        /// Apply changes without prompting (e.g., auto-upgrade to llms-full)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Force re-parse and re-index even if content unchanged
        #[arg(long)]
        reindex: bool,
        /// Enable content filters (comma-separated: lang). Use --filter with no value to enable all filters.
        ///
        /// Available filters:
        ///   lang,language  - Filter non-English content
        ///
        /// Examples:
        ///   --filter           # Enable all filters
        ///   --filter lang      # Only language filter
        ///   --no-filter        # Disable all filters
        #[arg(long, value_name = "FILTERS", num_args = 0..=1, default_missing_value = "all", conflicts_with = "no_filter")]
        filter: Option<String>,
        /// Disable all content filters for this refresh
        #[arg(long, conflicts_with = "filter")]
        no_filter: bool,
    },

    /// Update sources (deprecated: use 'refresh' instead)
    #[command(hide = true)]
    #[deprecated(since = "1.4.0", note = "use 'refresh' command instead")]
    Update {
        /// Source aliases to refresh (refreshes all if omitted)
        #[arg(
            value_name = "ALIAS",
            num_args = 0..,
            conflicts_with = "all"
        )]
        aliases: Vec<String>,
        /// Refresh all sources
        #[arg(long, conflicts_with = "aliases")]
        all: bool,
        /// Apply changes without prompting (e.g., auto-upgrade to llms-full)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Remove/delete a source (deprecated: use `rm` instead)
    #[command(alias = "delete", display_order = 111, hide = true)]
    #[deprecated(since = "1.5.0", note = "use 'rm' instead")]
    Remove {
        /// Source to remove
        alias: String,
        /// Apply removal without prompting
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Clear the entire cache (removes all sources and their data)
    #[command(display_order = 17, hide = true)]
    Clear {
        /// Skip confirmation prompt
        #[arg(short = 'f', long = "force")]
        force: bool,
    },

    /// View diffs (coming soon)
    #[command(hide = true, display_order = 101)]
    Diff {
        /// Source to compare
        alias: String,
        /// Show changes since timestamp
        #[arg(long)]
        since: Option<String>,
    },

    #[command(name = "mcp-server", hide = true)]
    McpServer,

    /// Unified find command (deprecated: use `query` or `get` instead)
    ///
    /// Smart pattern detection:
    /// - If input matches `alias:digits-digits` format → retrieve mode (like get)
    /// - Otherwise → search mode (like search)
    ///
    /// Examples:
    ///   blz find "async patterns"        # Search mode → use `blz query`
    ///   blz find bun:120-142             # Retrieve mode → use `blz get`
    #[command(display_order = 105, hide = true)]
    #[deprecated(since = "1.5.0", note = "use 'query' or 'get' instead")]
    Find {
        /// Query terms or citation(s) (e.g., "query" or "alias:123-456")
        #[arg(value_name = "INPUT", required = true, num_args = 1..)]
        inputs: Vec<String>,

        /// Filter by source(s) for search mode - comma-separated or repeated (-s a -s b)
        #[arg(
            long = "source",
            short = 's',
            visible_alias = "alias",
            visible_alias = "sources",
            value_name = "SOURCE",
            value_delimiter = ','
        )]
        sources: Vec<String>,

        /// Maximum number of results per page (search mode only)
        #[arg(short = 'n', long, value_name = "COUNT", conflicts_with = "all")]
        limit: Option<usize>,

        /// Show all results - no limit (search mode only)
        #[arg(long, conflicts_with = "limit")]
        all: bool,

        /// Page number for pagination (search mode only)
        #[arg(long, default_value = "1")]
        page: usize,

        /// Show only top N percentile of results (1-100, search mode only)
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
        top: Option<u8>,

        /// Filter results by heading level (search mode only)
        ///
        /// Supports comparison operators (<=2, >2, >=3, <4, =2), lists (1,2,3), and ranges (1-3).
        ///
        /// Examples:
        ///   -H <=2       # Level 1 and 2 headings only
        ///   -H >2        # Level 3+ headings only
        ///   -H 1,2,3     # Levels 1, 2, and 3 only
        ///   -H 2-4       # Levels 2, 3, and 4 only
        #[arg(short = 'H', long = "heading-level", value_name = "FILTER")]
        heading_level: Option<String>,

        /// Output format (text, json, jsonl)
        #[command(flatten)]
        format: FormatArg,

        /// Additional columns to include in text output (search mode only)
        #[arg(long = "show", value_enum, value_delimiter = ',', env = "BLZ_SHOW")]
        show: Vec<ShowComponent>,

        /// Hide the summary/footer line (search mode only)
        #[arg(long = "no-summary")]
        no_summary: bool,

        /// Number of decimal places to show for scores (0-4, search mode only)
        #[arg(
            long = "score-precision",
            value_name = "PLACES",
            value_parser = clap::value_parser!(u8).range(0..=4),
            env = "BLZ_SCORE_PRECISION"
        )]
        score_precision: Option<u8>,

        /// Maximum snippet lines to display around a hit (1-10, search mode only)
        #[arg(
            long = "snippet-lines",
            value_name = "LINES",
            value_parser = clap::value_parser!(u8).range(1..=10),
            env = "BLZ_SNIPPET_LINES",
            default_value_t = 3,
            hide = true
        )]
        snippet_lines: u8,

        /// Maximum total characters in snippet (search mode, range: 50-1000, default: 200)
        #[arg(
            long = "max-chars",
            value_name = "CHARS",
            env = "BLZ_MAX_CHARS",
            value_parser = clap::value_parser!(usize)
        )]
        max_chars: Option<usize>,

        /// Print LINES lines of context (both before and after match). Same as -C.
        ///
        /// Use "all" to expand to the full heading section containing the match.
        /// If no heading encompasses the match, returns only the matched lines.
        #[arg(
            short = 'C',
            long = "context",
            value_name = "LINES",
            num_args = 0..=1,
            default_missing_value = "5",
            allow_hyphen_values = false,
            conflicts_with_all = ["block", "context_deprecated"],
            display_order = 30
        )]
        context: Option<ContextMode>,

        /// Deprecated: use -C or --context instead
        #[arg(
            short = 'c',
            value_name = "LINES",
            num_args = 0..=1,
            default_missing_value = "5",
            allow_hyphen_values = false,
            conflicts_with_all = ["block", "context"],
            hide = true,
            display_order = 100
        )]
        context_deprecated: Option<ContextMode>,

        /// Print LINES lines of context after each match
        #[arg(
            short = 'A',
            long = "after-context",
            value_name = "LINES",
            num_args = 0..=1,
            default_missing_value = "5",
            allow_hyphen_values = false,
            conflicts_with = "block",
            display_order = 31
        )]
        after_context: Option<usize>,

        /// Print LINES lines of context before each match
        #[arg(
            short = 'B',
            long = "before-context",
            value_name = "LINES",
            num_args = 0..=1,
            default_missing_value = "5",
            allow_hyphen_values = false,
            conflicts_with = "block",
            display_order = 32
        )]
        before_context: Option<usize>,

        /// Expand to the full heading section containing each hit.
        ///
        /// If no heading encompasses the range, returns only the requested lines.
        /// Legacy alias for --context all.
        #[arg(long, conflicts_with_all = ["context", "context_deprecated", "after_context", "before_context"], display_order = 33)]
        block: bool,

        /// Maximum number of lines to include when using block expansion
        #[arg(
            long = "max-lines",
            value_name = "LINES",
            value_parser = clap::value_parser!(usize),
            display_order = 34
        )]
        max_lines: Option<usize>,

        /// Restrict matches to heading text only (search mode only)
        #[arg(long = "headings-only", display_order = 35)]
        headings_only: bool,

        /// Don't save this search to history (search mode only)
        #[arg(long = "no-history")]
        no_history: bool,

        /// Copy results to clipboard using OSC 52 escape sequence
        #[arg(long)]
        copy: bool,

        /// Show detailed timing breakdown for performance analysis
        #[arg(long)]
        timing: bool,
    },
}

/// Subcommands for `blz docs`.
#[derive(Subcommand, Clone, Debug)]
pub enum DocsCommands {
    /// Search the bundled `blz-docs` source without touching other aliases.
    #[command(alias = "find")]
    Search(DocsSearchArgs),
    /// Sync (or resync) the embedded documentation files and index.
    Sync {
        /// Rebuild even when hashes already match.
        #[arg(long)]
        force: bool,
        /// Suppress status output (errors still emit).
        #[arg(long)]
        quiet: bool,
    },
    /// Print a concise quick-start overview for humans and agents.
    Overview,
    /// Print the entire bundled llms-full.txt to stdout.
    Cat,
    /// Export autogenerated CLI docs (clap schema) in markdown or JSON.
    Export {
        /// Output format for docs export (defaults to markdown).
        #[arg(long = "format", value_enum, default_value = "markdown")]
        format: crate::commands::DocsFormat,
    },
}

/// Installation scope for Claude plugin installs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum ClaudePluginScope {
    /// Install for the current user.
    #[value(name = "user")]
    User,
    /// Install for the current project.
    #[value(name = "project")]
    Project,
}

/// Subcommands for `blz claude-plugin`.
#[derive(Subcommand, Clone, Copy, Debug)]
pub enum ClaudePluginCommands {
    /// Install the local Claude plugin from this repository.
    Install {
        /// Installation scope for Claude Code.
        #[arg(long, value_enum, default_value = "user")]
        scope: ClaudePluginScope,
    },
}

/// Arguments accepted by the `blz docs search` subcommand.
#[derive(Args, Clone, Debug)]
pub struct DocsSearchArgs {
    /// Query terms passed directly to the Tantivy searcher.
    #[arg(value_name = "QUERY", trailing_var_arg = true, num_args = 1..)]
    pub query: Vec<String>,
    /// Maximum number of hits to return.
    #[arg(long, default_value_t = 20, value_name = "N")]
    pub limit: usize,
    /// Optional percentile cap applied before pagination.
    #[arg(long, value_name = "PERCENT")]
    pub top: Option<u8>,
    /// Render format (text/json/jsonl) and `--json` convenience flag.
    #[command(flatten)]
    pub format: FormatArg,
    /// Show optional metadata columns alongside each hit.
    #[arg(long = "show", value_enum, value_name = "COMPONENT")]
    pub show: Vec<ShowComponent>,
    /// Skip match summaries and show headings only.
    #[arg(long)]
    pub no_summary: bool,
    /// Number of snippet lines to include per hit.
    #[arg(long, default_value_t = 4, value_name = "LINES", hide = true)]
    pub snippet_lines: u8,
    /// Maximum total characters in snippet (including newlines). Range: 50-1000, default: 200.
    #[arg(long = "max-chars", value_name = "CHARS", value_parser = clap::value_parser!(usize))]
    pub max_chars: Option<usize>,
    /// Add surrounding context lines.
    #[arg(long, value_name = "LINES")]
    pub context: Option<usize>,
    /// Emit contiguous paragraphs instead of discrete snippet blocks.
    #[arg(long)]
    pub block: bool,
    /// Maximum number of lines when --block is used.
    #[arg(long, value_name = "LINES")]
    pub max_block_lines: Option<usize>,
    /// Override default score precision.
    #[arg(long, value_name = "DIGITS")]
    pub score_precision: Option<u8>,
    /// Copy results to the system clipboard when supported.
    #[arg(long)]
    pub copy: bool,
}

/// Arguments for `blz query` (full-text search, rejects citations)
#[derive(Args, Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct QueryArgs {
    /// Search query terms (not citations - use `get` for retrieval)
    #[arg(value_name = "QUERY", required = true, num_args = 1..)]
    pub inputs: Vec<String>,

    /// Filter by source(s) - comma-separated or repeated (-s a -s b)
    #[arg(
        long = "source",
        short = 's',
        visible_alias = "alias",
        visible_alias = "sources",
        value_name = "SOURCE",
        value_delimiter = ','
    )]
    pub sources: Vec<String>,

    /// Maximum number of results per page
    #[arg(short = 'n', long, value_name = "COUNT", conflicts_with = "all")]
    pub limit: Option<usize>,

    /// Show all results - no limit
    #[arg(long, conflicts_with = "limit")]
    pub all: bool,

    /// Page number for pagination
    #[arg(long, default_value = "1")]
    pub page: usize,

    /// Show only top N percentile of results (1-100)
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
    pub top: Option<u8>,

    /// Filter results by heading level
    ///
    /// Supports comparison operators (<=2, >2, >=3, <4, =2), lists (1,2,3), and ranges (1-3).
    #[arg(short = 'H', long = "heading-level", value_name = "FILTER")]
    pub heading_level: Option<String>,

    /// Output format (text, json, jsonl)
    #[command(flatten)]
    pub format: FormatArg,

    /// Additional columns to include in text output
    #[arg(long = "show", value_enum, value_delimiter = ',', env = "BLZ_SHOW")]
    pub show: Vec<ShowComponent>,

    /// Hide the summary/footer line
    #[arg(long = "no-summary")]
    pub no_summary: bool,

    /// Number of decimal places to show for scores (0-4)
    #[arg(
        long = "score-precision",
        value_name = "PLACES",
        value_parser = clap::value_parser!(u8).range(0..=4),
        env = "BLZ_SCORE_PRECISION"
    )]
    pub score_precision: Option<u8>,

    /// Maximum snippet lines to display around a hit (1-10)
    #[arg(
        long = "snippet-lines",
        value_name = "LINES",
        value_parser = clap::value_parser!(u8).range(1..=10),
        env = "BLZ_SNIPPET_LINES",
        default_value_t = 3,
        hide = true
    )]
    pub snippet_lines: u8,

    /// Maximum total characters in snippet (range: 50-1000, default: 200)
    #[arg(
        long = "max-chars",
        value_name = "CHARS",
        env = "BLZ_MAX_CHARS",
        value_parser = clap::value_parser!(usize)
    )]
    pub max_chars: Option<usize>,

    /// Print LINES lines of context (both before and after match). Same as -C.
    ///
    /// Use "all" to expand to the full heading section containing the match.
    /// If no heading encompasses the match, returns only the matched lines.
    #[arg(
        short = 'C',
        long = "context",
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "5",
        allow_hyphen_values = false,
        conflicts_with_all = ["block", "context_deprecated"],
        display_order = 30
    )]
    pub context: Option<ContextMode>,

    /// Deprecated: use -C or --context instead
    #[arg(
        short = 'c',
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "5",
        allow_hyphen_values = false,
        conflicts_with_all = ["block", "context"],
        hide = true,
        display_order = 100
    )]
    pub context_deprecated: Option<ContextMode>,

    /// Print LINES lines of context after each match
    #[arg(
        short = 'A',
        long = "after-context",
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "5",
        allow_hyphen_values = false,
        conflicts_with = "block",
        display_order = 31
    )]
    pub after_context: Option<usize>,

    /// Print LINES lines of context before each match
    #[arg(
        short = 'B',
        long = "before-context",
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "5",
        allow_hyphen_values = false,
        conflicts_with = "block",
        display_order = 32
    )]
    pub before_context: Option<usize>,

    /// Expand to the full heading section containing each hit.
    ///
    /// If no heading encompasses the range, returns only the requested lines.
    /// Legacy alias for --context all.
    #[arg(long, conflicts_with_all = ["context", "context_deprecated", "after_context", "before_context"], display_order = 33)]
    pub block: bool,

    /// Maximum number of lines to include when using block expansion
    #[arg(
        long = "max-lines",
        value_name = "LINES",
        value_parser = clap::value_parser!(usize),
        display_order = 34
    )]
    pub max_lines: Option<usize>,

    /// Restrict matches to heading text only
    #[arg(long = "headings-only", display_order = 35)]
    pub headings_only: bool,

    /// Don't save this search to history
    #[arg(long = "no-history")]
    pub no_history: bool,

    /// Copy results to clipboard using OSC 52 escape sequence
    #[arg(long)]
    pub copy: bool,

    /// Show detailed timing breakdown for performance analysis
    #[arg(long)]
    pub timing: bool,
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
    pub heading_level: Option<crate::utils::heading_filter::HeadingLevelFilter>,

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

/// Arguments for `blz sync` (fetch latest docs)
#[derive(Args, Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct SyncArgs {
    /// Source aliases to sync
    #[arg(
        value_name = "ALIAS",
        num_args = 0..,
        conflicts_with = "all"
    )]
    pub aliases: Vec<String>,

    /// Sync all sources
    #[arg(long, conflicts_with = "aliases")]
    pub all: bool,

    /// Apply changes without prompting (e.g., auto-upgrade to llms-full)
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,

    /// Force re-parse and re-index even if content unchanged
    #[arg(long)]
    pub reindex: bool,

    /// Enable content filters (comma-separated: lang). Use --filter with no value to enable all filters.
    ///
    /// Available filters:
    ///   lang,language  - Filter non-English content
    ///
    /// Examples:
    ///   --filter           # Enable all filters
    ///   --filter lang      # Only language filter
    ///   --no-filter        # Disable all filters
    #[arg(long, value_name = "FILTERS", num_args = 0..=1, default_missing_value = "all", conflicts_with = "no_filter")]
    pub filter: Option<String>,

    /// Disable all content filters for this sync
    #[arg(long, conflicts_with = "filter")]
    pub no_filter: bool,
}

/// Arguments for `blz check` (validate sources)
#[derive(Args, Clone, Debug)]
pub struct CheckArgs {
    /// Source to validate (validates all if not specified)
    pub alias: Option<String>,

    /// Validate all sources
    #[arg(long)]
    pub all: bool,

    /// Output format
    #[command(flatten)]
    pub format: FormatArg,
}

/// Arguments for `blz rm` (remove sources)
#[derive(Args, Clone, Debug)]
pub struct RmArgs {
    /// Source to remove
    pub alias: String,

    /// Apply removal without prompting
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,
}

/// Arguments for `blz add`
#[derive(Args, Clone, Debug)]
pub struct AddArgs {
    /// Source name (used as identifier)
    #[arg(value_name = "ALIAS", required_unless_present_any = ["manifest"])]
    pub alias: Option<String>,

    /// URL to fetch llms.txt from
    #[arg(value_name = "URL", required_unless_present_any = ["manifest"], requires = "alias")]
    pub url: Option<String>,

    /// Path to a manifest TOML describing multiple sources
    #[arg(long, value_name = "FILE")]
    pub manifest: Option<PathBuf>,

    /// Restrict manifest processing to specific aliases
    #[arg(long = "only", value_delimiter = ',', requires = "manifest")]
    pub only: Vec<String>,

    /// Additional aliases for this source (comma-separated, e.g., "react-docs,@react/docs")
    #[arg(long, value_delimiter = ',')]
    pub aliases: Vec<String>,

    /// Display name for the source (defaults to a Title Case version of the alias)
    #[arg(long)]
    pub name: Option<String>,

    /// Description for the source (plain text)
    #[arg(long)]
    pub description: Option<String>,

    /// Category label used for grouping (defaults to "uncategorized")
    #[arg(long)]
    pub category: Option<String>,

    /// Tags to associate with the source (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,

    /// Skip confirmation prompts (non-interactive mode)
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Analyze source without adding it (outputs JSON analysis)
    #[arg(long)]
    pub dry_run: bool,

    /// Disable language filtering (keep all languages)
    ///
    /// By default, BLZ filters non-English content from multilingual documentation.
    /// Use this flag to keep all languages.
    ///
    /// Examples:
    ///   blz add anthropic <https://docs.anthropic.com/llms-full.txt> --no-language-filter
    #[arg(long)]
    pub no_language_filter: bool,
}

#[derive(Subcommand, Clone, Debug)]
pub enum AnchorCommands {
    /// List table-of-contents entries (headings) for a source
    List {
        /// Source alias
        alias: String,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Show anchor metadata and remap history
        #[arg(long, alias = "mappings")]
        anchors: bool,
        /// Maximum number of headings to display
        #[arg(short = 'n', long, value_name = "COUNT")]
        limit: Option<usize>,
        /// Limit results to headings at or above this level (1-6)
        #[arg(
            long = "max-depth",
            value_name = "DEPTH",
            value_parser = clap::value_parser!(u8).range(1..=6)
        )]
        max_depth: Option<u8>,
        /// Filter headings by boolean expression (use AND/OR/NOT; whitespace implies OR)
        #[arg(long = "filter", value_name = "EXPR")]
        filter: Option<String>,
    },
    /// Get content by anchor
    Get {
        /// Source alias
        alias: String,
        /// Anchor value (from list)
        anchor: String,
        /// Context lines around the section
        #[arg(short = 'c', long)]
        context: Option<usize>,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub enum AliasCommands {
    /// Add an alias for a source
    Add {
        /// Canonical source
        source: String,
        /// Alias to add (e.g., @scope/package)
        alias: String,
    },
    /// Remove an alias from a source
    Rm {
        /// Canonical source
        source: String,
        /// Alias to remove
        alias: String,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub enum RegistryCommands {
    /// Create a new source in the registry
    CreateSource {
        /// Source name/alias
        name: String,
        /// URL to fetch llms.txt from
        #[arg(long)]
        url: String,
        /// Description of the source
        #[arg(long)]
        description: Option<String>,
        /// Category (library, framework, language, tool, etc.)
        #[arg(long)]
        category: Option<String>,
        /// Tags (comma-separated)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// NPM package names (comma-separated)
        #[arg(long, value_delimiter = ',')]
        npm: Vec<String>,
        /// GitHub repositories (comma-separated)
        #[arg(long, value_delimiter = ',')]
        github: Vec<String>,
        /// Also add this source to your local index after creating
        #[arg(long)]
        add: bool,
        /// Skip confirmation prompts (non-interactive mode)
        #[arg(short = 'y', long)]
        yes: bool,
    },
}
