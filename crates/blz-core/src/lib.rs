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
/// Error types and result aliases
pub mod error;
/// HTTP fetching with conditional requests support
pub mod fetcher;
/// Search index implementation using Tantivy
pub mod index;
/// Anchor remapping utilities between versions
pub mod mapping;
/// Tree-sitter based markdown parser
pub mod parser;
/// Performance profiling utilities
pub mod profiling;
/// Built-in registry of known documentation sources
pub mod registry;
/// Local filesystem storage for cached documentation
pub mod storage;
/// Core data types and structures
pub mod types;

// Re-export commonly used types
pub use config::{
    Config, DefaultsConfig, FetchConfig, FollowLinks, IndexConfig, PathsConfig, ToolConfig,
    ToolMeta,
};
pub use error::{Error, Result};
pub use fetcher::{FetchResult, Fetcher};
pub use index::SearchIndex;
pub use mapping::{build_anchors_map, compute_anchor_mappings};
pub use parser::{MarkdownParser, ParseResult};
pub use profiling::{PerformanceMetrics, ResourceMonitor};
pub use registry::Registry;
pub use storage::Storage;
pub use types::*;
