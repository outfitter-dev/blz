# blz MCP Server

Model Context Protocol (MCP) server for `blz`, enabling AI agents to search and retrieve documentation from indexed llms.txt sources.

## Overview

The `blz` MCP server provides AI agents with fast, local access to indexed documentation through the Model Context Protocol. This allows agents to search across documentation sources and retrieve exact content without internet requests.

## Installation & Setup

### Prerequisites

- `blz` CLI tool installed and configured
- MCP-compatible AI client (Claude Desktop, etc.)

### Configuration

Add to your MCP client configuration (e.g., `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "blz": {
      "command": "blz",
      "args": ["mcp"]
    }
  }
}
```

### Verification

Test that the server starts correctly:

```bash
blz mcp
```

You should see the server initialize and wait for MCP protocol messages.

## Available Tools

### `blz_search`

Search across all indexed documentation sources.

**Parameters:**
- `query` (required) - Search terms
- `alias` (optional) - Filter to specific source
- `limit` (optional) - Maximum results (default: 50)
- `format` (optional) - Output format: "text" or "json" (default: "text")

**Example Usage:**
```json
{
  "name": "blz_search",
  "arguments": {
    "query": "test runner",
    "alias": "bun",
    "limit": 10
  }
}
```

**Response:**
```json
{
  "results": [
    {
      "alias": "bun",
      "lines": "304-324",
      "score": 4.09,
      "heading_path": ["Bun Documentation", "Guides", "Test runner"],
      "content": "### Guides: Test runner..."
    }
  ],
  "total": 1,
  "query": "test runner"
}
```

### `blz_get`

Retrieve exact lines from an indexed source.

**Parameters:**
- `alias` (required) - Source to retrieve from
- `lines` (required) - Line range(s) to get
- `context` (optional) - Context lines around each range

**Example Usage:**
```json
{
  "name": "blz_get",
  "arguments": {
    "alias": "bun",
    "lines": "304-324",
    "context": 2
  }
}
```

**Response:**
```
302: Previous context line
303: Another context line
304: ### Guides: Test runner
305: 
306: Bun ships with a built-in test runner...
...
324: expect(result).toBe(true);
325: Next context line
326: Another context line
```

### `blz_list`

List all indexed documentation sources.

**Parameters:**
- `format` (optional) - Output format: "text" or "json" (default: "text")

**Example Usage:**
```json
{
  "name": "blz_list",
  "arguments": {
    "format": "json"
  }
}
```

**Response:**
```json
{
  "sources": [
    {
      "alias": "bun",
      "url": "https://bun.sh/llms.txt",
      "fetched": "2025-08-23T00:55:33Z",
      "lines": 364,
      "headings": 26
    }
  ]
}
```

### `blz_add`

Add a new documentation source.

**Parameters:**
- `alias` (required) - Short name for the source
- `url` (required) - URL to llms.txt file
- `auto_confirm` (optional) - Skip flavor selection prompts (default: false)

**Example Usage:**
```json
{
  "name": "blz_add",
  "arguments": {
    "alias": "node",
    "url": "https://nodejs.org/llms.txt",
    "auto_confirm": true
  }
}
```

**Response:**
```
✓ Added node (42 headings, 528 lines)
```

### `blz_update`

Update indexed sources with latest content.

**Parameters:**
- `alias` (optional) - Specific source to update
- `all` (optional) - Update all sources (default: false)

**Example Usage:**
```json
{
  "name": "blz_update",
  "arguments": {
    "alias": "bun"
  }
}
```

**Response:**
```
✓ Updated bun (26 headings, 364 lines)
```

### `blz_remove`

Remove an indexed source.

**Parameters:**
- `alias` (required) - Source to remove

**Example Usage:**
```json
{
  "name": "blz_remove",
  "arguments": {
    "alias": "old-source"
  }
}
```

**Response:**
```
✓ Removed old-source
```

## Resources

The MCP server exposes indexed documentation as resources that can be read directly.

### Resource Format

Resources follow the pattern: `blz://sources/{alias}`

**Example:**
- `blz://sources/bun` - Full Bun documentation
- `blz://sources/node` - Full Node.js documentation

### Reading Resources

Use the standard MCP resource reading:

```json
{
  "method": "resources/read",
  "params": {
    "uri": "blz://sources/bun"
  }
}
```

## Integration Patterns

### AI Agent Workflows

1. **Discovery**: Use `blz_list` to see available sources
2. **Search**: Use `blz_search` to find relevant documentation
3. **Retrieval**: Use `blz_get` to fetch specific content
4. **Expansion**: Use `blz_add` to index new sources as needed

### Example Agent Flow

```python
# 1. Check available sources
sources = await mcp_client.call_tool("blz_list", {"format": "json"})

# 2. Search for specific information
results = await mcp_client.call_tool("blz_search", {
    "query": "async/await patterns",
    "limit": 5
})

# 3. Get detailed content for best match
if results["results"]:
    best_match = results["results"][0]
    content = await mcp_client.call_tool("blz_get", {
        "alias": best_match["alias"],
        "lines": best_match["lines"],
        "context": 3
    })
```

## Performance Characteristics

- **Search Speed**: ~6ms for most queries (local index)
- **Memory Usage**: Minimal - indices are memory-mapped
- **Network**: Only during `add` and `update` operations
- **Concurrency**: Thread-safe for read operations

## Error Handling

The MCP server returns structured errors:

```json
{
  "error": {
    "code": -1,
    "message": "Source 'unknown' not found",
    "data": {
      "available_sources": ["bun", "node"]
    }
  }
}
```

### Common Error Codes

- `-1` - Source not found
- `-2` - Invalid line range
- `-3` - Network/fetch error
- `-4` - File system error
- `-5` - Parse error

## Configuration

### Server Settings

The MCP server uses the same configuration as the CLI:

```
~/.outfitter/blz/
├── sources/          # Indexed documentation
├── indices/          # Search indices  
└── config.json      # Server configuration
```

### Logging

Enable verbose logging:

```bash
blz mcp --verbose
```

## Security Considerations

- **Local Only**: No network access except for `add`/`update`
- **Read-Only**: Sources are indexed locally and read-only
- **Sandboxed**: No file system access outside data directory
- **Validated**: All URLs must serve valid llms.txt format

## Troubleshooting

### Server Won't Start

Check that `blz` is in your PATH:

```bash
which blz
```

Verify basic functionality:

```bash
blz list
```

### No Sources Available

Add your first source:

```bash
blz add bun https://bun.sh/llms.txt
```

### Search Returns No Results

Check that sources are indexed:

```bash
blz list
```

Update sources if stale:

```bash
blz update --all
```

### MCP Client Connection Issues

Verify MCP protocol version compatibility. The `blz` server supports MCP v0.1.0+.

## Development

### Testing the Server

Use the MCP development tools:

```bash
# Test basic functionality
echo '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}' | blz mcp

# Test a search tool call
cat << EOF | blz mcp
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "blz_search", 
    "arguments": {"query": "test"}
  },
  "id": 2
}
EOF
```

### Custom Integrations

The MCP server can be integrated into any MCP-compatible system. See the [MCP specification](https://spec.modelcontextprotocol.io/) for details.

## Limitations

- **Read-Only**: Cannot modify source content through MCP
- **Local Sources**: Only works with indexed documentation
- **Text Format**: Optimized for text-based documentation
- **Single User**: Designed for single-user local usage

## Future Enhancements

Planned MCP features:

- **Streaming Search**: Real-time search result streaming
- **Subscriptions**: Notifications when sources are updated  
- **Batch Operations**: Multiple tool calls in single request
- **Advanced Filtering**: More sophisticated search filters
- **Resource Watching**: Monitor source changes
