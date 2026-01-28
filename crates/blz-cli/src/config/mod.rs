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
//! // After: 2 parameters
//! pub async fn execute(args: &QueryArgs, config: &ExecutionConfig) -> Result<()>
//! ```
//!
//! # Available Types
//!
//! - [`ExecutionConfig`] - Common execution context for all commands
//!
//! # Examples
//!
//! ```ignore
//! use blz_cli::config::ExecutionConfig;
//! use blz_cli::args::{OutputFormat, Verbosity};
//! use blz_core::PerformanceMetrics;
//!
//! let config = ExecutionConfig::new(
//!     Verbosity::Normal,
//!     OutputFormat::Json,
//!     PerformanceMetrics::new(),
//! );
//!
//! // Use with command execution
//! commands::execute(&args, &config).await?;
//! ```

mod resolved;

pub use resolved::ExecutionConfig;
