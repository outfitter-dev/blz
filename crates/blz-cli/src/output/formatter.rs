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
//! - NDJSON enables streaming processing for large result sets
//!
//! ## Examples
//!
//! ```rust,no_run
//! use blz_core::SearchHit;
//! use std::time::Duration;
//!
//! let formatter = SearchResultFormatter::new(OutputFormat::Text);
//! let hits: Vec<SearchHit> = vec![/* search hits */];
//! let params = FormatParams {
//!     hits: &hits[0..10],
//!     query: "useEffect",
//!     total_results: 100,
//!     total_lines_searched: 50_000,
//!     search_time: Duration::from_millis(5),
//!     show_pagination: true,
//!     single_source: false,
//!     sources: &["react".to_string()],
//!     start_idx: 0,
//! };
//! formatter.format(&params)?;
//! ```

use anyhow::Result;
use blz_core::SearchHit;
use std::time::Duration;

use super::{json::JsonFormatter, text::TextFormatter};

/// Parameters for formatting search results
pub struct FormatParams<'a> {
    pub hits: &'a [SearchHit],
    pub query: &'a str,
    pub total_results: usize,
    pub total_lines_searched: usize,
    pub search_time: Duration,
    pub show_pagination: bool,
    pub single_source: bool,
    pub sources: &'a [String],
    pub start_idx: usize,
    pub page: usize,
    pub limit: usize,
    pub total_pages: usize,
    pub suggestions: Option<Vec<serde_json::Value>>, // optional fuzzy suggestions (JSON only)
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
/// - **Ndjson**: Newline-delimited JSON where each line is a separate
///   JSON object. Enables streaming processing and is memory-efficient
///   for large result sets.
///
/// # Usage in CLI
///
/// ```bash
/// # Default text output
/// blz search "useEffect"
///
/// # JSON output for scripting
/// blz search "useEffect" --output json | jq '.[0].content'
///
/// # NDJSON for streaming
/// blz search "useEffect" --output ndjson | head -5
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Pretty text output (default)
    Text,
    /// Single JSON array
    Json,
    /// Newline-delimited JSON
    Ndjson,
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
/// - **NDJSON**: Streaming format for processing large result sets
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
/// ```rust,no_run
/// use std::time::Duration;
///
/// let formatter = SearchResultFormatter::new(OutputFormat::Text);
///
/// // Format results with comprehensive metadata
/// formatter.format(
///     &search_hits,
///     "React hooks",
///     250,                    // total matching results
///     45000,                  // total lines searched  
///     Duration::from_millis(8), // search execution time
///     true,                   // show pagination info
///     false,                  // multiple sources
///     &["react".to_string(), "next".to_string()],
///     0,                      // starting index for pagination
/// )?;
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
    /// ```rust,no_run
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
    /// # Performance
    ///
    /// - Text formatting: Optimized for readability, includes syntax highlighting
    /// - JSON formatting: Single allocation for the entire result set
    /// - NDJSON formatting: Streaming output, memory-efficient for large results
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    ///
    /// let formatter = SearchResultFormatter::new(OutputFormat::Text);
    /// let search_time = Duration::from_millis(12);
    ///
    /// // Display first page of results
    /// let p1 = FormatParams {
    ///     hits: &hits[0..10],
    ///     query: "useEffect cleanup",
    ///     total_results: 156,
    ///     total_lines_searched: 38_000,
    ///     search_time,
    ///     show_pagination: true,
    ///     single_source: false,
    ///     sources: &["react".to_string(), "next".to_string()],
    ///     start_idx: 0,
    /// };
    /// formatter.format(&p1)?;
    ///
    /// // Display a later page
    /// let p3 = FormatParams {
    ///     hits: &hits[20..30],
    ///     query: "useEffect cleanup",
    ///     total_results: 156,
    ///     total_lines_searched: 38_000,
    ///     search_time,
    ///     show_pagination: true,
    ///     single_source: false,
    ///     sources: &["react".to_string(), "next".to_string()],
    ///     start_idx: 20,
    /// };
    /// formatter.format(&p3)?;
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
                    params.limit,
                    params.total_pages,
                    params.sources,
                    suggestions_ref,
                )?;
            },
            OutputFormat::Ndjson => {
                JsonFormatter::format_search_results_ndjson(params.hits)?;
            },
            OutputFormat::Text => {
                TextFormatter::format_search_results(params);
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
/// - **NDJSON**: One source object per line for streaming processing
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
    /// ```rust,no_run
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
    /// // Output as streaming NDJSON
    /// SourceInfoFormatter::format(&sources, OutputFormat::Ndjson)?;
    /// ```
    #[allow(dead_code)]
    pub fn format(source_info: &[serde_json::Value], format: OutputFormat) -> Result<()> {
        match format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(source_info)?;
                println!("{json}");
            },
            OutputFormat::Ndjson => {
                for info in source_info {
                    println!("{}", serde_json::to_string(info)?);
                }
            },
            OutputFormat::Text => {
                // Text formatting is handled in the list command
            },
        }
        Ok(())
    }
}
