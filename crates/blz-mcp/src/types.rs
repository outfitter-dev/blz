//! Common types for BLZ MCP server

use std::sync::Arc;

use blz_core::SearchIndex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Shared, async-safe cache for loaded search indices.
///
/// Used with a double-checked locking pattern to avoid redundant index loads
/// while allowing concurrent readers.
pub type IndexCache = Arc<RwLock<std::collections::HashMap<String, Arc<SearchIndex>>>>;

/// Response format for tool outputs
///
/// Controls the verbosity of tool responses to optimize token usage.
/// Based on Anthropic research showing 30-65% token savings with concise mode.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ResponseFormat {
    /// Minimal response with only essential data (default)
    ///
    /// Returns core information without metadata, snippets, or auxiliary fields.
    /// Use for initial searches and rapid scanning.
    #[default]
    Concise,

    /// Full response with all available metadata
    ///
    /// Returns complete information including snippets, headings, URLs, timestamps.
    /// Use when context is needed for decision-making.
    Detailed,
}
