//! # Output Formatting Abstractions
//!
//! This module provides the core abstractions for formatting output in different
//! formats. It acts as the central dispatch point for format selection and
//! delegates actual formatting to specialized implementations.
//!
//! ## Architecture
//!
//! The module follows a strategy pattern where:
//!
//! - [`OutputFormat`] defines available output formats
//! - [`SearchResultFormatter`] handles search result formatting
//! - [`SourceInfoFormatter`] handles source information display
//! - Specific formatters ([`JsonFormatter`], [`TextFormatter`]) implement the details
//!
//! ## Format Selection
//!
//! Format selection happens at the CLI parsing level via clap's `ValueEnum` derive,
//! ensuring type-safe format handling throughout the application.
//!
//! ## Performance Considerations
//!
//! - Text formatting optimizes for readability over processing speed
//! - JSON formatting prioritizes machine consumption and parsing speed
//! - JSONL enables streaming processing for large result sets (alias: `ndjson`)
//!
//! ## Examples
//!
//! ```rust,ignore
//! use crate::output::formatter::{FormatParams, SearchResultFormatter};
//! use crate::output::OutputFormat;
//! use blz_core::SearchHit;
//! use std::time::Duration;
//!
//! let formatter = SearchResultFormatter::new(OutputFormat::Text);
//! let hits: Vec<SearchHit> = vec![/* search hits */];
//! let params = FormatParams::new(
//!     &hits[0..10],
//!     "useEffect",
//!     100,
//!     50_000,
//!     Duration::from_millis(5),
//!     &["react".to_string()],
//!     0,
//!     1,
//!     10,
//!     10,
//!     false,
//!     false,
//!     false,
//!     1,
//!     3,
//! );
//! formatter.format(&params)?;
//! ```

use anyhow::Result;
use blz_core::SearchHit;
use std::time::Duration;

use super::{json::JsonFormatter, text::TextFormatter};

/// Parameters for formatting search results
#[allow(clippy::struct_excessive_bools)]
#[non_exhaustive]
pub struct FormatParams<'a> {
    /// Search hits to render.
    pub hits: &'a [SearchHit],
    /// Raw query string.
    pub query: &'a str,
    /// Total hits across all pages.
    pub total_results: usize,
    /// Total lines searched for the query.
    pub total_lines_searched: usize,
    /// Wall-clock search duration.
    pub search_time: Duration,
    /// Source aliases included in the search.
    pub sources: &'a [String],
    /// Zero-based index of the first hit on this page.
    pub start_idx: usize,
    /// Current page number (1-based).
    pub page: usize,
    /// Total pages available.
    pub total_pages: usize,
    /// Page size used for pagination.
    pub page_size: usize,
    /// Whether to include source URLs.
    pub show_url: bool,
    /// Whether to include line numbers.
    pub show_lines: bool,
    /// Whether to include heading anchors.
    pub show_anchor: bool,
    /// Whether to display raw relevance scores.
    pub show_raw_score: bool,
    /// Whether to suppress the summary footer.
    pub no_summary: bool,
    /// Decimal precision used for scores.
    pub score_precision: u8,
    /// Number of context lines per snippet.
    pub snippet_lines: usize,
    /// Optional fuzzy suggestions (JSON output only).
    pub suggestions: Option<Vec<serde_json::Value>>,
}

impl<'a> FormatParams<'a> {
    #[allow(
        clippy::too_many_arguments,
        clippy::fn_params_excessive_bools,
        dead_code,
        clippy::missing_const_for_fn
    )]
    pub fn new(
        hits: &'a [SearchHit],
        query: &'a str,
        total_results: usize,
        total_lines_searched: usize,
        search_time: Duration,
        sources: &'a [String],
        start_idx: usize,
        page: usize,
        total_pages: usize,
        page_size: usize,
        show_url: bool,
        show_lines: bool,
        show_anchor: bool,
        no_summary: bool,
        score_precision: u8,
        snippet_lines: usize,
    ) -> Self {
        Self {
            hits,
            query,
            total_results,
            total_lines_searched,
            search_time,
            sources,
            start_idx,
            page,
            total_pages,
            page_size,
            show_url,
            show_lines,
            show_anchor,
            show_raw_score: false,
            no_summary,
            score_precision,
            snippet_lines,
            suggestions: None,
        }
    }
}

