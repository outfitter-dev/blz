# BLZ MCP Server

> Local documentation search through the Model Context Protocol

The BLZ MCP server exposes BLZ's fast documentation search through the [Model Context Protocol](https://modelcontextprotocol.io), enabling AI agents to search and retrieve documentation with millisecond latency and exact line citations.

## Overview

The MCP server provides a Rust-native interface to BLZ's documentation cache. It's designed for:

- **Low context overhead**: <1 KB handshake size
- **Fast search**: Sub-50ms search latency (warm cache)
- **Precise retrieval**: Exact line citations instead of full pages
- **Read-only security**: Safe by default, no destructive operations

**Key capabilities:**

- Search across locally cached llms.txt documentation
- Retrieve precise snippets with exact line numbers
- Discover and add new documentation sources
- Access curated reference data and examples

## Quick Start

### Launch the Server

```bash
# Start MCP server (stdio mode)
blz mcp-server
```

The server communicates via JSON-RPC over stdin/stdout. It's designed to be launched by MCP-compatible clients (Claude Code, Cursor, etc.).

### Basic Usage

Once connected, the MCP client can use these tools:

```javascript
// Search for documentation
{
  "name": "find",
  "arguments": {
    "query": "test runner",
    "source": "bun",
    "maxResults": 5
  }
}

// Retrieve exact content
{
  "name": "find",
  "arguments": {
    "snippets": ["bun:304-324"],
    "contextMode": "symmetric"
  }
}
```

See [SETUP.md](SETUP.md) for client-specific configuration.

## Tools

The MCP server exposes 5 tools:

### 1. `find` - Search & Retrieve

Unified tool for searching documentation and retrieving exact content spans.

**Search mode:**

```json
{
  "query": "test runner",
  "source": "bun",
  "maxResults": 10
}
```

**Retrieval mode:**

```json
{
  "snippets": ["bun:304-324"],
  "contextMode": "symmetric"
}
```

**Parameters:**

- `query` (string, optional): Search text
- `snippets` (array, optional): Citation references (e.g., `["bun:120-145"]`)
- `source` (string, optional): Alias of the documentation source to search (required with `query`)
- `contextMode` (enum, optional): `"none"` | `"symmetric"` | `"all"` - How to expand snippets
- `linePadding` (integer, optional): Lines to add before/after (0-50)
- `maxResults` (integer, optional): Limit search hits (1-50, default: 10)

**Returns:**

```json
{
  "snippets": [
    {
      "alias": "bun",
      "lines": "304-324",
      "content": "### Test runner\nBun includes...",
      "headingPath": ["Bun Documentation", "Guides", "Test runner"]
    }
  ],
  "hits": [
    {
      "alias": "bun",
      "lines": "304-324",
      "headingPath": ["Bun Documentation", "Guides", "Test runner"],
      "snippet": "Bun includes a fast built-in test runner...",
      "score": 92.5,
      "sourceUrl": "https://bun.sh/llms.txt"
    }
  ],
  "executed": {
    "searched": true,
    "retrievedSnippets": false
  }
}
```

### 2. `list-sources` - List Documentation

List installed sources and registry candidates in one call.

**Parameters:**

- `filter` (string, optional): Filter sources by name (case-insensitive)

**Returns:**

```json
{
  "sources": [
    {
      "alias": "bun",
      "title": "Bun runtime docs",
      "url": "https://bun.sh/llms-full.txt",
      "kind": "installed",
      "fetchedAt": "2025-10-15T14:30:00Z",
      "metadata": {
        "totalLines": 42000,
        "headings": 156
      }
    },
    {
      "alias": "react",
      "url": "https://react.dev/llms-full.txt",
      "kind": "registry",
      "suggestedCommand": "blz add react"
    }
  ]
}
```

### 3. `source-add` - Add Documentation

Add new documentation source from the registry or a custom URL.

**Parameters:**

- `alias` (string, required): Source identifier
- `url` (string, optional): Custom URL (uses registry if omitted)
- `force` (boolean, optional): Overwrite existing source

**Returns:**

```json
{
  "alias": "astro",
  "url": "https://docs.astro.build/llms.txt",
  "message": "Added astro (2,451 headings, 18,732 lines) in 450ms"
}
```

### 4. `run-command` - Execute Safe Commands

Execute whitelisted read-only BLZ commands.

**Parameters:**

- `command` (string, required): Whitelisted command name (`stats`, `history`, `list`, `validate`, `inspect`, `schema`)
- `source` (string, optional): Documentation alias for commands that operate on a specific source

**Returns:**

```json
{
  "stdout": "Total sources: 3\nTotal indexed lines: 87,523\n...",
  "stderr": "",
  "exitCode": 0
}
```

**Note:** For write operations (add, update, remove), use the dedicated tools or run `blz` CLI directly.

### 5. `learn-blz` - Get Reference Data

Returns curated reference data about BLZ capabilities and usage patterns.

**No parameters required.**

**Returns:**

```json
{
  "prompts": [
    {
      "name": "discover-docs",
      "summary": "Find and add project docs"
    }
  ],
  "flags": {
    "contextMode": ["none", "symmetric", "all"],
    "source": ["bun", "react", "tanstack"]
  },
  "examples": [
    "find(query='test runner', source='bun')",
    "list-sources(filter='react')"
  ]
}
```

See [TOOLS.md](TOOLS.md) for detailed schemas and examples.

## Resources

Resources provide read-only access to BLZ metadata:

### `blz://sources/{alias}`

Individual source metadata:

```json
{
  "alias": "bun",
  "url": "https://bun.sh/llms-full.txt",
  "fetchedAt": "2025-10-15T14:30:00Z",
  "totalLines": 42000,
  "headings": 156,
  "lastUpdated": "2025-10-15T14:30:00Z"
}
```

