//! Command implementations for the blz CLI
//!
//! This module contains all CLI command implementations, with each command
//! in its own submodule for better organization and maintainability.

mod add;
mod alias;
mod anchors;
mod completions;
mod diff;
pub mod docs;
mod get;
mod list;
mod lookup;
mod remove;
mod search;
mod update;

pub use add::execute as add_source;
pub use alias::{AliasCommand, execute as manage_alias};
pub use anchors::execute as show_anchors;
pub use anchors::get_by_anchor;
pub use completions::generate;
pub use diff::show as show_diff;
pub use docs::{DocsFormat, execute as generate_docs};
pub use get::execute as get_lines;
pub use list::execute as list_sources;
pub use lookup::execute as lookup_registry;
pub use remove::execute as remove_source;
pub use search::{execute as search, handle_default as handle_default_search};
pub use update::{execute as update_source, execute_all as update_all};

// Re-export types that commands might need
