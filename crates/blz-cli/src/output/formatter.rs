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
//! let hits = vec![/* search hits */];
//!
//! formatter.format(
//!     &hits,
//!     "useEffect",
//!     100,           // total results
//!     50000,         // lines searched
//!     Duration::from_millis(5), // search time
//!     true,          // show pagination
//!     false,         // single source
//!     &["react".to_string()], // sources
//!     0,             // start index
//! )?;
//! ```

use anyhow::Result;
use blz_core::SearchHit;
use std::time::Duration;

use super::{json::JsonFormatter, text::TextFormatter};

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
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
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
    /// formatter.format(
    ///     &hits[0..10],           // First 10 results
    ///     "useEffect cleanup",    // Original query
    ///     156,                    // Total matches found
    ///     38000,                  // Lines searched
    ///     search_time,
    ///     true,                   // Show "Page 1 of 16" etc.
    ///     false,                  // Multiple sources
    ///     &["react".to_string(), "next".to_string()],
    ///     0,                      // First page (start at 0)
    /// )?;
    ///
    /// // Display specific page
    /// formatter.format(
    ///     &hits[20..30],          // Third page of results
    ///     "useEffect cleanup",
    ///     156,
    ///     38000,
    ///     search_time,
    ///     true,
    ///     false,
    ///     &["react".to_string(), "next".to_string()],
    ///     20,                     // Start at result #20
    /// )?;
    /// ```
    pub fn format(
        &self,
        hits: &[SearchHit],
        query: &str,
        total_results: usize,
        total_lines_searched: usize,
        search_time: Duration,
        show_pagination: bool,
        single_source: bool,
        sources: &[String],
        start_idx: usize,
    ) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                JsonFormatter::format_search_results(hits)?;
            },
            OutputFormat::Ndjson => {
                JsonFormatter::format_search_results_ndjson(hits)?;
            },
            OutputFormat::Text => {
                TextFormatter::format_search_results(
                    hits,
                    query,
                    total_results,
                    total_lines_searched,
                    search_time,
                    show_pagination,
                    single_source,
                    sources,
                    start_idx,
                )?;
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
