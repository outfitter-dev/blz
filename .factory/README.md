# Factory Custom Commands for blz

This directory contains custom slash commands for the Factory AI development environment. These commands provide project-specific shortcuts and workflows for developing and testing the blz CLI tool, plus enhanced MCP (Model Context Protocol) integration.

## Available Commands

### Core Development Commands

### `/cli-test [focus-area]`
**Type:** Markdown template  
**Purpose:** Comprehensive testing of the blz CLI tool

Performs structured testing of blz functionality including:
- Environment and setup validation
- Core functionality tests (search, sources, indexing)
- Edge cases and error handling
- Output format validation
- Integration testing

**Usage Examples:**
- `/cli-test` - Full comprehensive test
- `/cli-test search` - Focus on search functionality
- `/cli-test performance` - Focus on performance testing
- `/cli-test error-handling` - Focus on error scenarios

### `/smoke-test [source-name]`
**Type:** Executable script  
**Purpose:** Quick automated validation of blz CLI

Runs automated smoke tests to verify basic functionality:
- Command availability and version check
- Help output validation
- Basic search operations
- JSON output testing
- Configuration validation

**Usage Examples:**
- `/smoke-test` - Test with first available source
- `/smoke-test rust` - Test specifically with rust source

### `/dev-setup [quick|full]`
**Type:** Markdown template  
**Purpose:** Development environment setup guidance

Provides comprehensive setup instructions for new contributors:
- Environment validation (Rust toolchain, dependencies)
- Build and test execution
- Development tool installation
- Project validation
- Workflow guidance

**Usage Examples:**
- `/dev-setup` - Full development setup
- `/dev-setup quick` - Minimal viable environment
- `/dev-setup full` - Complete setup with all tools

### `/review <branch-or-commit>`
**Type:** Markdown template  
**Purpose:** Structured code review for blz changes

Performs comprehensive code review focusing on:
- Rust code quality and safety
- Architecture and design patterns
- CLI-specific user experience
- Performance impact analysis
- Security and robustness

**Usage Examples:**
- `/review feature/new-search` - Review specific branch
- `/review HEAD~3..HEAD` - Review recent commits
- `/review main..feature-branch` - Review diff between branches

## MCP Integration Commands

### `/mcp-setup [linear|all]`
**Type:** Markdown template  
**Purpose:** Set up MCP servers for blz development workflow

Provides guided setup for Model Context Protocol servers:
- Linear MCP server for issue management
- blz documentation MCP server (planned)
- Environment configuration and validation
- Factory integration steps

**Usage Examples:**
- `/mcp-setup` - Complete MCP server setup
- `/mcp-setup linear` - Focus on Linear server only
- `/mcp-setup all` - Full setup with all available servers

### `/mcp-test [server-name]`
**Type:** Executable script  
**Purpose:** Test and validate MCP server configuration

Runs comprehensive MCP server testing:
- Configuration file validation
- Environment variable checking
- Server connectivity testing
- Requirements verification

**Usage Examples:**
- `/mcp-test` - Test all configured servers
- `/mcp-test linear` - Test Linear server specifically

### `/linear-issue <action> [issue-id]`
**Type:** Markdown template  
**Purpose:** Manage Linear issues for blz development

Comprehensive Linear issue management following blz conventions:
- Create issues with proper templates
- Update status and descriptions
- Generate git/PR templates with Linear IDs
- Search and link related issues

**Usage Examples:**
- `/linear-issue create` - Create new issue with templates
- `/linear-issue update BLZ-123` - Update existing issue
- `/linear-issue link BLZ-456` - Generate git/PR templates
- `/linear-issue search performance` - Find related issues

## MCP Integration Overview

The blz project includes enhanced MCP support for Factory integration:

### Linear MCP Server
- **Access Linear issues** directly from Factory sessions
- **Create and update tasks** following blz project conventions
- **Generate proper git workflows** with Linear ID integration
- **Search project history** and find related work

### Configuration Files

**`.mcp.json`** - MCP server configuration for Factory:
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

## Quick MCP Setup

1. **Get Linear API Key**: Visit [Linear Settings](https://linear.app/settings/api)
2. **Set Environment**: `export LINEAR_API_KEY="lin_api_..."`
3. **Add to Factory**: `/mcp add linear "npx -y @modelcontextprotocol/server-linear@latest" -e LINEAR_API_KEY=your_key`
4. **Test Setup**: `/mcp-test linear`

See [MCP_SETUP.md](./MCP_SETUP.md) for detailed configuration instructions.

## Linear Workflow Integration

### Branch Naming Convention
```bash
# Format: blz-123-feature-description
git checkout -b blz-123-add-search-pagination
```

### Commit Message Format
```bash
git commit -m "feat: add search pagination

Implements pagination for search results with configurable page size.

Fixes: BLZ-123"
```

### PR Title Format
```bash
# Format: type: description [BLZ-123]
feat: add search pagination [BLZ-123]
```

## Usage Notes

1. **Command Discovery**: Use `/commands` in Factory to see all available commands
2. **Reloading**: Press `R` in the commands UI to reload after making changes
3. **Arguments**: Commands support flexible argument handling with `$ARGUMENTS`
4. **Project Context**: All commands are designed specifically for blz development workflows
5. **MCP Integration**: Enhanced commands work with Linear MCP server when configured

## Command Development

These commands follow Factory's custom command conventions:

- **Markdown files** (`.md`) become prompt templates with YAML frontmatter
- **Executable files** with shebangs run as scripts and return output to chat
- **Filenames** are automatically slugified (spaces → dashes, lowercase)
- **Arguments** are passed via `$ARGUMENTS` (Markdown) or `$1, $2, etc.` (executables)

## Migration from Claude

These commands replace and improve upon the previous `.claude/commands/check/cli.md` implementation:

- **Enhanced Structure**: More comprehensive test plans and review criteria
- **Factory Syntax**: Proper YAML frontmatter and argument handling
- **Improved UX**: Better descriptions, hints, and flexible argument support
- **Automation**: Added executable scripts for quick validation
- **MCP Integration**: Linear workflow integration with Factory's MCP support
- **Documentation**: Clear usage examples and development guidance

## Contributing

When adding new Factory commands:

1. Follow the established naming and structure patterns
2. Include proper YAML frontmatter with description and argument hints
3. Test both standalone and integrated Factory usage
4. Update this README with command documentation
5. Consider MCP integration opportunities

For MCP-specific contributions:
1. Follow Linear conventions in `.agents/rules/LINEAR.md`
2. Test with various Linear workspace configurations
3. Ensure environment variable handling is secure
4. Document troubleshooting steps

See [Factory's custom commands documentation](https://docs.factory.ai/cli/configuration/custom-commands) for more details.