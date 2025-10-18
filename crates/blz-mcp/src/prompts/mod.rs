//! MCP prompts for BLZ
//!
//! Prompts provide guided workflows for agents to discover and work with documentation.

pub mod discover;

pub use discover::{DiscoverDocsParams, handle_discover_docs};
