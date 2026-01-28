//! # blz-core
//!
//! Core functionality for blz - a fast, local search cache for llms.txt documentation.
//!
//! This crate provides the foundational components for parsing, storing, and searching
//! llms.txt documentation files locally. It's designed for speed (sub-10ms search latency),
//! offline-first usage, and exact line citations.
//!
//! ## Architecture
//!
//! The crate is organized around several key components:
//!
//! - **Configuration**: Global and per-source settings management
//! - **Parsing**: Tree-sitter based markdown parsing with structured output
//! - **Types**: Core data structures representing sources, search results, and metadata
//! - **Error Handling**: Comprehensive error types with categorization and recovery hints
//!
//! ## Quick Start
//!
//! ```rust
//! use blz_core::{Config, MarkdownParser, Result};
//!
//! // Load global configuration
//! let config = Config::load()?;
//!
//! // Parse markdown content
//! let mut parser = MarkdownParser::new()?;
//! let result = parser.parse("# Hello World\n\nThis is content.")?;
//!
//! println!("Found {} heading blocks", result.heading_blocks.len());
//! println!("Generated TOC with {} entries", result.toc.len());
//! # Ok::<(), blz_core::Error>(())
//! ```
//!
//! ## Performance Characteristics
//!
//! - **Parse time**: < 150ms per MB of markdown content
//! - **Memory usage**: < 2x source document size during parsing
//! - **Thread safety**: All types are `Send + Sync` where appropriate
//!
//! ## Error Handling
//!
//! All operations return [`Result<T, Error>`] with structured error information:
//!
//! ```rust
//! use blz_core::{Error, MarkdownParser};
//!
//! let mut parser = MarkdownParser::new()?;
//! match parser.parse("malformed content") {
//!     Ok(result) => println!("Parsed successfully"),
//!     Err(Error::Parse(msg)) => eprintln!("Parse error: {}", msg),
//!     Err(e) if e.is_recoverable() => eprintln!("Recoverable error: {}", e),
//!     Err(e) => eprintln!("Fatal error: {}", e),
//! }
//! # Ok::<(), blz_core::Error>(())
//! ```

/// Configuration management for global and per-source settings
pub mod config;
/// Documentation source discovery
pub mod discovery;
/// Error types and result aliases
pub mod error;
/// HTTP fetching with conditional requests support
pub mod fetcher;
/// Heading sanitization and normalization helpers
pub mod heading;
/// Search index implementation using Tantivy
pub mod index;
/// JSON builder helpers for llms.json structures
pub mod json_builder;
/// Language filtering for multilingual llms.txt files
pub mod language_filter;
/// Anchor remapping utilities between versions
pub mod mapping;
/// Safe numeric conversion helpers
pub mod numeric;
/// Tree-sitter based markdown parser
pub mod parser;
/// Application profile detection helpers
pub mod profile;
/// Performance profiling utilities
pub mod profiling;
/// Refresh helpers shared across CLI and MCP
pub mod refresh;
/// Built-in registry of known documentation sources
pub mod registry;
/// Local filesystem storage for cached documentation
pub mod storage;
/// Core data types and structures
pub mod types;
/// URL resolver for llms.txt variants
pub mod url_resolver;

// Re-export commonly used types
pub use config::{
    Config, DefaultsConfig, FetchConfig, FollowLinks, IndexConfig, PathsConfig, ToolConfig,
    ToolMeta,
};
pub use discovery::{ProbeResult, probe_domain};
pub use error::{Error, Result};
pub use fetcher::{FetchResult, Fetcher};
pub use heading::{
    HeadingPathVariants, HeadingSegmentVariants, normalize_text_for_search, path_variants,
    segment_variants,
};
pub use index::SearchIndex;
pub use json_builder::build_llms_json;
pub use language_filter::{FilterStats, LanguageFilter};
pub use mapping::{build_anchors_map, compute_anchor_mappings};
pub use parser::{MarkdownParser, ParseResult};
pub use profiling::{PerformanceMetrics, ResourceMonitor};
pub use registry::Registry;
pub use storage::Storage;
pub use types::*;
