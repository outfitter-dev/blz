//! Common types for BLZ MCP server

use std::sync::Arc;

use blz_core::SearchIndex;
use tokio::sync::RwLock;

/// Type alias for the index cache
pub type IndexCache = Arc<RwLock<std::collections::HashMap<String, Arc<SearchIndex>>>>;
