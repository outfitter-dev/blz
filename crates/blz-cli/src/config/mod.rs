//! Runtime configuration for CLI command execution.
//!
//! This module provides resolved configuration types that bundle common
//! execution parameters, reducing the number of arguments passed to commands.
//!
//! # Design Philosophy
//!
//! Rather than passing many individual parameters to execute functions,
//! configuration is bundled into typed structs:
//!
//! ```ignore
//! // Before: 15+ parameters
//! pub async fn execute(
//!     query: &str,
//!     limit: usize,
//!     format: OutputFormat,
//!     quiet: bool,
//!     metrics: PerformanceMetrics,
//!     // ... many more
//! ) -> Result<()>
//!
//! // After: structured configs
//! pub async fn execute(
//!     args: &QueryArgs,
//!     search: &SearchConfig,
//!     display: &DisplayConfig,
//!     snippet: &SnippetConfig,
//!     content: &ContentConfig,
//!     // ...
//! ) -> Result<()>
//! ```
//!
//! # Available Types
//!
//! - [`ExecutionConfig`] - Common execution context for all commands
//! - [`SearchConfig`] - Search/query parameters (limit, page, filters)
//! - [`SnippetConfig`] - Snippet display parameters (lines, `max_chars`, precision)
//! - [`ContentConfig`] - Content retrieval parameters (context, block expansion)
//! - [`DisplayConfig`] - Output formatting parameters (format, show, summary)
//!
//! # Examples
//!
//! ```ignore
//! use blz_cli::config::{
//!     ContentConfig, DisplayConfig, ExecutionConfig, SearchConfig, SnippetConfig,
//! };
//! use blz_cli::args::Verbosity;
//! use blz_cli::output::OutputFormat;
//! use blz_core::PerformanceMetrics;
//!
//! let exec_config = ExecutionConfig::new(
//!     Verbosity::Normal,
//!     OutputFormat::Json,
//!     PerformanceMetrics::new(),
//! );
//!
//! let search_config = SearchConfig::new()
//!     .with_limit(20)
//!     .with_page(1);
//!
//! let display_config = DisplayConfig::new(OutputFormat::Json)
//!     .with_no_summary(false);
//!
//! // Use with command execution
//! commands::execute(&args, &search_config, &display_config, ...).await?;
//! ```

mod content;
mod display;
mod query;
mod resolved;
mod search;
mod snippet;

pub use content::ContentConfig;
pub use display::DisplayConfig;
pub use query::QueryExecutionConfig;
pub use resolved::ExecutionConfig;
pub use search::SearchConfig;
pub use snippet::SnippetConfig;
