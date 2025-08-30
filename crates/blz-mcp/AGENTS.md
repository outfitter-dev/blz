# blz-mcp Development Guide for Agents

## Context
This is the MCP (Model Context Protocol) server for Claude integration.
**Protocol compliance is critical** - follow MCP specification exactly for reliable Claude integration.

## Key Patterns Used Here

- @./.agents/rules/conventions/rust/async-patterns.md - Async server patterns for MCP
- @./.agents/rules/conventions/rust/compiler-loop.md - Debugging protocol implementation

### JSON-RPC Message Handling
```rust
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,  // Must be "2.0"
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// MCP-specific error codes
impl JsonRpcError {
    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".to_string(),
            data: None,
        }
    }
    
    pub fn invalid_request() -> Self {
        Self {
            code: -32600,
            message: "Invalid Request".to_string(),
            data: None,
        }
    }
    
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: "Method not found".to_string(),
            data: Some(json!({"method": method})),
        }
    }
    
    pub fn invalid_params(message: &str) -> Self {
        Self {
            code: -32602,
            message: "Invalid params".to_string(),
            data: Some(json!({"details": message})),
        }
    }
    
    pub fn internal_error(message: String) -> Self {
        Self {
            code: -32603,
            message: "Internal error".to_string(),
            data: Some(json!({"details": message})),
        }
    }
}
```

### MCP Tool Implementation
```rust
use async_trait::async_trait;

#[async_trait]
pub trait McpTool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn input_schema(&self) -> Value;
    
    async fn call(&self, args: Value) -> Result<Value, JsonRpcError>;
}

pub struct SearchTool {
    search_service: Arc<SearchService>,
}

#[async_trait]
impl McpTool for SearchTool {
    fn name(&self) -> &'static str {
        "blz_search"
    }
    
    fn description(&self) -> &'static str {
        "Search across llms.txt documentation sources for relevant content"
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query string"
                },
                "alias": {
                    "type": "string",
                    "description": "Specific source alias to search (optional)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return",
                    "minimum": 1,
                    "maximum": 100,
                    "default": 10
                }
            },
            "required": ["query"]
        })
    }
    
    async fn call(&self, args: Value) -> Result<Value, JsonRpcError> {
        // Extract and validate parameters
        let query = args["query"].as_str()
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'query' parameter"))?;
            
        if query.trim().is_empty() {
            return Err(JsonRpcError::invalid_params("Query cannot be empty"));
        }
        
        let alias = args["alias"].as_str();
        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
        
        if limit > 100 {
            return Err(JsonRpcError::invalid_params("Limit cannot exceed 100"));
        }
        
        // Perform search
        let results = self.search_service.search(query, alias, limit).await
            .map_err(|e| JsonRpcError::internal_error(format!("Search failed: {}", e)))?;
        
        // Format response according to MCP tool result schema
        Ok(json!({
            "content": [{
                "type": "text",
                "text": format_search_results(&results)
            }]
        }))
    }
}

fn format_search_results(results: &SearchResults) -> String {
    if results.hits.is_empty() {
        return "No results found for your query.".to_string();
    }
    
    let mut output = format!(
        "Found {} results (showing top {}):\n\n", 
        results.total_count,
        results.hits.len()
    );
    
    for (i, hit) in results.hits.iter().enumerate() {
        output.push_str(&format!(
            "{}. **{}** ({})\n   {}\n   Lines {}-{}\n\n",
            i + 1,
            hit.title.trim(),
            hit.alias,
            hit.content.trim(),
            hit.line_range.start,
            hit.line_range.end
        ));
    }
    
    output
}
```

### Server Implementation
```rust
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use std::collections::HashMap;

pub struct McpServer {
    tools: HashMap<String, Box<dyn McpTool>>,
    initialized: bool,
}

impl McpServer {
    pub fn new() -> Self {
        let mut server = Self {
            tools: HashMap::new(),
            initialized: false,
        };
        
        // Register built-in tools
        server.register_tool(Box::new(SearchTool::new()));
        server.register_tool(Box::new(ListSourcesTool::new()));
        server.register_tool(Box::new(AddSourceTool::new()));
        
        server
    }
    
    pub fn register_tool(&mut self, tool: Box<dyn McpTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }
    
    pub async fn handle_connection(&mut self, stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
        let (reader, writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut writer = BufWriter::new(writer);
        
        let mut line = String::new();
        
        while reader.read_line(&mut line).await? > 0 {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                line.clear();
                continue;
            }
            
            // Parse JSON-RPC request
            let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
                Ok(req) => req,
                Err(_) => {
                    let error_response = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(JsonRpcError::parse_error()),
                    };
                    
                    let response_json = serde_json::to_string(&error_response)?;
                    writer.write_all(response_json.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                    
                    line.clear();
                    continue;
                }
            };
            
            // Handle request
            if let Some(response) = self.handle_request(request).await {
                // Send response
                let response_json = serde_json::to_string(&response)?;
                writer.write_all(response_json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
            }
            
            line.clear();
        }
        
        Ok(())
    }
    
    async fn handle_request(&mut self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone();
        
        match request.method.as_str() {
            "initialize" => Some(self.handle_initialize(request).await),
            "notifications/initialized" => { self.handle_initialized(); None }
            "tools/list" => Some(self.handle_list_tools(id)),
            "tools/call" => Some(self.handle_tool_call(request).await),
            _ => Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError::method_not_found(&request.method)),
            }),
        }
    }
    
    async fn handle_initialize(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "blz-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                }
            })),
            error: None,
        }
    }
    
    fn handle_initialized(&mut self) {
        self.initialized = true;
    }
    
    fn handle_list_tools(&self, id: Option<Value>) -> JsonRpcResponse {
        let tools: Vec<Value> = self.tools.values()
            .map(|tool| json!({
                "name": tool.name(),
                "description": tool.description(),
                "inputSchema": tool.input_schema()
            }))
            .collect();
        
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "tools": tools
            })),
            error: None,
        }
    }
    
    async fn handle_tool_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();
        
        if !self.initialized {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32000,  // Server error range
                    message: "Server not initialized".to_string(),
                    data: Some(json!({"details": "Call initialize method first"})),
                }),
            };
        }
        
        let params = match request.params {
            Some(p) => p,
            None => return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError::invalid_params("Missing parameters")),
            },
        };
        
        let tool_name = match params["name"].as_str() {
            Some(name) => name,
            None => return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError::invalid_params("Missing 'name' parameter")),
            },
        };
        
        let tool = match self.tools.get(tool_name) {
            Some(t) => t,
            None => return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError::method_not_found(tool_name)),
            },
        };
        
        let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
        
        match tool.call(args).await {
            Ok(result) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            },
            Err(error) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(error),
            },
        }
    }
}
```

