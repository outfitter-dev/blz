//! MCP resources for BLZ
//!
//! Exposes BLZ data via custom `blz://` URI scheme with fallback support.

pub mod registry;
pub mod sources;

pub use registry::handle_registry_resource;
pub use sources::handle_source_resource;
