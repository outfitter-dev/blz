//! MCP server implementation for BLZ

use std::{collections::HashMap, sync::Arc};

use blz_core::Storage;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, ErrorCode, ErrorData, GetPromptRequestParam,
    GetPromptResult, Implementation, ListPromptsResult, ListResourcesResult, ListToolsResult,
    PaginatedRequestParam, Prompt, PromptArgument, PromptsCapability, ProtocolVersion, RawContent,
    RawResource, RawTextContent, ReadResourceRequestParam, ReadResourceResult, Resource,
    ResourceContents, ResourcesCapability, ServerCapabilities, ServerInfo, Tool, ToolsCapability,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler};
use serde_json::json;
use tokio::sync::RwLock;

use crate::{error::McpResult, prompts, resources, tools, types::IndexCache};

/// MCP server for BLZ
#[derive(Clone)]
pub struct McpServer {
    /// Storage backend for accessing cached documentation
    storage: Arc<Storage>,
    /// Index cache with double-checked locking for search operations
    index_cache: IndexCache,
}

/// Build the JSON schema for the `find` tool.
fn build_find_tool_schema() -> serde_json::Map<String, serde_json::Value> {
    let schema = json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["search", "get", "toc"],
                "description": "Action to execute (optional; inferred from parameters)"
            },
            "query": {
                "type": "string",
                "description": "Search query"
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
            "context": {
                "type": "integer",
                "minimum": 0,
                "maximum": 50,
                "default": 0,
                "description": "Lines of context padding"
            },
            "linePadding": {
                "type": "integer",
                "minimum": 0,
                "maximum": 50,
                "description": "Alias for context"
            },
            "maxResults": {
                "type": "integer",
                "minimum": 1,
                "default": 10,
                "description": "Maximum search results"
            },
            "maxLines": {
                "type": "integer",
                "minimum": 1,
                "description": "Maximum lines to return for snippets"
            },
            "source": {
                "description": "Optional source filter: omit or set to \"all\" to search every source, provide a string alias for one source, or an array of aliases to target multiple sources",
                "oneOf": [
                    {
                        "type": "string"
                    },
                    {
                        "type": "array",
                        "items": {"type": "string"},
                        "minItems": 1
                    }
                ]
            },
            "format": {
                "type": "string",
                "enum": ["concise", "detailed"],
                "default": "concise",
                "description": "Response format (concise = minimal, detailed = full metadata)"
            },
            "headingsOnly": {
                "type": "boolean",
                "default": false,
                "description": "Restrict search results to headings only"
            },
            "headings": {
                "type": "string",
                "description": "TOC heading levels filter (e.g., \"1,2\" or \"<=2\")"
            },
            "tree": {
                "type": "boolean",
                "default": false,
                "description": "Return TOC as a tree"
            },
            "maxDepth": {
                "type": "integer",
                "minimum": 1,
                "description": "Maximum heading depth to include in TOC"
            },
            "includeTiming": {
                "type": "boolean",
                "default": false,
                "description": "Include timing metrics in the response"
            }
        }
    });
    // SAFETY: The json! macro above produces an object literal; as_object() cannot fail.
    #[allow(clippy::expect_used)]
    schema
        .as_object()
        .expect("find schema is an object")
        .clone()
}

/// Build the JSON schema for the `blz` tool.
fn build_blz_tool_schema() -> serde_json::Map<String, serde_json::Value> {
    let schema = json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["list", "add", "remove", "refresh", "info", "validate", "history", "help"],
                "description": "Action to execute (optional; inferred from parameters)"
            },
            "alias": {
                "type": "string",
                "description": "Source alias for add/remove/refresh/info/validate/history"
            },
            "url": {
                "type": "string",
                "description": "URL override for add"
            },
            "force": {
                "type": "boolean",
                "default": false,
                "description": "Force override if source exists (add only)"
            },
            "kind": {
                "type": "string",
                "enum": ["installed", "registry", "all"],
                "description": "List filter: installed, registry, or all"
            },
            "query": {
                "type": "string",
                "description": "Search query to filter sources"
            },
            "reindex": {
                "type": "boolean",
                "default": false,
                "description": "Re-index cached content instead of fetching (refresh)"
            },
            "all": {
                "type": "boolean",
                "default": false,
                "description": "Refresh all sources"
            }
        }
    });
    // SAFETY: The json! macro above produces an object literal; as_object() cannot fail.
    #[allow(clippy::expect_used)]
    schema.as_object().expect("blz schema is an object").clone()
}

