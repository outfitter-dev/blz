# blz MCP Server

Model Context Protocol (MCP) server implementation for blz, providing AI assistants with fast local search capabilities for cached llms.txt documentation.

## Overview

The blz MCP server exposes JSON-RPC methods that allow AI assistants to:
- List available documentation sources
- Search across cached documentation with millisecond latency
- Retrieve specific line ranges from documents

## Protocol Methods

### `list_sources`
Returns all cached documentation sources with metadata.

### `search`
Performs full-text search across cached documentation.

Parameters:
- `query` (required): Search query string
- `alias` (optional): Limit search to specific source
- `limit` (optional): Maximum results to return (default: 10)

### `get_lines`
Retrieves specific line ranges from cached documents.

Parameters:
- `alias` (required): Source alias
- `start` (required): Starting line number (1-based)
- `end` (required): Ending line number (inclusive)

## Development

The MCP server is built on:
- `jsonrpc-core` for JSON-RPC protocol handling
- `jsonrpc-stdio-server` for stdio transport
- `blz-core` for search and storage functionality

## Future Enhancements

- Streaming search results for large result sets
- Incremental indexing notifications
- Custom ranking parameters
- Semantic search capabilities