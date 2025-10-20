//! MCP (Model Context Protocol) server command
//!
//! Launches the BLZ MCP server via stdio transport for AI agent integration.

use anyhow::Result;

/// Execute the MCP server command
///
/// Starts the BLZ MCP server and runs it until interrupted by SIGINT/SIGTERM.
///
/// # Errors
///
/// Returns an error if the server fails to initialize or encounters a runtime error.
pub async fn execute() -> Result<()> {
    // Tracing is already initialized by the CLI in lib.rs via set_global_default()
    // The MCP server's serve_stdio() will fail if we call it directly since it
    // tries to initialize tracing again. Instead, we manually create and serve
    // the server without re-initializing tracing.

    tracing::debug!("initializing BLZ MCP server");

    let server = blz_mcp::McpServer::new()?;
    server.serve_stdio().await?;

    Ok(())
}
