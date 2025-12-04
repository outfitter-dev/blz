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
//! - **Default command**: When no subcommand is specified, performs search
//! - **Subcommands**: Specific operations like `add`, `search`, `list`, etc.
//!
//! ## Usage Patterns
//!
//! ```bash
//! # Default search (no subcommand)
//! blz "React hooks"
//! blz useEffect cleanup
//!
//! # Explicit search command
//! blz search "async/await" --limit 10
//!
//! # Source management
//! blz add react https://react.dev/llms.txt
//! blz list
//! blz refresh --all
//!
//! # Content retrieval
//! blz get react --lines 120-142
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
/// It supports both a default search mode (when no subcommand is provided) and
/// explicit subcommands for specific operations.
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
/// # Default Behavior
///
/// When invoked without a subcommand, `blz` performs a search using the provided
/// arguments as the search query:
///
/// ```bash
/// blz "React hooks"  # Equivalent to: blz search "React hooks"
/// ```
///
/// # Examples
///
/// ```bash
/// # Search with debugging enabled
/// blz --debug search "async patterns"
///
/// # Add source with profiling
/// blz --profile add react https://react.dev/llms.txt
///
/// # Generate flamegraph while searching
/// blz --flamegraph search "performance optimization"
/// ```
#[derive(Parser, Clone, Debug)]
#[command(name = "blz")]
#[command(version)]
#[command(about = "blz - Fast local search for llms.txt documentation", long_about = None)]
#[command(
    override_usage = "blz [COMMAND] [COMMAND_ARGS]... [OPTIONS]\n       blz [QUERY]... [OPTIONS]"
)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Positional query arguments used when no explicit command is provided
    #[arg(value_name = "QUERY", trailing_var_arg = true)]
    pub query: Vec<String>,

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
    #[command(display_order = 51)]
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
    #[command(display_order = 52)]
    Alias {
        #[command(subcommand)]
        command: AliasCommands,
    },

    /// Bundled documentation hub and CLI reference export
    #[command(display_order = 50)]
    Docs {
        #[command(subcommand)]
        command: Option<DocsCommands>,
    },

    /// Legacy anchor utilities (use `toc` instead)
    #[command(display_order = 53, hide = true)]
    Anchor {
        #[command(subcommand)]
        command: AnchorCommands,
    },

    /// Show table of contents (headings) for a source
    #[command(display_order = 54, alias = "anchors")]
    Toc {
        /// Source alias (optional when using --source or --all)
        alias: Option<String>,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Filter headings by boolean expression (use +term for AND, -term for NOT)
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
    #[command(display_order = 1)]
    Add(AddArgs),

    /// Search registries for documentation to add
    #[command(display_order = 30)]
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
    #[command(display_order = 55)]
    Registry {
        #[command(subcommand)]
        command: RegistryCommands,
    },

    /// Search across cached docs
    ///
    /// Query Syntax:
    ///   "exact phrase"      Match exact phrase (use single quotes: blz '"exact phrase"')
    ///   +term               Require term (AND)
    ///   term1 term2         Match any term (OR - default)
    ///   +api +key           Require both terms
    ///
    /// Examples:
    ///   blz "react hooks"              # Search all sources
    ///   blz '+api +key'                # Require both terms
    ///   blz '"exact phrase"'           # Exact phrase match
    ///   blz search "async" -s bun      # Search specific source
    #[command(display_order = 2)]
    Search {
        /// Search query (required unless --next, --previous, or --last)
        #[arg(required_unless_present_any = ["next", "previous", "last"])]
        query: Option<String>,
        /// Filter by source(s) - comma-separated for multiple
        #[arg(
            long = "source",
            short = 's',
            visible_alias = "alias",
            visible_alias = "sources",
            value_name = "SOURCE",
            value_delimiter = ',',
            num_args = 0..
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
        /// Examples:
        ///   -C 10              # 10 lines before and after
        ///   -C all             # Full section expansion
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
        /// Return the full heading block containing each hit (legacy alias for --context all)
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
    },

    /// Show recent search history and defaults (last 20 entries by default)
    ///
    /// Displays the last 20 searches unless `--limit` is provided to override the count.
    #[command(display_order = 14)]
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
    /// Get exact lines from a source
    ///
    /// Preferred syntax matches search output: `blz get bun:120-142`
    ///
    /// Multiple spans from the same source can be comma-separated:
    /// `blz get bun:120-142,200-210`
    ///
    /// `--lines` remains available for compatibility: `blz get bun --lines 120-142`
    #[command(display_order = 3)]
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
        /// Examples:
        ///   -C 10              # 10 lines before and after
        ///   -C all             # Full section expansion
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
        /// Return the full heading block containing the range (legacy alias for --context all)
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
    #[command(display_order = 12)]
    Info {
        /// Source to inspect
        alias: String,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
    },

    /// List all cached sources
    #[command(visible_alias = "sources", display_order = 4)]
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
    #[command(display_order = 13)]
    Stats {
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Maximum number of sources to display in statistics
        #[arg(short = 'n', long, value_name = "COUNT")]
        limit: Option<usize>,
    },

    /// Validate source integrity and availability
    #[command(display_order = 15)]
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
    #[command(display_order = 16)]
    Doctor {
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Fix issues automatically where possible
        #[arg(long)]
        fix: bool,
    },

    /// Refresh sources by fetching latest content
    #[command(display_order = 10)]
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

    /// Remove/delete a source
    #[command(alias = "rm", alias = "delete", display_order = 11)]
    Remove {
        /// Source to remove
        alias: String,
        /// Apply removal without prompting
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Clear the entire cache (removes all sources and their data)
    #[command(display_order = 17)]
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

    /// Launch MCP server for AI agent integration
    ///
    /// Starts the BLZ MCP (Model Context Protocol) server over stdio transport.
    /// This enables AI agents like Claude Desktop to use BLZ for documentation search
    /// via the standardized MCP protocol.
    ///
    /// The server runs until interrupted with SIGINT (Ctrl+C) or SIGTERM.
    Mcp,
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

/// Context mode for result expansion
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextMode {
    /// Symmetric context (same before and after)
    Symmetric(usize),
    /// Asymmetric context (different before and after)
    Asymmetric { before: usize, after: usize },
    /// Full section/block expansion
    All,
}

impl ContextMode {
    /// Get the before and after context line counts
    ///
    /// Returns (before, after) tuple. For All mode, returns None.
    #[must_use]
    pub const fn lines(&self) -> Option<(usize, usize)> {
        match self {
            Self::Symmetric(n) => Some((*n, *n)),
            Self::Asymmetric { before, after } => Some((*before, *after)),
            Self::All => None,
        }
    }

    /// Merge two context modes, taking the maximum value for each direction
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            // All takes precedence over everything
            (Self::All, _) | (_, Self::All) => Self::All,
            // Extract line counts and compute maximum for each direction
            (a, b) => {
                let (a_before, a_after) = a.lines().unwrap_or((0, 0));
                let (b_before, b_after) = b.lines().unwrap_or((0, 0));
                let before = a_before.max(b_before);
                let after = a_after.max(b_after);
                if before == after {
                    Self::Symmetric(before)
                } else {
                    Self::Asymmetric { before, after }
                }
            },
        }
    }
}

