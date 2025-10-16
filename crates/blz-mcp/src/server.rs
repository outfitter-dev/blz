//! MCP server implementation for BLZ

use std::{collections::HashMap, sync::Arc};

use blz_core::Storage;
use rmcp::ServerHandler;
use rmcp::model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo};
use tokio::sync::RwLock;

use crate::{error::McpResult, types::IndexCache};

/// MCP server for BLZ
#[derive(Clone)]
pub struct McpServer {
    /// Storage backend (unused in this phase, will be used for tool implementations)
    #[allow(dead_code)]
    storage: Arc<Storage>,
    /// Index cache with double-checked locking (unused in this phase, will be used for tool implementations)
    #[allow(dead_code)]
    index_cache: IndexCache,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new() -> McpResult<Self> {
        let storage = Storage::new()?;
        Ok(Self {
            storage: Arc::new(storage),
            index_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Serve the MCP protocol over stdio
    pub async fn serve_stdio(&self) -> McpResult<()> {
        tracing::info!("BLZ MCP server starting");

        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let service = rmcp::serve_server(self.clone(), (stdin, stdout))
            .await
            .map_err(|e| {
                tracing::error!("server initialization error: {}", e);
                crate::error::McpError::Protocol(e.to_string())
            })?;

        // Keep the service running until it's cancelled
        service.waiting().await.map_err(|e| {
            tracing::error!("server runtime error: {}", e);
            crate::error::McpError::Protocol(e.to_string())
        })?;

        tracing::info!("BLZ MCP server stopped");
        Ok(())
    }
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "blz-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: None,
        }
    }
}

// Note: Default impl removed - McpServer::new() can fail, so Default is inappropriate here.
// Use McpServer::new() directly instead.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_info_response() {
        let server = McpServer::new().expect("Failed to create server");
        let info = server.get_info();

        assert_eq!(info.server_info.name, "blz-mcp");
        assert!(!info.server_info.version.is_empty());
        assert_eq!(info.protocol_version, ProtocolVersion::default());
    }

    #[test]
    fn test_server_info_serialization_size() {
        let server = McpServer::new().expect("Failed to create server");
        let info = server.get_info();
        let json = serde_json::to_string(&info).expect("Failed to serialize");

        // DoD requirement: handshake < 1KB
        assert!(
            json.len() < 1024,
            "Handshake response {} bytes exceeds 1KB limit",
            json.len()
        );
    }
}