/// Output format options supported by the CLI
///
/// This enum defines the available output formats for various commands.
/// It implements `clap::ValueEnum` to provide automatic command-line
/// argument parsing and validation.
///
/// # Format Descriptions
///
/// - **Text**: Human-readable output with colors, boxes, and formatting
///   optimized for terminal display. Includes contextual information
///   like performance metrics and pagination.
///
/// - **Json**: Single JSON object or array containing all results.
///   Suitable for programmatic consumption and further processing.
///   Output is pretty-printed for readability.
///
/// - **Jsonl**: Newline-delimited JSON (alias: `ndjson`) where each line is a
///   separate JSON object. Enables streaming processing and is memory-efficient
///   for large result sets.
///
/// # Default Behavior
///
/// When no format is specified:
/// - **Interactive terminals**: Text format (human-readable)
/// - **Piped/redirected output**: JSON format (machine-readable)
///
/// This ensures optimal defaults for both human and programmatic usage.
/// Use `--format text` to force text output when piping.
///
/// # Usage in CLI
///
/// ```bash
/// # Default text output (interactive terminal)
/// blz search "useEffect"
///
/// # Pipe automatically uses JSON
/// blz search "useEffect" | jq '.results[0].content'
///
/// # Force text when piping
/// blz search "useEffect" --format text | less
///
/// # JSON Lines for streaming
/// blz search "useEffect" --format jsonl | head -5
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Pretty text output (default for terminals, use --format text to force when piping)
    Text,
    /// Single JSON array (default when output is piped/redirected)
    Json,
    /// Newline-delimited JSON (aka JSON Lines)
    #[value(name = "jsonl", alias = "ndjson")]
    Jsonl,
    /// Raw content only (no formatting, no metadata)
    Raw,
}

/// Formatter for search results with multiple output format support
///
/// This formatter handles the display of search results from blz's search operations.
/// It supports multiple output formats and includes comprehensive information about
/// the search operation, including performance metrics and pagination.
///
/// # Format-Specific Features
///
/// - **Text**: Includes color coding, Unicode box drawing, line numbers,
///   performance statistics, and pagination information
/// - **JSON**: Structured data suitable for programmatic processing
/// - **JSONL**: Streaming format for processing large result sets (alias: `ndjson`)
///
/// # Performance Information
///
/// All formats include performance metadata:
/// - Search execution time
/// - Number of lines searched
/// - Total number of matching results
/// - Sources included in search
///
/// # Examples
///
/// ```rust,ignore
/// use crate::output::formatter::{FormatParams, SearchResultFormatter};
/// use crate::output::OutputFormat;
/// use blz_core::SearchHit;
/// use std::time::Duration;
///
/// # let hits: Vec<SearchHit> = Vec::new();
/// # let sources = vec!["react".to_string()];
/// let params = FormatParams::new(
///     &hits,
///     "React hooks",
///     0,
///     0,
///     Duration::from_millis(8),
///     &sources,
///     0,
///     1,
///     1,
///     10,
///     true,
///     true,
///     false,
///     false,
///     2,
///     6,
/// );
/// SearchResultFormatter::new(OutputFormat::Text)
///     .format(&params)
///     .unwrap();
/// ```
pub struct SearchResultFormatter {
    format: OutputFormat,
}

impl SearchResultFormatter {
    /// Create a new formatter for the specified output format
    ///
    /// # Arguments
    ///
    /// * `format` - The desired output format for search results
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::output::formatter::SearchResultFormatter;
    /// use crate::output::OutputFormat;
    ///
    /// let text_formatter = SearchResultFormatter::new(OutputFormat::Text);
    /// let json_formatter = SearchResultFormatter::new(OutputFormat::Json);
    /// ```
    pub const fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    /// Format and display search results with comprehensive metadata
    ///
    /// This method handles the complete formatting and display of search results,
    /// including performance metrics, pagination information, and source details.
    /// The output format is determined by the formatter's configuration.
    ///
    /// # Arguments
    ///
    /// * `hits` - The search results to format and display
    /// * `query` - The original search query for context
    /// * `total_results` - Total number of matching results (may exceed `hits.len()`)
    /// * `total_lines_searched` - Number of lines searched across all sources
    /// * `search_time` - Time taken to execute the search
    /// * `show_pagination` - Whether to display pagination information
    /// * `single_source` - Whether results are from a single source (affects display)
    /// * `sources` - List of source aliases included in the search
    /// * `start_idx` - Starting index for pagination (0-based)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful formatting, or an error if output fails.
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization or output writing fails.
    ///
    /// # Performance
    ///
    /// - Text formatting: Optimized for readability, includes syntax highlighting
    /// - JSON formatting: Single allocation for the entire result set
    /// - JSONL formatting: Streaming output, memory-efficient for large results (alias: `ndjson`)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// use crate::output::formatter::{FormatParams, SearchResultFormatter};
    /// use crate::output::OutputFormat;
    /// use blz_core::SearchHit;
    /// use std::time::Duration;
    ///
    /// # let hits: Vec<SearchHit> = Vec::new();
    /// # let sources = vec!["react".to_string(), "next".to_string()];
    /// let params = FormatParams::new(
    ///     &hits,
    ///     "useEffect cleanup",
    ///     156,
    ///     38_000,
    ///     Duration::from_millis(12),
    ///     &sources,
    ///     0,
    ///     1,
    ///     16,
    ///     10,
    ///     true,
    ///     true,
    ///     false,
    ///     false,
    ///     2,
    ///     4,
    /// );
    /// SearchResultFormatter::new(OutputFormat::Text)
    ///     .format(&params)
    ///     .unwrap();
    /// ```
    pub fn format(&self, params: &FormatParams) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                let suggestions_ref = params.suggestions.as_deref();
                JsonFormatter::format_search_results_with_meta(
                    params.hits,
                    params.query,
                    params.total_results,
                    params.total_lines_searched,
                    params.search_time,
                    params.page,
                    params.page_size,
                    params.total_pages,
                    params.sources,
                    suggestions_ref,
                    params.show_raw_score,
                    params.score_precision,
                )?;
            },
            OutputFormat::Jsonl => {
                JsonFormatter::format_search_results_jsonl(params.hits)?;
            },
            OutputFormat::Text => {
                TextFormatter::format_search_results(params);
            },
            OutputFormat::Raw => {
                // Raw format: just print snippet from each hit
                for hit in params.hits {
                    println!("{}", hit.snippet);
                }
            },
        }
        Ok(())
    }
}

