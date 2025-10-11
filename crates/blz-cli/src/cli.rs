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
//! blz update --all
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
/// blz update react
/// blz rm react
///
/// # Content access
/// blz search "useEffect" --limit 5
/// blz get react --lines 120-142 --context 3
/// blz diff react --since "2024-01-01"
///
/// # Utility
/// blz completions bash > ~/.bash_completion.d/blz
/// ```
#[derive(Subcommand, Clone, Debug)]
pub enum Commands {
    /// Deprecated: use `blz --prompt`
    #[command(hide = true)]
    Instruct,
    /// Generate shell completions
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
    Alias {
        #[command(subcommand)]
        command: AliasCommands,
    },

    /// Bundled documentation hub and CLI reference export
    Docs {
        #[command(subcommand)]
        command: Option<DocsCommands>,
        /// Back-compat: `blz docs --format json` still exports CLI docs
        #[arg(long = "format", value_enum)]
        format: Option<crate::commands::DocsFormat>,
    },

    /// Anchor utilities
    Anchor {
        #[command(subcommand)]
        command: AnchorCommands,
    },

    /// Show anchors for a source or remap mappings
    Anchors {
        /// Source alias
        alias: String,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Show anchors remap mappings if available
        #[arg(long)]
        mappings: bool,
    },
    /// Add a new source
    Add(AddArgs),

    /// Search registries for documentation to add
    Lookup {
        /// Search query (tool name, partial name, etc.)
        query: String,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
    },

    /// Manage the registry (create sources, validate, etc.)
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
    Search {
        /// Search query (required unless --next or --last)
        #[arg(required_unless_present_any = ["next", "last"])]
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
        #[arg(long, conflicts_with = "page", conflicts_with = "last")]
        next: bool,
        /// Jump to last page of results
        #[arg(long, conflicts_with = "next", conflicts_with = "page")]
        last: bool,
        /// Maximum number of results per page (default 50; internally fetches up to 3x this value for scoring stability)
        #[arg(short = 'n', long, value_name = "COUNT", conflicts_with = "all")]
        limit: Option<usize>,
        /// Show all results (no limit)
        #[arg(long, conflicts_with = "limit")]
        all: bool,
        /// Page number for pagination
        #[arg(
            long,
            default_value = "1",
            conflicts_with = "next",
            conflicts_with = "last"
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
            default_value_t = 3
        )]
        snippet_lines: u8,
        /// Return surrounding context lines or full section (defaults to 5 lines when no value supplied)
        ///
        /// Use 'all' to return the full heading block containing each hit.
        /// Use a number to return that many context lines around the hit.
        ///
        /// Examples:
        ///   -c 10           # 10 lines of context
        ///   --context all   # Full section expansion
        #[arg(
            short = 'c',
            long = "context",
            value_name = "LINES|all",
            num_args = 0..=1,
            default_missing_value = "5",
            conflicts_with = "block"
        )]
        context: Option<ContextMode>,
        /// Return the full heading block containing each hit (legacy alias for --context all)
        #[arg(long, conflicts_with = "context")]
        block: bool,
        /// Maximum number of lines to include when using block expansion (--block or --context all)
        #[arg(
            long = "max-lines",
            value_name = "LINES",
            value_parser = clap::value_parser!(usize)
        )]
        max_lines: Option<usize>,
        /// Don't save this search to history
        #[arg(long = "no-history")]
        no_history: bool,
        /// Copy results to clipboard using OSC 52 escape sequence
        #[arg(long)]
        copy: bool,
    },

    /// Show recent search history and defaults
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
    /// Preferred syntax matches search results: `blz get bun:120-142`
    ///
    /// `--lines` remains available for compatibility: `blz get bun --lines 120-142`
    Get {
        /// Source or "source:lines" (preferred: matches search output, e.g., "bun:1-3")
        ///
        /// When using colon syntax, the --lines flag is optional
        #[arg(value_name = "ALIAS")]
        alias: String,
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
        /// Context lines around each line/range, or 'all' for full section
        ///
        /// Examples:
        ///   -c 10       # 10 lines of context
        ///   --context all  # Full section expansion
        #[arg(short = 'c', long, conflicts_with = "block")]
        context: Option<ContextMode>,
        /// Return the full heading block containing the range (legacy alias for --context all)
        #[arg(long, conflicts_with = "context")]
        block: bool,
        /// Maximum number of lines to include when using block expansion (--block or --context all)
        #[arg(
            long = "max-lines",
            value_name = "LINES",
            value_parser = clap::value_parser!(usize)
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
    Info {
        /// Source to inspect
        alias: String,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
    },

    /// List all cached sources
    #[command(visible_alias = "sources")]
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
    },

    /// Show cache statistics and overview
    Stats {
        /// Output format
        #[command(flatten)]
        format: FormatArg,
    },

    /// Validate source integrity and availability
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
    Doctor {
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Fix issues automatically where possible
        #[arg(long)]
        fix: bool,
    },

    /// Update sources
    Update {
        /// Source to update (updates all if not specified)
        alias: Option<String>,
        /// Update all sources
        #[arg(long)]
        all: bool,
        /// Apply changes without prompting (e.g., auto-upgrade to llms-full)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Remove/delete a source
    #[command(alias = "rm", alias = "delete")]
    Remove {
        /// Source to remove
        alias: String,
        /// Apply removal without prompting
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Clear the entire cache (removes all sources and their data)
    Clear {
        /// Skip confirmation prompt
        #[arg(short = 'f', long = "force")]
        force: bool,
    },

    /// View diffs (coming soon)
    #[command(hide = true)]
    Diff {
        /// Source to compare
        alias: String,
        /// Show changes since timestamp
        #[arg(long)]
        since: Option<String>,
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
    #[arg(long, default_value_t = 4, value_name = "LINES")]
    pub snippet_lines: u8,
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
    #[arg(long)]
    pub no_language_filter: bool,
}

/// Context mode for result expansion
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextMode {
    /// Fixed number of context lines
    Lines(usize),
    /// Full section/block expansion
    All,
}

impl std::str::FromStr for ContextMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("all") {
            Ok(Self::All)
        } else {
            s.parse::<usize>()
                .map(Self::Lines)
                .map_err(|_| format!("Invalid context value: '{s}'. Expected a number or 'all'"))
        }
    }
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
    /// List anchors for a source
    List {
        /// Source alias
        alias: String,
        /// Output format
        #[command(flatten)]
        format: FormatArg,
        /// Show anchors remap mappings if available
        #[arg(long)]
        mappings: bool,
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
