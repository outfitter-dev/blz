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
    AddArgs, AddRequest, DescriptorInput, dispatch as dispatch_add, execute as add_source,
};
pub use alias::{AliasCommands, dispatch as dispatch_alias};
pub use check::{CheckArgs, execute as check_source};
pub use clear::run as clear_cache;
pub use completions::dispatch as dispatch_completions;
#[cfg(test)]
pub use completions::generate;
pub use toc::{AnchorCommands, TocArgs, dispatch as dispatch_toc, dispatch_anchor};
// config command removed in v1.0.0-beta.1 - flavor preferences eliminated
pub use claude_plugin::{ClaudePluginCommands, dispatch as dispatch_claude_plugin};
pub use create_source::{RegistryCommands, dispatch as dispatch_registry};
pub use diff::show as show_diff;
pub use docs::{DocsCommands, dispatch as dispatch_docs};
pub use docs_bundle::{
    BUNDLED_ALIAS, SyncStatus as DocsSyncStatus, print_full_content, print_overview,
    sync as sync_bundled_docs,
};
pub use doctor::execute as run_doctor;
pub use find::{FindArgs, dispatch as dispatch_find};
pub use get::{RequestSpec, dispatch as dispatch_get, execute as get_lines};
pub use history::dispatch as dispatch_history;
pub use info::execute_info;
pub use list::dispatch as dispatch_list;
pub use lookup::dispatch as dispatch_lookup;
pub use map::{MapArgs, dispatch as dispatch_map};
pub use mcp::execute as mcp_server;
pub use query::{QueryArgs, dispatch as dispatch_query};
#[allow(deprecated)]
pub use refresh::{dispatch_deprecated as dispatch_refresh_deprecated, dispatch_update_deprecated};
#[allow(deprecated)]
pub use remove::dispatch_deprecated as dispatch_remove_deprecated;
pub use rm::{RmArgs, execute as rm_source};
pub use search::{DEFAULT_MAX_CHARS, SearchArgs, dispatch as dispatch_search, execute as search};
pub use stats::execute as show_stats;
pub use sync::{SyncArgs, dispatch as dispatch_sync};
#[allow(deprecated)]
pub use validate::dispatch_deprecated as dispatch_validate_deprecated;

// Re-export types that commands might need