impl std::str::FromStr for ContextMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("all") {
            Ok(Self::All)
        } else {
            s.parse::<usize>()
                .map(Self::Symmetric)
                .map_err(|_| format!("Invalid context value: '{s}'. Expected a number or 'all'"))
        }
    }
}

/// Merge context flags from CLI arguments into a single `ContextMode`
///
/// Implements grep-style merging logic:
/// - `-C` takes precedence as symmetric context
/// - `-A` and `-B` can be combined for asymmetric context
/// - If multiple flags are provided, takes maximum value for each direction
/// - Supports deprecated `-c` flag for backward compatibility
#[must_use]
pub fn merge_context_flags(
    context: Option<ContextMode>,
    context_deprecated: Option<ContextMode>,
    after_context: Option<usize>,
    before_context: Option<usize>,
) -> Option<ContextMode> {
    // Start with the primary context flag (or deprecated -c flag)
    let mut result = context.or(context_deprecated);

    // Merge in -A and -B flags if present
    if let Some(after) = after_context {
        let new_mode = before_context
            .map_or(ContextMode::Asymmetric { before: 0, after }, |before| {
                ContextMode::Asymmetric { before, after }
            });

        result = Some(match result.take() {
            Some(existing) => existing.merge(new_mode),
            None => new_mode,
        });
    } else if let Some(before) = before_context {
        // Only -B specified, create asymmetric mode with 0 after
        let new_mode = ContextMode::Asymmetric { before, after: 0 };
        result = Some(match result.take() {
            Some(existing) => existing.merge(new_mode),
            None => new_mode,
        });
    }

    result.map(|mode| match mode {
        ContextMode::Asymmetric { before, after } if before == after => {
            ContextMode::Symmetric(before)
        },
        other => other,
    })
}

