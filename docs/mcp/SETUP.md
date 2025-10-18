# MCP Server Setup

This guide covers setting up the BLZ MCP server with various AI coding assistants.

## Prerequisites

1. **Install BLZ**: Follow the [installation guide](../QUICKSTART.md)
2. **Verify installation**: `blz --version`
3. **Add at least one source**: `blz add bun https://bun.sh/llms.txt`

## Claude Code

### Configuration

1. Open Claude Code settings (Cmd/Ctrl + Shift + P → "Claude Code: Settings")
2. Navigate to MCP Servers
3. Add BLZ server:

```json
{
  "mcpServers": {
    "blz": {
      "command": "blz",
      "args": ["mcp"],
      "env": {
        "RUST_LOG": "warn"
      }
    }
  }
}
```

### Optional: Debug Logging

For troubleshooting, enable debug logging:

```json
{
  "mcpServers": {
    "blz": {
      "command": "blz",
      "args": ["mcp"],
      "env": {
        "RUST_LOG": "blz_mcp=debug,blz_core=debug"
      }
    }
  }
}
```

### Verification

1. Restart Claude Code
2. Open the MCP panel
3. Verify "blz" server is connected
4. Check available tools: `find`, `list-sources`, `source-add`, `run-command`, `learn-blz`

### Usage

Ask Claude to search documentation:

```
Can you search the Bun docs for information about the test runner?
```

Claude will use the `find` tool automatically:

```javascript
// Claude's internal call
find({
  query: "test runner",
  sources: ["bun"]
})
```

## Cursor

### Configuration

1. Open Cursor settings (Cmd/Ctrl + ,)
2. Navigate to Features → MCP Servers
3. Add BLZ configuration:

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

1. Restart Cursor
2. Open Command Palette (Cmd/Ctrl + Shift + P)
3. Run "MCP: Show Servers"
4. Verify "blz" appears in the list

### Usage

In the chat panel:

```
Find documentation about async/await in the Bun docs
```

Cursor will use the MCP server to search and retrieve relevant content.

## Other MCP Clients

### Generic Setup

For any MCP-compatible client:

1. **Stdio transport**: BLZ uses stdio for communication
2. **Command**: `blz mcp`
3. **No special arguments**: The server auto-detects stdio mode

**Example configuration:**

```json
{
  "servers": {
    "blz": {
      "command": "blz",
      "args": ["mcp"],
      "transport": "stdio"
    }
  }
}
```

### Environment Variables

**BLZ-specific:**

- `BLZ_DATA_DIR`: Override data directory (default: `~/.blz`)
- `BLZ_GLOBAL_CONFIG_DIR`: Override config directory
- `RUST_LOG`: Set logging level

**Example:**

```json
{
  "command": "blz",
  "args": ["mcp"],
  "env": {
    "BLZ_DATA_DIR": "/custom/path",
    "RUST_LOG": "info"
  }
}
```

## Testing the Connection

### Method 1: Inspector

Use the official MCP Inspector:

```bash
# Install inspector (if not already installed)
npm install -g @modelcontextprotocol/inspector

# Run BLZ MCP server through inspector
blz mcp | npx @modelcontextprotocol/inspector
```

The inspector provides a web UI to:
- View available tools and their schemas
- Send test requests
- Inspect responses
- Validate JSON-RPC messages

### Method 2: Manual JSON-RPC

Test with direct JSON-RPC calls:

```bash
# Start the server
blz mcp

# Send initialize request (paste this and press Enter)
{"jsonrpc":"2.0","method":"initialize","params":{},"id":1}

# Expected response
{"jsonrpc":"2.0","result":{"protocolVersion":"2024-11-05","capabilities":{...}},"id":1}

# Test a tool call
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"list-sources","arguments":{}},"id":2}
```

### Method 3: CLI Verification

Verify BLZ works independently:

```bash
# List sources
blz list --json

# Search
blz search "test runner" --json

# Get content
blz get bun:304-324 --json
```

If these work, the MCP server should work too.

## Common Setup Issues

### Server Not Found

**Symptom:** IDE shows "blz command not found"

**Cause:** `blz` not in PATH

**Fix:**

1. Find BLZ installation:
   ```bash
   which blz
   # Example output: /Users/you/.cargo/bin/blz
   ```

2. Use absolute path in config:
   ```json
   {
     "command": "/Users/you/.cargo/bin/blz",
     "args": ["mcp"]
   }
   ```

