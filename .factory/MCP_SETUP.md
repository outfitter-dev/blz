# MCP Server Setup for Factory + blz Development

This document provides comprehensive setup instructions for configuring Model Context Protocol (MCP) servers to enhance blz development workflow in Factory.

## Overview

The blz project includes MCP server configurations to integrate:

1. **Linear MCP Server** - Access Linear issues, create tasks, manage project workflow
2. **blz Documentation MCP Server** - Search indexed documentation via blz (planned feature)

## Quick Setup

### 1. Linear MCP Server

**Prerequisites:**
- Linear account with API access
- Node.js/npx installed
- Linear API key

**Factory Commands:**
```bash
# Add Linear MCP server to Factory
/mcp add linear "npx -y @modelcontextprotocol/server-linear@latest" -e LINEAR_API_KEY=your_api_key_here

# Verify server is added
/mcp list

# Test server functionality  
/mcp get linear
```

**Environment Setup:**
```bash
# Set Linear API key (replace with your actual key)
export LINEAR_API_KEY="lin_api_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"

# Or add to your shell profile (.bashrc, .zshrc, etc.)
echo 'export LINEAR_API_KEY="lin_api_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"' >> ~/.zshrc
```

### 2. Get Linear API Key

1. Go to [Linear Settings](https://linear.app/settings/api)
2. Generate a new Personal API key
3. Copy the key (starts with `lin_api_`)
4. Set it as the `LINEAR_API_KEY` environment variable

### 3. Test Configuration

```bash
# Use the Factory command to test MCP setup
/mcp-test linear

# Or test all servers
/mcp-test
```

## Configuration Files

### `.mcp.json`
```json
{
  "mcpServers": {
    "linear": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-linear@latest"],
      "env": {
        "LINEAR_API_KEY": ""
      }
    },
    "blz-docs": {
      "command": "blz",
      "args": ["mcp-server"],
      "env": {
        "BLZ_MCP_SOURCES": "rust,typescript,react,node,python",
        "BLZ_MCP_MAX_RESULTS": "50"
      }
    }
  }
}
```

This configuration supports both stdio-based MCP servers that Factory can manage directly.

## Available Factory Commands

### `/mcp-setup [linear|all]`
**Purpose:** Guided setup of MCP servers with environment validation  
**Usage:** 
- `/mcp-setup linear` - Focus on Linear server only
- `/mcp-setup all` - Complete MCP server setup

### `/mcp-test [server-name]`  
**Purpose:** Test and validate MCP server configuration  
**Usage:**
- `/mcp-test` - Test all configured servers
- `/mcp-test linear` - Test Linear server specifically

### `/linear-issue <action> [issue-id]`
**Purpose:** Manage Linear issues following blz project conventions  
**Usage:**
- `/linear-issue create` - Create new issue with templates
- `/linear-issue update BLZ-123` - Update existing issue
- `/linear-issue link BLZ-456` - Generate git/PR templates
- `/linear-issue search performance` - Find related issues

## Linear Integration Features

With the Linear MCP server configured, you can:

### Issue Management
- Query issues by status, assignee, project
- Create new issues with proper templates
- Update issue status and descriptions
- Add comments and track progress

### Git Integration  
- Generate proper branch names: `blz-123-feature-description`
- Create commit messages with Linear magic words
- Format PR titles: `feat: description [BLZ-123]`
- Link commits to issues automatically

### Project Workflow
- Track sprint progress and milestones
- Link code changes to Linear issues
- Follow blz project conventions for issue management
- Integrate with GitHub via Linear's sync features

## Workflow Examples

### Creating a New Feature
```bash
# 1. Create Linear issue
/linear-issue create

# 2. Generate branch name (using Linear's copy feature)
git checkout -b blz-123-add-search-pagination

# 3. Make changes and commit with Linear magic words
git commit -m "feat: add search pagination

Implements pagination for search results with configurable page size.

Fixes: BLZ-123"

# 4. Create PR with Linear ID
# Title: feat: add search pagination [BLZ-123]
```

### Bug Fixing Workflow
```bash
# 1. Search for related issues
/linear-issue search memory leak

# 2. Update issue status to In Progress
/linear-issue update BLZ-456

# 3. Create branch and fix
git checkout -b blz-456-fix-memory-leak-indexer

# 4. Commit with proper reference
git commit -m "fix: resolve memory leak in indexer

Free allocated memory after index build completion.

Fixes: BLZ-456
Refs: BLZ-123"
```

## Troubleshooting

### Common Issues

**Linear API Key Not Working:**
- Verify key is valid at [Linear API settings](https://linear.app/settings/api)
- Check key has proper permissions
- Ensure environment variable is set correctly

**NPX Package Issues:**
- Update Node.js to latest stable version
- Clear npm cache: `npm cache clean --force`
- Try manual install: `npm install -g @modelcontextprotocol/server-linear`

**Factory MCP Integration:**
- Verify Factory CLI version supports MCP
- Check MCP server status: `/mcp list`
- Review server logs for connection issues

### Debugging Commands

```bash
# Check environment
echo $LINEAR_API_KEY

# Verify npx and Node.js
npx --version
node --version

# Test Linear MCP package directly
npx -y @modelcontextprotocol/server-linear@latest --help

# Check Factory MCP configuration
/mcp list
/mcp get linear
```

## Future Enhancements

### blz-docs MCP Server (Planned)
The `blz-docs` MCP server will provide:
- Direct access to blz search functionality
- Integration with indexed documentation sources
- Code example search and retrieval
- API reference lookup

This requires implementing the `blz mcp-server` command in the blz CLI.

### Advanced Linear Features
- Custom Linear workflows and automations
- Integration with blz performance metrics
- Automated issue creation from CI/CD failures
- Cross-project dependency tracking

## Contributing

When enhancing MCP integration:

1. Follow Linear conventions in `.agents/rules/LINEAR.md`
2. Test both Factory and direct MCP usage
3. Update documentation and Factory commands
4. Verify environment variable handling
5. Test with various Linear workspace configurations

For questions or issues with MCP setup, create a Linear issue using `/linear-issue create` with the "infrastructure" label.