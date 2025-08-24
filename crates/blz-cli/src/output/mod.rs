//! # Output Formatting
//!
//! This module provides comprehensive output formatting capabilities for the `blz` CLI.
//! It supports multiple output formats to accommodate both human interaction and
//! programmatic consumption.
//!
//! ## Supported Formats
//!
//! - **Text**: Human-readable output with colors, alignment, and contextual information
//! - **JSON**: Single JSON object/array for programmatic consumption
//! - **NDJSON**: Newline-delimited JSON for streaming processing
//!
//! ## Architecture
//!
//! The module is organized into specialized formatters:
//!
//! - [`formatter`]: Core formatting abstractions and format selection
//! - [`text`]: Human-readable text output with color coding and alignment
//! - [`json`]: Machine-readable JSON output in various forms
//! - [`progress`]: Progress indicators and status displays
//!
//! ## Usage Patterns
//!
//! Most commands accept an `--output` flag to specify the desired format:
//!
//! ```bash
//! # Human-readable output (default)
//! blz search "useEffect" --output text
//!
//! # JSON for scripts
//! blz list --output json | jq '.[] | .alias'
//!
//! # Streaming JSON for processing
//! blz search "async" --output ndjson | while read line; do
//!     echo "$line" | jq .score
//! done
//! ```
//!
//! ## Design Principles
//!
//! - **Format Independence**: Core logic is separated from presentation
//! - **Consistency**: Same data structures across all output formats
//! - **Performance**: Minimal overhead for formatting operations
//! - **Extensibility**: Easy to add new output formats
//!
//! ## Examples
//!
//! ### Search Results
//!
//! Text format:
//! ```text
//! ┌─ react:12-15 ──────────────────────────────────────────────────────────┐
//! │ useEffect(() => {                                                       │
//! │   // Cleanup function                                                   │
//! │   return () => clearInterval(timer);                                    │
//! │ }, []);                                                                 │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! JSON format:
//! ```json
//! [{
//!   "alias": "react",
//!   "lines": {"start": 12, "end": 15},
//!   "content": "useEffect(() => {\n  // Cleanup function\n  return () => clearInterval(timer);\n}, []);",
//!   "score": 0.89
//! }]
//! ```

mod formatter;
mod json;
mod progress;
mod text;

pub use formatter::{OutputFormat, SearchResultFormatter};

// Re-export commonly used formatters
