//! # Utility Functions and Helpers
//!
//! This module contains shared utilities used across the CLI commands, organized
//! into functional categories for better maintainability and code reuse.
//!
//! ## Organization
//!
//! - [`constants`]: Reserved keywords, command names, and other static values
//! - [`formatting`]: Color schemes, text formatting, and display utilities
//! - [`parsing`]: Input parsing functions for line ranges, queries, etc.
//! - [`validation`]: Input validation functions for aliases, URLs, etc.
//!
//! ## Design Principles
//!
//! - **Pure Functions**: Most utilities are pure functions with no side effects
//! - **Error Handling**: Comprehensive validation with descriptive error messages
//! - **Performance**: Optimized for CLI usage patterns (small inputs, fast execution)
//! - **Consistency**: Uniform behavior across all CLI commands
//!
//! ## Common Usage Patterns
//!
//! ```rust,no_run
//! use crate::utils::{validate_alias, parse_line_ranges, get_alias_color};
//!
//! // Validate user input
//! validate_alias("react")?;
//!
//! // Parse line ranges
//! let ranges = parse_line_ranges("120-142,200+10")?;
//!
//! // Get consistent colors
//! let color = get_alias_color("react");
//! ```
//!
//! ## Reserved Keywords
//!
//! The [`RESERVED_KEYWORDS`] constant prevents conflicts between user-defined
//! aliases and CLI commands, ensuring the interface remains predictable as
//! new commands are added.
//!
//! ## Input Validation
//!
//! All user inputs are validated early in the command processing pipeline
//! to provide clear error messages and prevent invalid operations:
//!
//! - **Aliases**: Must not conflict with commands or reserved words
//! - **Line ranges**: Must be valid, non-empty, and properly formatted
//! - **URLs**: Must be well-formed and use supported protocols
//!
//! ## Color Consistency
//!
//! The formatting utilities ensure consistent color usage across the CLI:
//!
//! - Each alias gets a consistent color based on a hash of its name
//! - Colors are chosen for good terminal contrast and accessibility
//! - Color output respects `NO_COLOR` and terminal capabilities

pub mod constants;
pub mod formatting;
pub mod history;
pub mod parsing;
pub mod validation;

// Re-export commonly used utilities
