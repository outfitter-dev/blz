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
//! - [`Verbosity`] - Output verbosity level (quiet/normal/verbose/debug)
//!
//! # Future Additions
//!
//! Planned for future phases:
//! - `PaginationArgs` - Shared limit/offset arguments
//! - `ContextArgs` - Context line settings for retrieval
//! - `OutputArgs` - Format and color settings
//! - `SearchArgs` - Search-specific options

mod verbosity;

pub use verbosity::Verbosity;