### Main Server Entry Point
```rust
use tokio::net::TcpListener;
use tracing::{info, error};

async fn run_server(listener: TcpListener, mut server: McpServer) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let (stream, addr) = listener.accept().await?;
        info!("New connection from: {}", addr);
        
        // Clone server state for this connection  
        let server_handle = server.clone(); // Assume server implements Clone or use Arc
        tokio::spawn(async move {
            if let Err(e) = server_handle.handle_connection(stream).await {
                error!("Error handling connection {}: {}", addr, e);
            } else {
                info!("Connection {} closed", addr);
            }
        });
    }
}
```

### Error Recovery and Resilience
```rust
impl McpServer {
    async fn handle_connection_with_recovery(&mut self, stream: TcpStream) {
        let addr = stream.peer_addr().unwrap_or_else(|_| "unknown address".to_string());
        
        // Set connection timeout
        match tokio::time::timeout(
            Duration::from_secs(300), // 5 minutes
            self.handle_connection(stream)
        ).await {
            Ok(Ok(())) => {
                info!("Connection {} completed normally", addr);
            }
            Ok(Err(e)) => {
                error!("Connection {} error: {}", addr, e);
                // Log error but don't crash server
            }
            Err(_) => {
                error!("Connection {} timed out", addr);
            }
        }
    }
    
    // Graceful shutdown handler
    pub async fn shutdown(&mut self) {
        info!("Shutting down MCP server...");
        
        // Close any open connections
        // Clean up resources
        // Save state if necessary
        
        info!("MCP server shutdown complete");
    }
}

// Signal handling for graceful shutdown
use tokio::signal;
use std::time::Duration;

// Signal handling version (single entry point)
#[tokio::main] 
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    let host = std::env::var("MCP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("MCP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()?;
    
    let listener = TcpListener::bind(format!("{}:{}", host, port)).await?;
    info!("MCP Server listening on {}:{}", host, port);
    
    let server = McpServer::new();
    
    // Set up signal handling
    tokio::select! {
        _ = run_server(listener, server) => {
            info!("Server stopped");
        }
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
            // Graceful shutdown would be handled in run_server
        }
    }
    
    Ok(())
}
```

## MCP Protocol Compliance

### Required Methods
1. **initialize**: Capability negotiation
2. **notifications/initialized**: Confirm initialization
3. **tools/list**: List available tools
4. **tools/call**: Execute tool

### Response Format
All responses must include:
- `jsonrpc: "2.0"`
- `id`: Same as request (except notifications)
- Either `result` or `error` (never both)

### Error Handling
- Use standard JSON-RPC error codes
- Include helpful error messages and details
- Never let errors crash the server
- Log all errors for debugging

## Testing MCP Server
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_search_tool() {
        let tool = SearchTool::new();
        
        let args = json!({
            "query": "rust async",
            "limit": 5
        });
        
        let result = tool.call(args).await.unwrap();
        
        assert!(result["content"].is_array());
        assert!(!result["content"].as_array().unwrap().is_empty());
    }
    
    #[tokio::test]
    async fn test_invalid_parameters() {
        let tool = SearchTool::new();
        
        let args = json!({
            "limit": 5
            // Missing required "query" parameter
        });
        
        let result = tool.call(args).await;
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert_eq!(error.code, -32602); // Invalid params
    }
    
    #[tokio::test]
    async fn test_server_initialization() {
        let mut server = McpServer::new();
        
        let init_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(json!({})),
        };
        
        let response = server.handle_request(init_request).await;
        
        assert!(response.error.is_none());
        assert!(response.result.is_some());
        
        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert!(result["capabilities"]["tools"].is_object());
    }
}
```

## Deployment Considerations

### Configuration
```toml
# config.toml
[server]
host = "127.0.0.1"
port = 8080
timeout_seconds = 300

[logging]
level = "info"
format = "json"  # or "pretty" for development

[tools]
max_search_results = 100
search_timeout_seconds = 30
```

### Security
- Bind to localhost only by default
- Validate all input parameters
- Rate limit requests per connection
- Timeout long-running operations
- Don't expose internal error details

### Monitoring
- Log all requests and responses
- Track tool usage metrics  
- Monitor connection count
- Alert on error rates
- Health check endpoint

Remember: MCP compliance is essential for reliable Claude integration. Test thoroughly with actual Claude clients and follow the specification exactly.