//! BLZ MCP Server
//!
//! A Rust-native MCP (Model Context Protocol) server for BLZ that provides
//! local-first documentation search capabilities.

pub mod cache;
pub mod error;
pub mod prompts;
pub mod resources;
pub mod server;
pub mod tools;
pub mod types;

pub use error::{McpError, McpResult};
pub use server::McpServer;

/// Main entry point for the MCP server
///
/// This function creates and runs the MCP server over stdio.
///
/// # Errors
///
/// Returns an error if the server fails to initialize or run.
pub async fn serve_stdio() -> McpResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();

    tracing::debug!("initializing BLZ MCP server");

    let server = McpServer::new()?;
    server.serve_stdio().await
}
