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

use clap::{Parser, Subcommand};

use crate::output::OutputFormat;

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
#[command(override_usage = "blz [OPTIONS] [QUERY]... [COMMAND]")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Arguments for default search command
    #[arg(global = true)]
    pub args: Vec<String>,

    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    /// Show detailed performance metrics
    #[arg(long, global = true)]
    pub debug: bool,

    /// Show resource usage (memory, CPU)
    #[arg(long, global = true)]
    pub profile: bool,

    /// Generate CPU flamegraph (requires flamegraph feature)
    #[cfg(feature = "flamegraph")]
    #[arg(long, global = true)]
    pub flamegraph: bool,
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
/// blz sources --output json
/// blz update react
/// blz rm react
///
/// # Content access
/// blz search "useEffect" --limit 5
/// blz get react --lines 120-142 --context 3
///
/// # Utility
/// blz completions bash > ~/.bash_completion.d/blz
/// ```
#[derive(Subcommand, Clone, Debug)]
pub enum Commands {
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Add a new source
    Add {
        /// Alias for the source
        alias: String,
        /// URL to fetch llms.txt from
        url: String,
        /// Auto-select the best flavor without prompts
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Search registries for documentation to add
    Lookup {
        /// Search query (tool name, partial name, etc.)
        query: String,
    },

    /// Search across cached docs
    Search {
        /// Search query
        query: String,
        /// Filter by alias
        #[arg(long)]
        alias: Option<String>,
        /// Maximum number of results
        #[arg(short = 'n', long, default_value = "50")]
        limit: usize,
        /// Show all results (no limit)
        #[arg(long)]
        all: bool,
        /// Page number for pagination
        #[arg(long, default_value = "1")]
        page: usize,
        /// Show only top N percentile of results (1-100)
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
        top: Option<u8>,
        /// Output format
        #[arg(short = 'o', long, value_enum, default_value = "text")]
        output: OutputFormat,
    },

    /// Get exact lines from a source
    Get {
        /// Source alias
        alias: String,
        /// Line range(s) (e.g., "120-142", "36:43,320:350", "36+20")
        #[arg(short = 'l', long)]
        lines: String,
        /// Context lines around each line/range
        #[arg(short = 'c', long)]
        context: Option<usize>,
    },

    /// List all cached sources
    #[command(alias = "sources")]
    List {
        /// Output format
        #[arg(short = 'o', long, value_enum, default_value = "text")]
        output: OutputFormat,
    },

    /// Update sources
    Update {
        /// Specific alias to update (updates all if not specified)
        alias: Option<String>,
        /// Update all sources
        #[arg(long)]
        all: bool,
    },

    /// Remove/delete a source
    #[command(alias = "rm", alias = "delete")]
    Remove {
        /// Source alias
        alias: String,
    },
}
