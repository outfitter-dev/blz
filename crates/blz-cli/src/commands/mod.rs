//! Command implementations for the blz CLI
//!
//! This module contains all CLI command implementations, with each command
//! in its own submodule for better organization and maintainability.

mod add;
mod alias;
mod check;
mod clear;
mod completions;
mod toc;
// config module removed in v1.0.0-beta.1 - flavor preferences eliminated
mod claude_plugin;
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
mod map;
mod mcp;
mod query;
mod refresh;
mod remove;
mod rm;
mod search;
mod stats;
mod sync;
#[allow(deprecated)]
mod update;
mod validate;

pub use add::{
    AddArgs, AddFlowOptions, AddRequest, DescriptorInput, execute as add_source,
    execute_manifest as add_manifest,
};
pub use alias::{AliasCommand, AliasCommands, execute as manage_alias};
pub use check::{CheckArgs, execute as check_source};
pub use clear::run as clear_cache;
pub use completions::generate;
pub use completions::list_supported;
pub use toc::{AnchorCommands, TocArgs, execute as show_toc, get_by_anchor};
// config command removed in v1.0.0-beta.1 - flavor preferences eliminated
pub use claude_plugin::{ClaudePluginCommands, install_local_plugin};
pub use create_source::{RegistryCommands, execute as create_registry_source};
pub use diff::show as show_diff;
pub use docs::{DocsCommands, DocsFormat, DocsSearchArgs, execute as generate_docs};
pub use docs_bundle::{
    BUNDLED_ALIAS, SyncStatus as DocsSyncStatus, print_full_content, print_overview,
    sync as sync_bundled_docs,
};
pub use doctor::execute as run_doctor;
pub use find::{FindArgs, execute as find};
pub use get::{RequestSpec, execute as get_lines};
pub use history::show as show_history;
pub use info::execute_info;
pub use list::execute as list_sources;
pub use lookup::execute as lookup_registry;
pub use map::{MapArgs, execute as show_map};
pub use mcp::execute as mcp_server;
pub use query::{QueryArgs, execute as query};
pub use refresh::{execute as refresh_source, execute_all as refresh_all};
pub use remove::execute as remove_source;
pub use rm::{RmArgs, execute as rm_source};
pub use search::{DEFAULT_MAX_CHARS, SearchArgs, clamp_max_chars, execute as search};
pub use stats::execute as show_stats;
pub use sync::{SyncArgs, execute as sync_source};
pub use validate::execute as validate_source;

// Re-export types that commands might need
