---
description: Create or manage Linear issues for blz development
argument-hint: <action> [issue-id]
---

# Linear Issue Management for blz

Manage Linear issues for blz development work. Action: $ARGUMENTS

## Available Actions

### **create** - Create New Issue
Create a new Linear issue following blz project conventions:
- Use appropriate project and team assignments
- Set priority based on impact and urgency
- Include detailed description with acceptance criteria
- Add relevant labels (bug, feature, enhancement, etc.)
- Link to related issues or dependencies

### **update** - Update Existing Issue  
Update an existing Linear issue:
- Change status (Backlog → Todo → In Progress → In Review → Done)
- Update description, priority, or labels
- Add comments with progress updates
- Link to related PRs or commits

### **link** - Link Code to Issue
Create proper linkage between code and Linear issues:
- Generate git branch name using Linear's format
- Create commit messages with proper Linear magic words
- Format PR titles with Linear ID conventions
- Add issue references to code comments

### **search** - Search Issues
Find relevant Linear issues:
- Search by status, assignee, or project
- Filter by labels, priority, or date ranges
- Find related or dependent issues
- Check for duplicate or similar issues

## Linear Conventions for blz

Following the `.agents/rules/LINEAR.md` guidelines:

### **Branch Naming**
```bash
# Format: id-123-issue-slug
# Use Linear's "Copy git branch name" (Cmd/Ctrl + Shift + .)
git checkout -b blz-123-add-language-filtering
```

### **Commit Messages**
```bash
# Conventional commit with Linear footer
git commit -m "feat: add language filtering for multilingual docs

Implements filtering by language code in search results.

Fixes: BLZ-123
Refs: BLZ-124"
```

### **PR Titles**
```bash
# Format: type: description [ID]
feat: improve search performance [BLZ-123]
fix: resolve memory leak in indexer [BLZ-456]
```

## Magic Words Reference

**Closing Magic Words** (moves issue to Done):
- `fix`, `fixes`, `fixed`, `fixing`
- `close`, `closes`, `closed`, `closing`
- `resolve`, `resolves`, `resolved`, `resolving`
- `complete`, `completes`, `completed`, `completing`

**Non-Closing Magic Words** (keeps issue open):
- `ref`, `refs`, `references`
- `part of`, `related to`, `contributes to`
- `toward`, `towards`

## Issue Templates

### **Bug Report**
```
**Summary**: Brief description of the bug

**Expected Behavior**: What should happen
**Actual Behavior**: What actually happens
**Steps to Reproduce**: 
1. Step one
2. Step two
3. Step three

**Environment**: 
- OS: [macOS/Linux/Windows]
- blz version: [output of `blz --version`]
- Rust version: [output of `rustc --version`]

**Additional Context**: Screenshots, logs, etc.

**Acceptance Criteria**:
- [ ] Bug is fixed and verified
- [ ] Tests added to prevent regression
- [ ] Documentation updated if needed
```

### **Feature Request**
```
**Summary**: Brief description of the feature

**Problem**: What problem does this solve?
**Proposed Solution**: Detailed description of the feature
**Alternatives Considered**: Other approaches evaluated

**Acceptance Criteria**:
- [ ] Feature implemented according to specification
- [ ] Tests cover new functionality
- [ ] Documentation updated
- [ ] Performance impact assessed
```

## Usage Examples

Based on your specified action in $ARGUMENTS:

- `/linear-issue create` - Guide through creating a new issue
- `/linear-issue update BLZ-123` - Help update specific issue
- `/linear-issue link BLZ-456` - Generate git/PR templates for issue
- `/linear-issue search performance` - Find performance-related issues

## Integration Notes

When Linear MCP server is configured:
- Automatically fetch issue details and context
- Suggest related issues and dependencies  
- Validate issue status and assignments
- Cross-reference with existing code and PRs

Focus on: $ARGUMENTS