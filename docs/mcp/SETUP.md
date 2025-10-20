# MCP Server Setup

This guide covers setting up the BLZ MCP server with various AI coding assistants.

## Prerequisites

1. **Install BLZ**: Follow the [installation guide](../quickstart.md)
2. **Verify installation**: `blz --version`
3. **Add at least one source**: `blz add bun https://bun.sh/llms.txt`

## Quick Install

### Recommended: User-level Installation

Install globally for use across all your projects:

<details>
<summary><strong>Claude Code CLI</strong></summary>

```bash
claude mcp add blz blz mcp --scope user
```

The `--scope user` flag installs BLZ for all your projects. Verify installation:

```bash
claude mcp list
```

</details>

<details>
<summary><strong>Cursor</strong></summary>

Add to your user settings (`~/.cursor/config/settings.json`):

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

Restart Cursor and verify in Command Palette → "MCP: Show Servers"

</details>

<details>
<summary><strong>Windsurf</strong></summary>

Add to `~/.windsurf/.mcp_config.json`:

```json
{
  "mcpServers": {
    "blz": {
      "serverUrl": "blz mcp",
      "transport": "stdio"
    }
  }
}
```

Restart Windsurf to activate.

</details>

<details>
<summary><strong>Claude Code Desktop</strong></summary>

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

Restart Claude Code and verify in the MCP panel.

</details>

<details>
<summary><strong>Codex</strong></summary>

Add to your Codex configuration:

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

</details>

<details>
<summary><strong>Factory CLI</strong></summary>

```bash
/mcp add blz blz mcp
```

Verify with `/mcp list`

</details>

<details>
<summary><strong>OpenCode</strong></summary>

Add to OpenCode MCP configuration:

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

</details>

<details>
<summary><strong>AMP</strong></summary>

Configure in AMP settings under MCP Servers:

```json
{
  "blz": {
    "command": "blz",
    "args": ["mcp"]
  }
}
```

</details>

## Installation Scopes (Claude Code CLI)

When using `claude mcp add`, choose the appropriate scope:

- `--scope user`: Available across **all your projects** (recommended for personal tools)
- `--scope project`: Shared with your **team** via `.mcp.json` (committed to version control)
- `--scope local`: This **project only**, not shared (default)

**Examples:**

```bash
# Personal use across all projects
claude mcp add blz blz mcp --scope user

# Team-shared configuration
claude mcp add blz blz mcp --scope project

# Project-specific, not shared
claude mcp add blz blz mcp --scope local
```

## Verification

After installation, verify BLZ is connected:

### Claude Code

1. Restart Claude Code
2. Open the MCP panel
3. Verify "blz" server is connected
4. Check available tools: `find`, `list-sources`, `source-add`, `run-command`, `learn-blz`

### Other Clients

1. Restart your IDE
2. Look for MCP server status indicator
3. Try a search: "Can you search the Bun docs for information about the test runner?"

## Optional: Debug Logging

For troubleshooting, enable debug logging:

<details>
<summary><strong>Claude Code</strong></summary>

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

</details>

<details>
<summary><strong>Other Clients</strong></summary>

Add the `RUST_LOG` environment variable to your configuration:

```json
{
  "command": "blz",
  "args": ["mcp"],
  "env": {
    "RUST_LOG": "debug"
  }
}
```

</details>

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

### Environment Variables

**BLZ-specific:**

- `BLZ_DATA_DIR`: Override data directory (default: `~/.blz`)
- `BLZ_GLOBAL_CONFIG_DIR`: Override config directory
- `RUST_LOG`: Set logging level

**Debug logging levels:**

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