3. Or add to PATH permanently:
   ```bash
   # For Bash/Zsh
   echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc

   # For Fish
   set -Ux fish_user_paths $HOME/.cargo/bin $fish_user_paths
   ```

### Server Starts But No Tools

**Symptom:** MCP connection succeeds but no tools available

**Debug:**

```bash
# Check handshake
echo '{"jsonrpc":"2.0","method":"initialize","params":{},"id":1}' | blz mcp | jq '.result.capabilities'
```

**Expected output:**

```json
{
  "tools": {},
  "resources": {},
  "prompts": {}
}
```

If capabilities are empty, this is a bug. Please file an issue with:
- BLZ version: `blz --version`
- OS and architecture
- Output of the debug command above

### High Startup Latency

**Symptom:** IDE pauses for several seconds when starting MCP server

**Cause:** Cold index loading

**Optimize:**

1. **Reduce sources**: Only keep actively used documentation
   ```bash
   blz list
   blz remove unused-source
   ```

2. **Warm cache**: Search once to load indices
   ```bash
   blz search "test" --source bun
   ```

3. **Check stats**: Large indices take longer to load
   ```bash
   blz stats
   ```

### Permission Errors

**Symptom:** MCP server fails with "Permission denied"

**Cause:** BLZ data directory not writable

**Fix:**

```bash
# Check permissions
ls -la ~/.blz

# Fix if needed
chmod -R u+rw ~/.blz
```

Or specify a custom directory:

```json
{
  "command": "blz",
  "args": ["mcp"],
  "env": {
    "BLZ_DATA_DIR": "/tmp/blz-data"
  }
}
```

## Advanced Configuration

### Custom Data Directory

Use a project-specific BLZ cache:

```json
{
  "mcpServers": {
    "blz": {
      "command": "blz",
      "args": ["mcp"],
      "env": {
        "BLZ_DATA_DIR": "${workspaceFolder}/.blz"
      }
    }
  }
}
```

**Note:** `${workspaceFolder}` expansion depends on your IDE. Check IDE-specific documentation.

### Multiple Profiles

Run multiple BLZ instances with different configurations:

```json
{
  "mcpServers": {
    "blz-project": {
      "command": "blz",
      "args": ["mcp"],
      "env": {
        "BLZ_DATA_DIR": "/path/to/project-docs"
      }
    },
    "blz-system": {
      "command": "blz",
      "args": ["mcp"],
      "env": {
        "BLZ_DATA_DIR": "/path/to/system-docs"
      }
    }
  }
}
```

Each instance maintains separate caches and sources.

### Debug Logging Levels

Control logging verbosity:

| Level | Use Case | Setting |
|-------|----------|---------|
| `error` | Production | `RUST_LOG=error` |
| `warn` | Normal use | `RUST_LOG=warn` (default) |
| `info` | Troubleshooting | `RUST_LOG=info` |
| `debug` | Development | `RUST_LOG=debug` |
| `trace` | Deep debugging | `RUST_LOG=trace` |

**Module-specific logging:**

```json
{
  "env": {
    "RUST_LOG": "blz_mcp=debug,blz_core=info"
  }
}
```

## Performance Tuning

### Warmup Strategy

Pre-load indices for faster first queries:

```bash
# Add this to your shell startup or IDE launch script
blz search "warmup" --source bun >/dev/null 2>&1 &
```

### Memory Optimization

For systems with limited RAM:

1. **Reduce sources**: Keep only essential docs
2. **Monitor usage**: `ps aux | grep blz`
3. **Restart periodically**: Indices are reloaded fresh

### Latency Optimization

For faster searches:

1. **Use source filters**: Always specify `sources` parameter
2. **Limit results**: Use `maxResults` to reduce processing
3. **Cache indices**: Keep server running between requests

## Next Steps

- [README.md](README.md) - Overview and capabilities
- [TOOLS.md](TOOLS.md) - Detailed tool documentation
- [Troubleshooting](#testing-the-connection) - Common issues and solutions

## Support

If you encounter issues not covered here:

1. Check [GitHub Issues](https://github.com/outfitter-dev/blz/issues)
2. Run with debug logging and file an issue
3. Include:
   - BLZ version (`blz --version`)
   - OS and IDE
   - MCP configuration
   - Relevant logs
