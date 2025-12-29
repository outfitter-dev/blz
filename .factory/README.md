# Factory Custom Commands for blz

This directory contains custom slash commands for the Factory AI development environment. These commands provide project-specific shortcuts and workflows for developing and testing the blz CLI tool.

## Available Commands

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

## Usage Notes

1. **Command Discovery**: Use `/commands` in Factory to see all available commands
2. **Reloading**: Press `R` in the commands UI to reload after making changes
3. **Arguments**: Commands support flexible argument handling with `$ARGUMENTS`
4. **Project Context**: All commands are designed specifically for blz development workflows

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
- **Automation**: Added executable smoke test for quick validation
- **Documentation**: Clear usage examples and development guidance

See [Factory's custom commands documentation](https://docs.factory.ai/cli/configuration/custom-commands) for more details.
