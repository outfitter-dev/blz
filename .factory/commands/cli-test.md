---
description: Comprehensive test of the blz CLI tool
argument-hint: [focus-area]
---

# blz CLI Comprehensive Test

Please perform a comprehensive test of the `blz` CLI tool with the following focus: $ARGUMENTS

## Test Plan

### 1. **Environment & Setup**
- Verify `blz` is properly installed and accessible
- Check version information with `blz --version`
- Validate configuration directory structure
- Test help output for main command and subcommands

### 2. **Core Functionality Tests**
- **Search Operations**: Test various search patterns, filters, and output formats
- **Source Management**: Add, list, update, and remove sources
- **Index Operations**: Test indexing, updates, and cache management  
- **Configuration**: Verify config file handling and environment variables

### 3. **Edge Cases & Error Handling**
- Invalid input validation
- Network connectivity issues
- Corrupted cache handling
- Permission errors
- Large dataset performance

### 4. **Output Format Testing**
- Text output formatting and readability
- JSON output structure and validity
- Error message clarity and helpfulness
- Search result relevance and ranking

### 5. **Integration Testing**
- Shell completion functionality
- Pipe/redirection compatibility
- Exit codes for scripting
- Configuration precedence (env vars vs config files)

## Expected Deliverables

1. **Test Summary**: Overall health status of the CLI
2. **Performance Metrics**: Search latency, indexing speed, memory usage
3. **Issue Report**: Any bugs, inconsistencies, or UX problems found
4. **Improvement Suggestions**: Specific recommendations with priority levels

## Special Instructions

- Use verbose output (`-v` or `--verbose`) when available to capture detailed logs
- Test with both small and large document sets if possible  
- Verify that all documented features work as described in help text
- Pay attention to consistency across similar commands (flags, output format, etc.)
- Test error recovery and graceful degradation scenarios

If you identify any issues, please:
1. Document exact reproduction steps
2. Include relevant error messages or unexpected outputs
3. Suggest specific fixes or improvements
4. Prioritize issues by severity (critical, high, medium, low)

Focus particularly on: $ARGUMENTS
