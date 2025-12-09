//! Command implementations for the blz CLI
//!
//! This module contains all CLI command implementations, with each command
//! in its own submodule for better organization and maintainability.

mod add;
mod alias;
mod clear;
mod completions;
mod toc;
// config module removed in v1.0.0-beta.1 - flavor preferences eliminated
mod create_source;
mod diff;
pub mod docs;
pub mod docs_bundle;
mod doctor;
mod find;
mod get;
mod history;
mod info;
mod list;
mod lookup;
mod mcp;
mod refresh;
mod remove;
mod search;
mod stats;
#[allow(deprecated)]
mod update;
mod validate;

pub use add::{
    AddFlowOptions, AddRequest, DescriptorInput, execute as add_source,
    execute_manifest as add_manifest,
};
pub use alias::{AliasCommand, execute as manage_alias};
pub use clear::run as clear_cache;
pub use completions::generate;
pub use completions::list_supported;
pub use toc::{execute as show_toc, get_by_anchor};
// config command removed in v1.0.0-beta.1 - flavor preferences eliminated
pub use create_source::execute as create_registry_source;
pub use diff::show as show_diff;
pub use docs::{DocsFormat, execute as generate_docs};
pub use docs_bundle::{
    BUNDLED_ALIAS, SyncStatus as DocsSyncStatus, print_full_content, print_overview,
    sync as sync_bundled_docs,
};
pub use doctor::execute as run_doctor;
pub use find::execute as find;
pub use get::{RequestSpec, execute as get_lines};
pub use history::show as show_history;
pub use info::execute_info;
pub use list::execute as list_sources;
pub use lookup::execute as lookup_registry;
pub use mcp::execute as mcp_server;
pub use refresh::{execute as refresh_source, execute_all as refresh_all};
pub use remove::execute as remove_source;
pub use search::{DEFAULT_MAX_CHARS, clamp_max_chars};
pub use search::{execute as search, handle_default as handle_default_search};
pub use stats::execute as show_stats;
pub use validate::execute as validate_source;

// Re-export types that commands might need
