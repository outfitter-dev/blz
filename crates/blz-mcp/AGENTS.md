# blz-mcp Agent Guide

This crate implements the Model Context Protocol (MCP) server for blz, enabling integration with Claude Code and other MCP-compatible tools.

## Architecture Overview

- **Protocol Compliance**: Implements MCP specification for tool integration
- **JSON-RPC**: All communication via JSON-RPC 2.0 over stdio/network
- **Async Server**: Built on tokio for concurrent request handling
- **Resource Management**: Provides search capabilities as MCP resources

## Key Components

- **`main.rs`**: MCP server implementation and protocol handling
- **Protocol handlers**: Request/response processing for MCP methods
- **Resource providers**: Exposes blz search functionality as MCP resources
- **Client adapters**: Bridge between MCP protocol and blz-core

## MCP Protocol Integration

### Server Implementation

```rust
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

// ✅ GOOD: Proper MCP server structure
pub struct McpServer {
    core: Arc<blz_core::SearchEngine>,
    client_info: Option<ClientInfo>,
}

impl McpServer {
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "resources/list" => self.handle_resources_list(request).await,
            "resources/read" => self.handle_resources_read(request).await,
            "tools/list" => self.handle_tools_list(request).await,
            "tools/call" => self.handle_tools_call(request).await,
            _ => JsonRpcResponse::error(
                request.id,
                -32601, // Method not found
                "Method not supported",
            ),
        }
    }
}
```

### Resource Management

```rust
// ✅ GOOD: Expose search capabilities as MCP resources
#[derive(Debug, Serialize)]
pub struct SearchResource {
    uri: String,
    name: String,
    description: String,
    mime_type: String,
}

impl McpServer {
    async fn handle_resources_list(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let sources = match self.core.list_sources().await {
            Ok(sources) => sources,
            Err(e) => return JsonRpcResponse::error(request.id, -1, &e.to_string()),
        };
        
        let resources: Vec<SearchResource> = sources
            .into_iter()
            .map(|source| SearchResource {
                uri: format!("blz://{}", source.alias),
                name: format!("blz search: {}", source.alias),
                description: format!("Search {} documentation", source.name),
                mime_type: "application/json".to_string(),
            })
            .collect();
        
        JsonRpcResponse::success(request.id, json!({ "resources": resources }))
    }
    
    async fn handle_resources_read(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params = match request.params {
            Some(params) => params,
            None => return JsonRpcResponse::error(request.id, -32602, "Missing parameters"),
        };
        
        let uri = match params.get("uri").and_then(|v| v.as_str()) {
            Some(uri) => uri,
            None => return JsonRpcResponse::error(request.id, -32602, "Missing uri parameter"),
        };
        
        // Parse blz:// URI
        if !uri.starts_with("blz://") {
            return JsonRpcResponse::error(request.id, -32602, "Invalid URI scheme");
        }
        
        let source_alias = &uri[6..]; // Remove "blz://" prefix
        
        // Return source information
        match self.core.get_source_info(source_alias).await {
            Ok(info) => {
                let content = json!({
                    "alias": info.alias,
                    "url": info.url,
                    "last_updated": info.last_updated,
                    "document_count": info.document_count,
                });
                
                JsonRpcResponse::success(request.id, json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&content).unwrap()
                    }]
                }))
            }
            Err(e) => JsonRpcResponse::error(request.id, -1, &e.to_string()),
        }
    }
}
```

### Tool Implementation

