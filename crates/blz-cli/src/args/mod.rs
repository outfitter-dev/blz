//! Shared argument types and groups for the BLZ CLI.
//!
//! This module provides reusable argument definitions that can be composed
//! across multiple commands using clap's `#[command(flatten)]` attribute.
//!
//! # Design Philosophy
//!
//! Rather than duplicating argument definitions across commands, shared
//! argument groups ensure:
//! - Consistent flag names and help text
//! - Reduced code duplication
//! - Easier maintenance and updates
//!
//! # Available Types
//!
//! ## Core Types
//!
//! - [`Verbosity`] - Output verbosity level (quiet/normal/verbose/debug)
//!
//! ## Argument Groups
//!
//! - [`PaginationArgs`] - Limit and offset for result pagination
//! - [`ContextArgs`] - Context lines for content retrieval (grep-style)
//! - [`OutputArgs`] - Format selection with TTY auto-detection
//!
//! # Examples
//!
//! ```ignore
//! use blz_cli::args::{ContextArgs, OutputArgs, PaginationArgs, Verbosity};
//! use clap::{Args, Parser};
//!
//! #[derive(Parser)]
//! struct SearchCommand {
//!     /// Search query
//!     query: String,
//!
//!     #[command(flatten)]
//!     pagination: PaginationArgs,
//!
//!     #[command(flatten)]
//!     context: ContextArgs,
//!
//!     #[command(flatten)]
//!     output: OutputArgs,
//! }
//! ```

mod context;
mod output;
mod pagination;
mod verbosity;

pub use context::{ContextArgs, ContextMode, merge_context_flags};
pub use output::{OutputArgs, OutputFormat};
pub use pagination::PaginationArgs;
pub use verbosity::Verbosity;