/// Map a find tool error to the appropriate MCP error code.
const fn map_find_error_code(e: &crate::error::McpError) -> ErrorCode {
    match e.error_code() {
        -32700 => ErrorCode::PARSE_ERROR,
        -32600 => ErrorCode::INVALID_REQUEST,
        -32601 => ErrorCode::METHOD_NOT_FOUND,
        -32602 => ErrorCode::INVALID_PARAMS,
        -32603 => ErrorCode::INTERNAL_ERROR,
        other => ErrorCode(other),
    }
}

/// Map a blz tool error to the appropriate MCP error code.
const fn map_blz_error_code(e: &crate::error::McpError) -> ErrorCode {
    match e {
        crate::error::McpError::InvalidParams(_)
        | crate::error::McpError::SourceExists(_)
        | crate::error::McpError::SourceNotFound(_)
        | crate::error::McpError::MissingParameter(_)
        | crate::error::McpError::UnsupportedCommand(_) => ErrorCode::INVALID_PARAMS,
        _ => ErrorCode::INTERNAL_ERROR,
    }
}

/// Build a successful tool call result from a serializable output value.
fn build_tool_result<T: serde::Serialize>(output: &T) -> Result<CallToolResult, ErrorData> {
    let result_json = serde_json::to_value(output).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to serialize output: {e}"),
            None,
        )
    })?;

    let text = serde_json::to_string_pretty(&result_json).map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to format output: {e}"),
            None,
        )
    })?;

    Ok(CallToolResult {
        content: vec![Content {
            raw: RawContent::Text(RawTextContent { text, meta: None }),
            annotations: None,
        }],
        structured_content: Some(result_json),
        is_error: None,
        meta: None,
    })
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
    /// Describe server capabilities and implementation metadata.
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                resources: Some(ResourcesCapability {
                    subscribe: None,
                    list_changed: None,
                }),
                prompts: Some(PromptsCapability {
                    list_changed: None,
                    ..Default::default()
                }),
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

    /// List the tools supported by the BLZ MCP server.
    ///
    /// Provides minimal JSON schemas for tool parameters to keep the MCP
    /// handshake payload small.
    #[tracing::instrument(skip(self, _context))]
    #[allow(clippy::used_underscore_binding)]
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        tracing::debug!("listing tools");

        let tools = vec![
            Tool::new(
                "find",
                "Search, retrieve, and browse documentation (actions: search, get, toc)",
                Arc::new(build_find_tool_schema()),
            ),
            Tool::new(
                "blz",
                "Manage sources and metadata (actions: list, add, remove, refresh, info, validate, history, help)",
                Arc::new(build_blz_tool_schema()),
            ),
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    /// Execute a tool call and return the response payload.
    #[tracing::instrument(skip(self, _context))]
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
                        ErrorData::new(map_find_error_code(&e), e.to_string(), None)
                    })?;

                build_tool_result(&output)
            },
            "blz" => {
                let params: tools::BlzParams = serde_json::from_value(serde_json::Value::Object(
                    request.arguments.unwrap_or_default(),
                ))
                .map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!("Invalid blz parameters: {e}"),
                        None,
                    )
                })?;

                let output = tools::handle_blz(params, &self.storage, &self.index_cache)
                    .await
                    .map_err(|e| {
                        tracing::error!("blz tool error: {}", e);
                        ErrorData::new(map_blz_error_code(&e), e.to_string(), None)
                    })?;

                build_tool_result(&output)
            },
            _ => Err(ErrorData::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("Unknown tool: {}", request.name),
                None,
            )),
        }
    }

    /// List cached documentation sources as MCP resources.
    #[tracing::instrument(skip(self, _context))]
    #[allow(clippy::used_underscore_binding)]
    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        tracing::debug!("listing resources");

        // Get all installed sources
        let installed_sources = self.storage.list_sources();

        // Create resource entries for each source
        let mut resources: Vec<Resource> = installed_sources
            .into_iter()
            .map(|alias| {
                let uri = format!("blz://sources/{alias}");
                Resource {
                    raw: RawResource {
                        uri,
                        name: alias.clone(),
                        title: None,
                        description: Some(format!("Metadata for source '{alias}'")),
                        mime_type: Some("application/json".to_string()),
                        size: None,
                        icons: None,
                    },
                    annotations: None,
                }
            })
            .collect();

        // Add registry resource
        resources.push(Resource {
            raw: RawResource {
                uri: "blz://registry".to_string(),
                name: "registry".to_string(),
                title: None,
                description: Some("Complete BLZ registry of available sources".to_string()),
                mime_type: Some("application/json".to_string()),
                size: None,
                icons: None,
            },
            annotations: None,
        });

        tracing::debug!(count = resources.len(), "listed resources");

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    /// Read the contents of a single MCP resource.
    #[tracing::instrument(skip(self, _context))]
    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        tracing::debug!(uri = %request.uri, "reading resource");

        let result = if matches!(
            request.uri.as_str(),
            "blz://registry" | "resource://blz/registry"
        ) {
            // Handle registry resource
            resources::handle_registry_resource(&request.uri)
                .await
                .map_err(|e| {
                    tracing::error!("registry resource error: {}", e);
                    let error_code = match e {
                        crate::error::McpError::InvalidParams(_) => ErrorCode::INVALID_PARAMS,
                        _ => ErrorCode::INTERNAL_ERROR,
                    };
                    ErrorData::new(error_code, e.to_string(), None)
                })?
        } else if request.uri.starts_with("blz://sources/")
            || request.uri.starts_with("resource://blz/sources/")
        {
            // Handle source resource
            resources::handle_source_resource(&request.uri, &self.storage)
                .await
                .map_err(|e| {
                    tracing::error!("source resource error: {}", e);
                    let error_code = match e {
                        crate::error::McpError::SourceNotFound(_) => ErrorCode::INVALID_PARAMS,
                        _ => ErrorCode::INTERNAL_ERROR,
                    };
                    ErrorData::new(error_code, e.to_string(), None)
                })?
        } else {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Unknown resource URI: {}", request.uri),
                None,
            ));
        };

        // Convert JSON value to text content
        let text = serde_json::to_string_pretty(&result).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to serialize resource: {e}"),
                None,
            )
        })?;

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(text, request.uri)],
        })
    }

    /// List available prompt templates.
    #[tracing::instrument(skip(self, _context))]
    #[allow(clippy::used_underscore_binding)]
    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        tracing::debug!("listing prompts");

        let prompts = vec![Prompt::new(
            "discover-docs",
            Some("Find and add documentation sources for given technologies"),
            Some(vec![PromptArgument {
                name: "technologies".to_string(),
                title: None,
                description: Some(
                    "Comma-separated list of technologies to discover documentation for"
                        .to_string(),
                ),
                required: Some(true),
            }]),
        )];

        Ok(ListPromptsResult {
            prompts,
            next_cursor: None,
        })
    }

    /// Retrieve a prompt template and expand its arguments.
    #[tracing::instrument(skip(self, _context))]
    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, ErrorData> {
        tracing::debug!(prompt = %request.name, "getting prompt");

        match request.name.as_ref() {
            "discover-docs" => {
                let params: prompts::DiscoverDocsParams = serde_json::from_value(
                    serde_json::Value::Object(request.arguments.unwrap_or_default()),
                )
                .map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!("Invalid discover-docs parameters: {e}"),
                        None,
                    )
                })?;

                let output =
                    prompts::handle_discover_docs(&params, &self.storage).map_err(|e| {
                        tracing::error!("discover-docs prompt error: {}", e);
                        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                    })?;

                Ok(GetPromptResult {
                    description: None,
                    messages: output.messages,
                })
            },
            _ => Err(ErrorData::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("Unknown prompt: {}", request.name),
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