```rust
// ✅ GOOD: Implement search as MCP tool
#[derive(Debug, Serialize)]
pub struct SearchTool {
    name: String,
    description: String,
    input_schema: Value,
}

impl Default for SearchTool {
    fn default() -> Self {
        Self {
            name: "blz_search".to_string(),
            description: "Search local documentation cache".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "source": {
                        "type": "string",
                        "description": "Specific source to search (optional)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 10)",
                        "minimum": 1,
                        "maximum": 100
                    }
                },
                "required": ["query"]
            }),
        }
    }
}

impl McpServer {
    async fn handle_tools_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params = match request.params {
            Some(params) => params,
            None => return JsonRpcResponse::error(request.id, -32602, "Missing parameters"),
        };
        
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some("blz_search") => "blz_search",
            Some(name) => return JsonRpcResponse::error(
                request.id,
                -32602,
                &format!("Unknown tool: {}", name)
            ),
            None => return JsonRpcResponse::error(request.id, -32602, "Missing tool name"),
        };
        
        let arguments = match params.get("arguments") {
            Some(args) => args,
            None => return JsonRpcResponse::error(request.id, -32602, "Missing arguments"),
        };
        
        self.execute_search_tool(request.id, arguments).await
    }
    
    async fn execute_search_tool(&self, id: Value, args: &Value) -> JsonRpcResponse {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) if !q.trim().is_empty() => q,
            _ => return JsonRpcResponse::error(id, -32602, "Missing or empty query"),
        };
        
        let source = args.get("source").and_then(|v| v.as_str());
        let limit = args.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(100) as usize;
        
        match self.core.search(query, source, limit).await {
            Ok(results) => {
                let formatted_results = self.format_search_results(results);
                JsonRpcResponse::success(id, json!({
                    "content": [{
                        "type": "text",
                        "text": formatted_results
                    }]
                }))
            }
            Err(e) => JsonRpcResponse::error(id, -1, &e.to_string()),
        }
    }
    
    fn format_search_results(&self, results: blz_core::SearchResults) -> String {
        let mut output = String::new();
        output.push_str(&format!("Found {} results:\n\n", results.hits.len()));
        
        for (i, hit) in results.hits.iter().enumerate() {
            output.push_str(&format!(
                "{}. {} ({}:{})\n   {}\n\n",
                i + 1,
                hit.title,
                hit.source,
                hit.line_number,
                hit.snippet.trim()
            ));
        }
        
        if results.hits.is_empty() {
            output.push_str("No results found. Try a different query or check available sources.");
        }
        
        output
    }
}
```

## JSON-RPC Protocol

### Message Format

```rust
// ✅ GOOD: Proper JSON-RPC 2.0 structure
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String, // Always "2.0"
    pub id: Value,       // Request ID
    pub method: String,  // Method name
    pub params: Option<Value>, // Parameters
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String, // Always "2.0"
    pub id: Value,       // Request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }
    
    pub fn error(id: Value, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}
```

### Connection Handling

```rust
// ✅ GOOD: Robust connection handling
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

pub async fn run_server() -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    
    let mut reader = BufReader::new(stdin);
    let mut writer = BufWriter::new(stdout);
    
    let server = Arc::new(McpServer::new().await?);
    
    let mut line = String::new();
    
    loop {
        line.clear();
        
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                
                let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                    Ok(request) => server.handle_request(request).await,
                    Err(e) => JsonRpcResponse::error(
                        Value::Null,
                        -32700, // Parse error
                        &format!("Parse error: {}", e),
                    ),
                };
                
                let response_json = serde_json::to_string(&response)?;
                writer.write_all(response_json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
            }
            Err(e) => {
                eprintln!("Error reading from stdin: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}
```

## Error Handling

### MCP-Specific Errors

```rust
// ✅ GOOD: Map blz errors to MCP error codes
impl From<blz_core::Error> for JsonRpcError {
    fn from(error: blz_core::Error) -> Self {
        match error {
            blz_core::Error::SourceNotFound { .. } => JsonRpcError {
                code: -32001,
                message: "Source not found".to_string(),
                data: Some(json!({ "error": error.to_string() })),
            },
            blz_core::Error::InvalidQuery { .. } => JsonRpcError {
                code: -32002,
                message: "Invalid query".to_string(),
                data: Some(json!({ "error": error.to_string() })),
            },
            blz_core::Error::IndexError { .. } => JsonRpcError {
                code: -32003,
                message: "Index error".to_string(),
                data: Some(json!({ "error": error.to_string() })),
            },
            _ => JsonRpcError {
                code: -1,
                message: "Internal error".to_string(),
                data: Some(json!({ "error": error.to_string() })),
            },
        }
    }
}
```