### `blz://registry`

Available registry sources:

```json
[
  {
    "alias": "react",
    "url": "https://react.dev/llms-full.txt",
    "description": "React documentation",
    "category": "frontend"
  }
]
```

## Prompts

### `discover-docs` - Guided Doc Discovery

Helps discover and add relevant documentation for a project.

**Parameters:**

- `technologies` (string, required): Comma-separated list (e.g., `"bun,react,tailwind"`)

**Flow:**

1. Lists matching sources (installed + registry)
2. Suggests additions for missing sources
3. Demonstrates search patterns

**Example call:**

```json
{
  "name": "discover-docs",
  "arguments": {
    "technologies": "bun,react"
  }
}
```

## Performance

| Metric | Target | Typical |
|--------|--------|---------|
| Handshake size | <1 KB | ~800 bytes |
| Search (warm) | <50ms p95 | 6ms p50 |
| Search (cold) | <100ms p95 | 45ms p50 |
| Memory (idle) | <50 MB | ~30 MB |

Performance characteristics:

- **Index caching**: Indices remain loaded across requests
- **Zero allocations**: Hot paths avoid unnecessary allocations
- **Direct API access**: No CLI shell-outs, all operations via `blz-core`

See [Architecture](../architecture/README.md) for implementation details.

## Security Model

The MCP server follows a security-first design:

### Read-Only by Default

- Most operations are read-only
- `source-add` is the only mutation (explicit user action)
- No shell escape or arbitrary command execution

### Whitelisted Commands

The `run-command` tool only accepts safe, read-only commands:

```rust
const WHITELIST: &[&str] = &[
    "stats", "history", "list",
    "validate", "inspect", "schema"
];
```

### Path Sanitization

Output paths are sanitized to prevent information leakage:

- `$HOME` → `~`
- Workspace paths → `<project>`
- Absolute paths minimized where possible

### No Destructive Operations

Write operations (update, remove, clear) are not exposed. Users must run `blz` CLI directly for these operations.

## Common Workflows

### 1. Search and Retrieve

```javascript
// Step 1: Search for relevant documentation
const searchResult = await mcp.callTool("find", {
  query: "test runner",
  source: "bun",
  maxResults: 5
});

// Step 2: Pick the best hit
const citation = searchResult.hits[0];
const ref = `${citation.alias}:${citation.lines}`;

// Step 3: Retrieve full content with context
const content = await mcp.callTool("find", {
  snippets: [ref],
  contextMode: "symmetric"
});
```

### 2. Add New Documentation

```javascript
// Step 1: Check if source is available
const sources = await mcp.callTool("list-sources", {
  filter: "astro"
});

// Step 2: Add from registry
const result = await mcp.callTool("source-add", {
  alias: "astro"
});

// Step 3: Verify installation
const updated = await mcp.callTool("list-sources", {
  filter: "astro"
});
```

### 3. Discover Documentation

```javascript
// Use the discover-docs prompt
const guidance = await mcp.getPrompt("discover-docs", {
  technologies: "bun,react,tailwind"
});

// Follow the suggestions
// The prompt will list installed vs available sources
// and suggest specific commands to run
```

## Troubleshooting

### Server Won't Start

**Symptom:** `blz mcp-server` exits immediately or hangs

**Check:**

```bash
# Verify BLZ is installed
blz --version

# Test basic functionality
blz list

# Check for errors with debug logging
RUST_LOG=debug blz mcp-server
```

### Search Returns No Results

**Symptom:** `find` tool returns empty `hits` array

**Possible causes:**

1. Source not indexed - check `list-sources`
2. Typo in source name - sources are case-sensitive
3. Index corruption - try `blz validate <source>`

**Fix:**

```bash
# List installed sources
blz list

# Reindex if needed
blz refresh <source>  # deprecated alias: blz update <source>
```

### High Latency

**Symptom:** Search takes >100ms consistently

**Check:**

1. Cold cache - first query is slower
2. Large indices - some docs are 50K+ lines
3. System load - check CPU/memory

**Optimize:**

```bash
# Warm up the cache
blz search "test" --source bun

# Check index statistics
blz stats
```

### Tool Call Fails

**Symptom:** MCP client shows tool call error

**Common issues:**

- Invalid parameter types (string vs array)
- Missing required parameters
- Malformed citations (use `source:start-end` format)

**Debug:**

```bash
# Enable MCP debug logging
RUST_LOG=blz_mcp=debug blz mcp-server
```

### Path Shows Absolute Paths

**Symptom:** Output contains full file system paths

**This is a bug** - paths should be sanitized. Please file an issue with:

- The command that produced the output
- Your OS and BLZ version
- Example output (redact sensitive info)

## CLI Equivalents

For reference, here's how MCP tools map to CLI commands:

| MCP Tool | CLI Equivalent |
|----------|----------------|
| `find(query=...)` | `blz search "..." --json` |
| `find(snippets=...)` | `blz find source:lines --json` |
| `list-sources()` | `blz list --json` |
| `source-add(alias=...)` | `blz add alias url` |
| `run-command(command="stats")` | `blz stats` |

The MCP server uses the same `blz-core` library, so results should be identical to CLI output.

## Next Steps

- [SETUP.md](SETUP.md) - Configure the MCP server in your IDE
- [TOOLS.md](TOOLS.md) - Detailed tool schemas and examples
- [Architecture](../architecture/README.md) - Implementation details

## Additional Resources

- [Model Context Protocol Spec](https://spec.modelcontextprotocol.io/)
- [BLZ Architecture](../architecture/README.md)
- [CLI Documentation](../cli/README.md)