/// Formatter for source information and metadata display
///
/// This formatter handles the display of information about cached documentation
/// sources, including their status, metadata, and configuration. It supports
/// the same output formats as search results but focuses on source management
/// information rather than content.
///
/// # Information Displayed
///
/// Source information includes:
/// - **Alias**: Short name used to reference the source
/// - **URL**: Original llms.txt URL
/// - **Status**: Whether the source is up-to-date, needs updating, etc.
/// - **Last Updated**: Timestamp of last successful fetch
/// - **Content Stats**: Line count, file size, document count
/// - **Configuration**: Source-specific settings and overrides
///
/// # Output Formats
///
/// - **Text**: Tabular display with aligned columns and status indicators
/// - **JSON**: Array of source objects with complete metadata
/// - **JSONL**: One source object per line for streaming processing (alias: `ndjson`)
///
/// # Examples
///
/// Text format output:
/// ```text
/// ALIAS    STATUS   LINES   UPDATED               URL
/// react    ✓ Fresh  1,247   2024-01-15 09:30:22   https://react.dev/llms.txt
/// next     ⚠ Stale  2,103   2024-01-10 14:22:01   https://nextjs.org/llms.txt
/// ```
///
/// JSON format output:
/// ```json
/// [
///   {
///     "alias": "react",
///     "url": "https://react.dev/llms.txt",
///     "status": "fresh",
///     "lines": 1247,
///     "updated": "2024-01-15T09:30:22Z"
///   }
/// ]
/// ```
#[allow(dead_code)]
pub struct SourceInfoFormatter;

impl SourceInfoFormatter {
    /// Format source information for display in the specified format
    ///
    /// This method takes an array of source information objects (typically from
    /// the list command) and formats them according to the specified output format.
    /// For text format, formatting is handled by the calling command to maintain
    /// proper column alignment and status indicators.
    ///
    /// # Arguments
    ///
    /// * `source_info` - Array of JSON objects containing source metadata
    /// * `format` - The desired output format
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful formatting, or an error if JSON serialization
    /// or output fails.
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails while emitting JSON/JSONL output.
    ///
    /// # JSON Structure
    ///
    /// Each source info object contains:
    /// ```json
    /// {
    ///   "alias": "string",           // Source alias
    ///   "url": "string",             // Original URL
    ///   "status": "string",          // "fresh", "stale", "error", etc.
    ///   "lines": "number",           // Line count in cached content
    ///   "updated": "string",         // ISO 8601 timestamp
    ///   "size": "number",            // File size in bytes
    ///   "etag": "string?",           // HTTP ETag if available
    ///   "error": "string?"           // Error message if status is "error"
    /// }
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    ///
    /// let sources = vec![
    ///     json!({
    ///         "alias": "react",
    ///         "url": "https://react.dev/llms.txt",
    ///         "status": "fresh",
    ///         "lines": 1247,
    ///         "updated": "2024-01-15T09:30:22Z"
    ///     })
    /// ];
    ///
    /// // Output as pretty JSON
    /// SourceInfoFormatter::format(&sources, OutputFormat::Json)?;
    ///
    /// // Output as streaming JSONL
    /// SourceInfoFormatter::format(&sources, OutputFormat::Jsonl)?;
    /// ```
    #[allow(dead_code)]
    pub fn format(source_info: &[serde_json::Value], format: OutputFormat) -> Result<()> {
        match format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(source_info)?;
                println!("{json}");
            },
            OutputFormat::Jsonl => {
                for info in source_info {
                    println!("{}", serde_json::to_string(info)?);
                }
            },
            OutputFormat::Text => {
                // Text formatting is handled in the list command
            },
            OutputFormat::Raw => {
                // Raw format: just names/aliases, one per line
                for info in source_info {
                    if let Some(alias) = info.get("alias").and_then(|v| v.as_str()) {
                        println!("{alias}");
                    }
                }
            },
        }
        Ok(())
    }
}