/// Additional columns that can be displayed in text search results
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum ShowComponent {
    /// Include the global rank prefix (1., 2., ...)
    Rank,
    /// Display the source URL header for aliases present on the page
    Url,
    /// Prefix snippet lines with their line numbers
    Lines,
    /// Show the hashed section anchor above the snippet
    Anchor,
    /// Show raw BM25 scores instead of percentages
    #[value(name = "raw-score")]
    RawScore,
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
        /// Filter headings by boolean expression (use +term for AND, -term for NOT)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_mode_lines() {
        assert_eq!(ContextMode::Symmetric(5).lines(), Some((5, 5)));
        assert_eq!(
            ContextMode::Asymmetric {
                before: 3,
                after: 7
            }
            .lines(),
            Some((3, 7))
        );
        assert_eq!(ContextMode::All.lines(), None);
    }

    #[test]
    fn test_context_mode_merge_symmetric() {
        let mode1 = ContextMode::Symmetric(3);
        let mode2 = ContextMode::Symmetric(5);
        assert_eq!(mode1.merge(mode2), ContextMode::Symmetric(5));
    }

    #[test]
    fn test_context_mode_merge_asymmetric() {
        let mode1 = ContextMode::Asymmetric {
            before: 3,
            after: 5,
        };
        let mode2 = ContextMode::Asymmetric {
            before: 7,
            after: 2,
        };
        assert_eq!(
            mode1.merge(mode2),
            ContextMode::Asymmetric {
                before: 7,
                after: 5
            }
        );
    }

    #[test]
    fn test_context_mode_merge_with_all() {
        let mode1 = ContextMode::Symmetric(5);
        let mode2 = ContextMode::All;
        assert_eq!(mode1.clone().merge(mode2.clone()), ContextMode::All);
        assert_eq!(mode2.merge(mode1), ContextMode::All);
    }

    #[test]
    fn test_context_mode_merge_becomes_asymmetric() {
        let mode1 = ContextMode::Symmetric(3);
        let mode2 = ContextMode::Asymmetric {
            before: 5,
            after: 2,
        };
        assert_eq!(
            mode1.merge(mode2),
            ContextMode::Asymmetric {
                before: 5,
                after: 3
            }
        );
    }

    #[test]
    fn test_context_mode_merge_becomes_symmetric() {
        let mode1 = ContextMode::Asymmetric {
            before: 5,
            after: 3,
        };
        let mode2 = ContextMode::Asymmetric {
            before: 3,
            after: 5,
        };
        assert_eq!(mode1.merge(mode2), ContextMode::Symmetric(5));
    }

    #[test]
    fn test_merge_context_flags_none() {
        assert_eq!(merge_context_flags(None, None, None, None), None);
    }

    #[test]
    fn test_merge_context_flags_only_context() {
        let result = merge_context_flags(Some(ContextMode::Symmetric(5)), None, None, None);
        assert_eq!(result, Some(ContextMode::Symmetric(5)));
    }

    #[test]
    fn test_merge_context_flags_only_deprecated() {
        let result = merge_context_flags(None, Some(ContextMode::Symmetric(3)), None, None);
        assert_eq!(result, Some(ContextMode::Symmetric(3)));
    }

    #[test]
    fn test_merge_context_flags_context_wins_over_deprecated() {
        let result = merge_context_flags(
            Some(ContextMode::Symmetric(5)),
            Some(ContextMode::Symmetric(3)),
            None,
            None,
        );
        assert_eq!(result, Some(ContextMode::Symmetric(5)));
    }

    #[test]
    fn test_merge_context_flags_only_after() {
        let result = merge_context_flags(None, None, Some(3), None);
        assert_eq!(
            result,
            Some(ContextMode::Asymmetric {
                before: 0,
                after: 3
            })
        );
    }

    #[test]
    fn test_merge_context_flags_only_before() {
        let result = merge_context_flags(None, None, None, Some(5));
        assert_eq!(
            result,
            Some(ContextMode::Asymmetric {
                before: 5,
                after: 0
            })
        );
    }

    #[test]
    fn test_merge_context_flags_both_after_and_before() {
        let result = merge_context_flags(None, None, Some(3), Some(5));
        assert_eq!(
            result,
            Some(ContextMode::Asymmetric {
                before: 5,
                after: 3
            })
        );
    }

    #[test]
    fn test_merge_context_flags_context_plus_after() {
        let result = merge_context_flags(Some(ContextMode::Symmetric(2)), None, Some(5), None);
        assert_eq!(
            result,
            Some(ContextMode::Asymmetric {
                before: 2,
                after: 5
            })
        );
    }

    #[test]
    fn test_merge_context_flags_context_plus_before() {
        let result = merge_context_flags(Some(ContextMode::Symmetric(2)), None, None, Some(5));
        assert_eq!(
            result,
            Some(ContextMode::Asymmetric {
                before: 5,
                after: 2
            })
        );
    }

    #[test]
    fn test_merge_context_flags_context_plus_both() {
        let result = merge_context_flags(Some(ContextMode::Symmetric(2)), None, Some(3), Some(5));
        assert_eq!(
            result,
            Some(ContextMode::Asymmetric {
                before: 5,
                after: 3
            })
        );
    }

    #[test]
    fn test_merge_context_flags_all_with_after_before() {
        // All should take precedence even when -A/-B are present
        let result = merge_context_flags(Some(ContextMode::All), None, Some(3), Some(5));
        assert_eq!(result, Some(ContextMode::All));
    }

    #[test]
    fn test_merge_context_flags_asymmetric_plus_after_before() {
        let result = merge_context_flags(
            Some(ContextMode::Asymmetric {
                before: 2,
                after: 4,
            }),
            None,
            Some(6),
            Some(3),
        );
        assert_eq!(
            result,
            Some(ContextMode::Asymmetric {
                before: 3,
                after: 6
            })
        );
    }
}
