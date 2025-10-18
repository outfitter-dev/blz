//! MCP server implementation for BLZ

use std::{collections::HashMap, sync::Arc};

use blz_core::Storage;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, ErrorCode, ErrorData, Implementation,
    ListToolsResult, PaginatedRequestParam, ProtocolVersion, RawContent, RawTextContent,
    ServerCapabilities, ServerInfo, Tool, ToolsCapability,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler};
use serde_json::json;
use tokio::sync::RwLock;

use crate::{error::McpResult, tools, types::IndexCache};

/// MCP server for BLZ
#[derive(Clone)]
pub struct McpServer {
    /// Storage backend for accessing cached documentation
    storage: Arc<Storage>,
    /// Index cache with double-checked locking for search operations
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
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                ..Default::default()
            },
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

    #[tracing::instrument(skip(self, _context))]
    #[allow(clippy::too_many_lines)]
    async fn list_tools(
        &self,
        #[allow(clippy::used_underscore_binding)] _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        tracing::debug!("listing tools");

        // Minimal schema to keep handshake <1 KB
        let find_schema = json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query (optional if only retrieving snippets)"
                },
                "snippets": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Citation strings (e.g., 'bun:10-20,30-40')"
                },
                "contextMode": {
                    "type": "string",
                    "enum": ["none", "symmetric", "all"],
                    "default": "none",
                    "description": "Context expansion mode"
                },
                "linePadding": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 50,
                    "default": 0,
                    "description": "Lines of padding for symmetric mode"
                },
                "maxResults": {
                    "type": "integer",
                    "minimum": 1,
                    "default": 10,
                    "description": "Maximum search results"
                },
                "source": {
                    "type": "string",
                    "description": "Source to search (required when using query parameter)"
                }
            }
        });

        let list_sources_schema = json!({
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "enum": ["installed", "registry", "all"],
                    "description": "Filter by source kind"
                },
                "query": {
                    "type": "string",
                    "description": "Search query to filter sources"
                }
            }
        });

        let source_add_schema = json!({
            "type": "object",
            "properties": {
                "alias": {
                    "type": "string",
                    "description": "Alias for the source"
                },
                "url": {
                    "type": "string",
                    "description": "URL override (if not from registry)"
                },
                "force": {
                    "type": "boolean",
                    "default": false,
                    "description": "Force override if source exists"
                }
            },
            "required": ["alias"]
        });

        let find_schema_obj = find_schema
            .as_object()
            .ok_or_else(|| ErrorData::new(ErrorCode::INTERNAL_ERROR, "Invalid schema", None))?
            .clone();

        let list_sources_schema_obj = list_sources_schema
            .as_object()
            .ok_or_else(|| ErrorData::new(ErrorCode::INTERNAL_ERROR, "Invalid schema", None))?
            .clone();

        let source_add_schema_obj = source_add_schema
            .as_object()
            .ok_or_else(|| ErrorData::new(ErrorCode::INTERNAL_ERROR, "Invalid schema", None))?
            .clone();

        let tools = vec![
            Tool::new(
                "find",
                "Search & retrieve documentation snippets",
                Arc::new(find_schema_obj),
            ),
            Tool::new(
                "list-sources",
                "List docs",
                Arc::new(list_sources_schema_obj),
            ),
            Tool::new("source-add", "Add docs", Arc::new(source_add_schema_obj)),
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    #[tracing::instrument(skip(self, _context))]
    #[allow(clippy::too_many_lines)]
    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        tracing::debug!(tool = %request.name, "calling tool");

        match request.name.as_ref() {
            "find" => {
                let params: tools::FindParams = serde_json::from_value(serde_json::Value::Object(
                    request.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!("Invalid find parameters: {e}"),
                        None,
                    )
                })?;

                let output = tools::handle_find(params, &self.storage, &self.index_cache)
                    .await
                    .map_err(|e| {
                        tracing::error!("find tool error: {}", e);
                        let error_code = match e.error_code() {
                            -32700 => ErrorCode::PARSE_ERROR,
                            -32600 => ErrorCode::INVALID_REQUEST,
                            -32601 => ErrorCode::METHOD_NOT_FOUND,
                            -32602 => ErrorCode::INVALID_PARAMS,
                            -32603 => ErrorCode::INTERNAL_ERROR,
                            other => ErrorCode(other),
                        };
                        ErrorData::new(error_code, e.to_string(), None)
                    })?;

                let result_json = serde_json::to_value(&output).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to serialize output: {e}"),
                        None,
                    )
                })?;

                Ok(CallToolResult {
                    content: vec![Content {
                        raw: RawContent::Text(RawTextContent {
                            text: serde_json::to_string_pretty(&result_json).map_err(|e| {
                                ErrorData::new(
                                    ErrorCode::INTERNAL_ERROR,
                                    format!("Failed to format output: {e}"),
                                    None,
                                )
                            })?,
                            meta: None,
                        }),
                        annotations: None,
                    }],
                    structured_content: Some(result_json),
                    is_error: None,
                    meta: None,
                })
            },
            "list-sources" => {
                let params: tools::ListSourcesParams = serde_json::from_value(
                    serde_json::Value::Object(request.arguments.unwrap_or_default()),
                )
                .map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!("Invalid list-sources parameters: {e}"),
                        None,
                    )
                })?;

                let output = tools::handle_list_sources(params, &self.storage)
                    .await
                    .map_err(|e| {
                        tracing::error!("list-sources tool error: {}", e);
                        let error_code = match e {
                            crate::error::McpError::InvalidParams(_) => ErrorCode::INVALID_PARAMS,
                            _ => ErrorCode::INTERNAL_ERROR,
                        };
                        ErrorData::new(error_code, e.to_string(), None)
                    })?;

                let result_json = serde_json::to_value(&output).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to serialize output: {e}"),
                        None,
                    )
                })?;

                Ok(CallToolResult {
                    content: vec![Content {
                        raw: RawContent::Text(RawTextContent {
                            text: serde_json::to_string_pretty(&result_json).map_err(|e| {
                                ErrorData::new(
                                    ErrorCode::INTERNAL_ERROR,
                                    format!("Failed to format output: {e}"),
                                    None,
                                )
                            })?,
                            meta: None,
                        }),
                        annotations: None,
                    }],
                    structured_content: Some(result_json),
                    is_error: None,
                    meta: None,
                })
            },
            "source-add" => {
                let params: tools::SourceAddParams = serde_json::from_value(
                    serde_json::Value::Object(request.arguments.unwrap_or_default()),
                )
                .map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!("Invalid source-add parameters: {e}"),
                        None,
                    )
                })?;

                let output = tools::handle_source_add(params, &self.storage, &self.index_cache)
                    .await
                    .map_err(|e| {
                        tracing::error!("source-add tool error: {}", e);
                        let error_code = match e {
                            crate::error::McpError::SourceExists(_)
                            | crate::error::McpError::SourceNotFound(_)
                            | crate::error::McpError::InvalidParams(_) => ErrorCode::INVALID_PARAMS,
                            _ => ErrorCode::INTERNAL_ERROR,
                        };
                        ErrorData::new(error_code, e.to_string(), None)
                    })?;

                let result_json = serde_json::to_value(&output).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to serialize output: {e}"),
                        None,
                    )
                })?;

                Ok(CallToolResult {
                    content: vec![Content {
                        raw: RawContent::Text(RawTextContent {
                            text: serde_json::to_string_pretty(&result_json).map_err(|e| {
                                ErrorData::new(
                                    ErrorCode::INTERNAL_ERROR,
                                    format!("Failed to format output: {e}"),
                                    None,
                                )
                            })?,
                            meta: None,
                        }),
                        annotations: None,
                    }],
                    structured_content: Some(result_json),
                    is_error: None,
                    meta: None,
                })
            },
            _ => Err(ErrorData::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("Unknown tool: {}", request.name),
                None,
            )),
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
