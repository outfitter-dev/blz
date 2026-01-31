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

use clap::{Parser, Subcommand};

use crate::utils::cli_args::FormatArg;
use std::path::PathBuf;

// Re-export shared types from args module for backward compatibility
pub use crate::args::{ContextMode, ShowComponent, merge_context_flags};
// Re-export sub-enums and Args structs from commands module
pub use crate::commands::{
    AddArgs, AliasCommands, AnchorCommands, CheckArgs, ClaudePluginCommands, DocsCommands,
    DocsSearchArgs, FindArgs, MapArgs, QueryArgs, RegistryCommands, RmArgs, SearchArgs, SyncArgs,
    TocArgs,
};

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
    Toc(TocArgs),
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

    /// Search across cached docs (deprecated: use `query` instead)
    ///
    /// Query Syntax:
    ///   "exact phrase"      Match exact phrase (use single quotes: blz '"exact phrase"')
    ///   +term               Require term (AND)
    ///   term1 term2         Match any term (OR - default)
    ///   +api +key           Require both terms
    ///
    /// Examples:
    ///   blz query "react hooks"        # Preferred: use query
    ///   blz "react hooks"              # Default command still works
    #[command(display_order = 2, hide = true)]
    Search(SearchArgs),

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
    Find(FindArgs),
}
