---
description: Comprehensive code review for blz changes
argument-hint: <branch-or-commit>
---

# blz Code Review

Please perform a comprehensive code review of: $ARGUMENTS

## Review Focus Areas

### 1. **Rust Code Quality**
- **Safety**: Check for unsafe blocks, unwrap(), panic!(), and other dangerous patterns
- **Performance**: Identify potential bottlenecks, unnecessary allocations, or inefficient patterns
- **Error Handling**: Verify proper error propagation and user-friendly error messages
- **Memory Management**: Look for potential leaks or inefficient memory usage
- **Async Patterns**: Check async/await usage follows project conventions

### 2. **Architecture & Design**
- **Separation of Concerns**: Ensure blz-core, blz-cli, and blz-mcp boundaries are respected
- **API Design**: Check for consistency with existing patterns and interfaces
- **Configuration**: Verify proper handling of config files and environment variables
- **Testing**: Ensure adequate test coverage for new functionality

### 3. **CLI-Specific Concerns**
- **User Experience**: Check command line interface consistency and usability
- **Help Text**: Verify documentation accuracy and completeness
- **Output Formatting**: Ensure consistent text and JSON output formats
- **Backward Compatibility**: Flag any breaking changes to existing commands

### 4. **Performance Impact**
- **Search Latency**: Ensure changes don't negatively impact search performance
- **Index Building**: Check that indexing speed remains within acceptable bounds
- **Memory Usage**: Verify no significant memory footprint increases
- **Startup Time**: Ensure CLI startup remains fast

### 5. **Security & Robustness**
- **Input Validation**: Check for proper validation of user inputs
- **File System Access**: Verify safe handling of file operations
- **Network Requests**: Check HTTP client usage and error handling
- **Data Integrity**: Ensure cache and index data integrity is maintained

## Review Deliverables

Please provide:

1. **Executive Summary**
   - Overall assessment of code quality
   - Major concerns or blockers
   - Recommendation (approve/request changes/block)

2. **Detailed Findings**
   - Code quality issues with specific file:line references
   - Performance concerns with impact analysis
   - Security or safety issues requiring immediate attention
   - Style or convention violations

3. **Testing Assessment**
   - Test coverage evaluation
   - Missing test scenarios
   - Suggestions for additional test cases

4. **Documentation Review**
   - Help text accuracy
   - Code comment quality
   - README or docs updates needed

## Review Guidelines

- **Be Specific**: Include file paths and line numbers for all feedback
- **Be Constructive**: Suggest concrete improvements, not just problems
- **Consider Impact**: Distinguish between critical issues and minor improvements
- **Check Dependencies**: Verify any new dependencies are justified and secure
- **Validate Claims**: Test any performance or functionality claims in the PR

For critical issues, please:
- Mark as "MUST FIX" with clear reasoning
- Provide specific remediation steps
- Suggest alternative approaches when applicable

Focus particularly on: $ARGUMENTS