## Testing Patterns

### Protocol Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_initialize_request() {
        let server = McpServer::new_test().await;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "1.0",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0"
                }
            })),
        };
        
        let response = server.handle_request(request).await;
        
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, json!(1));
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }
    
    #[tokio::test]
    async fn test_search_tool() {
        let server = McpServer::new_test().await;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(2),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "blz_search",
                "arguments": {
                    "query": "rust programming",
                    "limit": 5
                }
            })),
        };
        
        let response = server.handle_request(request).await;
        
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        assert!(result.get("content").is_some());
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_full_mcp_workflow() {
    // Test complete MCP workflow
    let server = McpServer::new_test().await;
    
    // 1. Initialize
    let init_request = create_initialize_request();
    let init_response = server.handle_request(init_request).await;
    assert!(init_response.error.is_none());
    
    // 2. List resources
    let list_request = create_resources_list_request();
    let list_response = server.handle_request(list_request).await;
    assert!(list_response.error.is_none());
    
    // 3. Read resource
    let read_request = create_resources_read_request("blz://test");
    let read_response = server.handle_request(read_request).await;
    assert!(read_response.error.is_none());
    
    // 4. Call search tool
    let search_request = create_tool_call_request("test query");
    let search_response = server.handle_request(search_request).await;
    assert!(search_response.error.is_none());
}
```

## Common Agent Tasks

### Adding New Tools

1. **Define tool schema** in tool definitions
2. **Implement tool handler** in `handle_tools_call`
3. **Add validation** for tool parameters
4. **Write tests** for the new tool
5. **Update tool list** in `handle_tools_list`

### Extending Resources

1. **Define new resource types**
2. **Add URI schemes** (e.g., `blz://type/resource`)
3. **Implement resource handlers**
4. **Add to resource list**
5. **Test resource access**

### Protocol Updates

1. **Check MCP specification** for new features
2. **Update message types** if needed
3. **Implement new methods**
4. **Maintain backward compatibility**
5. **Update tests**

## Common Gotchas

### JSON-RPC Compliance

```rust
// ✅ GOOD: Always include jsonrpc field
let response = json!({
    "jsonrpc": "2.0",
    "id": request_id,
    "result": result_value
});

// ❌ BAD: Missing required fields
let bad_response = json!({
    "id": request_id,
    "result": result_value  // Missing jsonrpc!
});
```

### Error Handling

```rust
// ✅ GOOD: Proper error codes and messages
return JsonRpcResponse::error(
    request.id,
    -32602, // Invalid params
    "Missing required parameter 'query'"
);

// ❌ BAD: Generic error codes
return JsonRpcResponse::error(
    request.id,
    -1, // Too generic
    "Error" // Not helpful
);
```

### Async Resource Management

```rust
// ✅ GOOD: Proper resource cleanup
pub struct McpServer {
    core: Arc<blz_core::SearchEngine>,
    _cleanup_task: tokio::task::JoinHandle<()>,
}

impl Drop for McpServer {
    fn drop(&mut self) {
        self._cleanup_task.abort();
    }
}
```

## Development Workflow

### Testing the Server
```bash
# Run MCP server tests
cargo test -p blz-mcp

# Manual testing with stdio
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | cargo run -p blz-mcp

# Test with MCP client tools
mcp-client test blz-mcp
```

### Protocol Debugging
```bash
# Enable debug logging
RUST_LOG=debug cargo run -p blz-mcp

# Capture protocol messages
cargo run -p blz-mcp 2>&1 | tee mcp-debug.log
```

Remember: MCP compliance is critical - always test against the official specification and reference implementations.