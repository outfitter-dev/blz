---
description: Set up MCP servers for blz development workflow
argument-hint: [linear|all]
---

# MCP Server Setup for blz Development

Please help set up MCP (Model Context Protocol) servers to enhance the blz development workflow. Focus: $ARGUMENTS

## Available MCP Servers

### 1. **Linear MCP Server**
- **Purpose**: Access Linear issues, create tasks, update status
- **Configuration**: Requires `LINEAR_API_KEY` environment variable
- **Usage**: Query issues, create new tickets, update status

### 2. **blz Documentation MCP Server** 
- **Purpose**: Access indexed documentation via blz search capabilities
- **Configuration**: Uses local blz installation with configured sources
- **Usage**: Search documentation, get code examples, find reference materials

## Setup Tasks

### 1. **Environment Configuration**
- Verify LINEAR_API_KEY is properly set in environment
- Check blz CLI is installed and configured with relevant sources
- Validate Node.js/npx availability for Linear MCP server

### 2. **MCP Server Verification**
- Test Linear MCP server connection and authentication
- Verify blz MCP server functionality with sample queries
- Check MCP server status and availability

### 3. **Factory Integration**
- Add MCP servers to Factory configuration using `/mcp add` commands
- Configure environment variables and headers as needed
- Test MCP integration within Factory sessions

## Setup Commands

Based on the current `.mcp.json` configuration, here are the Factory commands to add the servers:

```bash
# Add Linear MCP server
/mcp add linear "npx -y @modelcontextprotocol/server-linear@latest" -e LINEAR_API_KEY=your_api_key_here

# Add blz documentation MCP server (when available)
/mcp add blz-docs "blz mcp-server" -e BLZ_MCP_SOURCES=rust,typescript,react,node,python -e BLZ_MCP_MAX_RESULTS=50
```

## Usage Examples

After setup, you can use MCP tools in Factory:

- **Linear Integration**: Create issues, query project status, update tasks
- **Documentation Search**: Access indexed docs via blz search capabilities
- **Cross-Reference**: Link code changes to Linear issues automatically

## Setup Focus Areas

**Linear Focus** ($ARGUMENTS contains "linear"):
- Focus exclusively on Linear MCP server setup
- Verify API key configuration and permissions
- Test Linear-specific functionality (issues, projects, teams)

**Complete Setup** (default or $ARGUMENTS contains "all"):
- Set up all available MCP servers
- Configure both Linear and blz documentation servers
- Provide comprehensive testing and validation

## Expected Outcome

After setup completion:
1. MCP servers are properly configured in Factory
2. Environment variables are correctly set
3. All MCP functionality is tested and working
4. Documentation on usage and troubleshooting is provided

If setup encounters issues:
- Provide specific error diagnosis and resolution steps
- Suggest alternative configuration approaches
- Document any known limitations or workarounds