//! Command implementations for the blz CLI
//!
//! This module contains all CLI command implementations, with each command
//! in its own submodule for better organization and maintainability.

mod add;
mod alias;
#[cfg(feature = "anchors")]
mod anchors;
mod completions;
mod config;
mod diff;
pub mod docs;
mod get;
mod history;
mod list;
mod lookup;
mod remove;
mod search;
mod update;

pub use add::execute as add_source;
pub use alias::{AliasCommand, execute as manage_alias};
// Anchor commands are behind a feature flag and not re-exported in v0.2
pub use completions::generate;
pub use completions::list_supported;
pub use config::{ConfigCommand, run as run_config};
pub use diff::show as show_diff;
pub use docs::{DocsFormat, execute as generate_docs};
pub use get::execute as get_lines;
pub use history::show as show_history;
pub use list::execute as list_sources;
pub use lookup::execute as lookup_registry;
pub use remove::execute as remove_source;
pub use search::{execute as search, handle_default as handle_default_search};
pub use update::{FlavorMode, execute as update_source, execute_all as update_all};

// Re-export types that commands might need
