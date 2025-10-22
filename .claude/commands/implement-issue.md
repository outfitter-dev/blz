# Implement Linear Issue

Instructions:

## Context

Linear Team ID: `28630e87-24b7-4401-840b-600703c308fc`
Linear Team Name: `BLZ`

## Overview

Orchestrate a complete implementation workflow for the Linear issue using specialized subagents. Follow best practices for investigation, implementation, testing, and quality assurance.

## Important

- When working on this issue, ensure that you're using `gt` commands exclusively.
- If at any point you encounter errors or unexpected behavior, pause and discuss with the user.

## Preparation

1. **Look up issue details**:
   - Use `mcp__linear-server__get_issue` to fetch the $ARGUMENTS issue (e.g., `BLZ-123`)
   - Extract the `gitBranchName` field from the response (e.g., `blz-123-fix-something`)
   - Note any dependencies mentioned in the issue description (links to other issues like "Depends on BLZ-XXX" or "Blocked by BLZ-XXX")

2. **Check dependencies and stack placement**:
   - Review the Graphite stack: !`gt log short`
   - **If issue has dependencies**:
     - Check if dependency issues are already in the stack (look for their branch names in `gt log short`)
     - Verify dependency branches are below (downstack from) where this work will go
     - **If dependencies are missing or not yet merged**: Inform user that the issue is blocked and suggest implementing dependencies first
     - **If dependencies are complete**: Proceed, ensuring this branch builds on top of them
   - **Determine stack position**:
     - If dependencies exist in stack: This branch should be created on top of the highest dependency
     - If no dependencies: Can branch from trunk (main)
     - Check for related work in stack that this should build upon

3. **Verify branch alignment**:
   - Check current branch: !`git branch --show-current`
   - Compare with the recommended `gitBranchName` from Linear
   - **If `gitBranchName` does not match the current branch**:
     - Inform the user of the mismatch
     - Consider stack position: Suggest `gt create <gitBranchName>` from appropriate parent branch
     - Example: If building on BLZ-100, suggest: `gt checkout blz-100-some-feature && gt create <gitBranchName>`
     - Wait for user confirmation before proceeding
   - **If on the correct branch**: Verify it's in the right position in the stack (correct parent branch)
   - **If branch exists but wrong position**: Suggest `gt move --onto <parent-branch>` to reposition

## Workflow Sequence

### Phase 1: Setup & Investigation

1. **Fetch issue details from Linear**:
   - Use Linear MCP to get the full issue description, acceptance criteria, and context
   - Update issue status to "In Progress"
   - Create a TodoWrite task list based on acceptance criteria

2. **Investigate the problem** (if needed):
   - Launch `@agent-systematic-debugger` subagent for bugs or behavioral issues
   - Launch `@agent-senior-engineer` subagent for exploratory work on features
   - Provide clear context: issue description, relevant code locations, and expected behavior
   - Review investigation findings and confirm understanding before proceeding

### Phase 2: Implementation

3. **Implement the solution**:
   - Launch `@agent-senior-engineer` subagent with:
     - Clear problem statement from investigation
     - Specific implementation requirements
     - File paths and line numbers where changes are needed
     - Expected behavior and edge cases to handle
   - Have subagent report back with changes made and reasoning

4. **Add comprehensive tests**:
   - Launch `@agent-test-driven-developer` subagent with:
     - Description of what was implemented
     - Test scenarios that need coverage
     - Location of existing test files for pattern matching
   - Verify tests are added and pass

### Phase 3: Quality Assurance

5. **Run quality checks in sequence**:
   ```bash
   cargo fmt --all
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   ```

6. **Fix any issues found**:
   - If clippy warnings exist, launch `@agent-code-reviewer` subagent to fix them
   - If tests fail, launch `@agent-systematic-debugger` to investigate
   - Repeat quality checks until all pass

7. **Install and manually verify** (for CLI changes):
   ```bash
   cargo install --path crates/blz-cli --bin blz --locked
   ```
   - Test relevant commands manually
   - Verify behavior matches acceptance criteria
   - Integration tests often provide sufficient coverage

### Phase 4: Documentation & Completion

8. **Update documentation** (if needed):
   - Launch `@agent-senior-engineer` to update relevant docs:
     - README.md for user-facing changes
     - docs/cli/commands.md for CLI changes
     - Agent instructions (.claude/agents/) for workflow changes
   - Ensure examples use current best practices

9. **Commit changes with Graphite**:
   - **Scenario A - Initial implementation (no existing PR)**:
     - Use `gt modify -am "<message>"` to amend the branch's commit
     - Message should describe ALL changes in the branch comprehensively
     - End message with: `Fixes: BLZ-XXX` (the issue being implemented)
     - Include co-authorship footer:
       ```
       🤖 Generated with Claude Code (https://claude.com/claude-code)

       Co-Authored-By: Claude <noreply@anthropic.com>
       ```
   - **Scenario B - Fixing an existing PR (PR already exists)**:
     - Use `gt modify -acm "<message>"` to create a NEW commit
     - Message should describe the specific fix/change being made
     - End message with: `Fixes: BLZ-XXX` (the current issue, even if different from branch)
     - Branch ID and fix issue ID may differ - this is OK
     - Example: On branch `blz-100-feature`, fixing via `BLZ-105`, commit ends with `Fixes: BLZ-105`
   - **How to determine which scenario**:
     - Check if PR exists: Look at `gt log short` for PR numbers
     - If branch shows a PR number → Scenario B (use `-acm`)
     - If no PR number shown → Scenario A (use `-am`)

10. **Verify acceptance criteria**:
    - Review each criterion in the Linear issue
    - Mark completed items as [x] in the issue description
    - Add implementation comment to Linear with:
      - Summary of changes
      - Files modified
      - Test coverage added
      - Quality check results

11. **Update Linear status**:
    - Move issue to "In Review" when complete
    - Add final comment summarizing the implementation

## Post-Implementation Checklist

Before considering the work complete, verify:

- [ ] All acceptance criteria marked as complete or documented as deferred
- [ ] Tests added and passing (110+ tests should all pass)
- [ ] Clippy clean (zero warnings with `-D warnings`)
- [ ] Code formatted (cargo fmt)
- [ ] Documentation updated for user-facing changes
- [ ] Linear issue updated with implementation details
- [ ] Binary installed and spot-checked (for CLI changes)

## Key Principles

1. **Use specialized subagents**: Don't do everything yourself - delegate to experts
2. **Sequence matters**: Investigate → Implement → Test → Quality → Document
3. **Verify subagent work**: Review their changes and confirm correctness
4. **Keep Linear updated**: Update status and criteria throughout, not just at the end
5. **Quality is non-negotiable**: All tests pass, zero clippy warnings, code formatted
6. **Document as you go**: Update todos, Linear comments, and acceptance criteria incrementally

## Example Subagent Instructions

### For Investigation (systematic-debugger):

```text
You are investigating [ISSUE-ID]: [title]

Problem: [describe the bug or inconsistency]

Your tasks:
1. Find the relevant code in [specific files]
2. Identify why [specific behavior] occurs
3. Explain the root cause with file paths and line numbers
4. Suggest what needs to change to fix it

Do NOT make changes - only investigate and report back.
```

### For Implementation (senior-engineer):

```text
You are implementing the fix for [ISSUE-ID]: [title]

Root cause: [explanation from investigation]

Your tasks:
1. Modify [specific files] to [specific changes]
2. Ensure [edge cases] are handled
3. Keep changes minimal and focused
4. Report back with what you changed and why

Begin implementation now.
```

### For Testing (test-driven-developer):

```text
You are adding tests for [ISSUE-ID]: [title]

What was implemented: [summary]

Your tasks:
1. Add tests to [test file location]
2. Cover these scenarios: [list scenarios]
3. Follow existing test patterns in the file
4. Ensure all tests pass

Report back with tests added and verification they pass.
```

## Notes

- Adapt the sequence based on issue complexity - not all phases are needed for all issues
- Use fewer subagents for simple fixes
- Keep the user informed of progress through Linear updates throughout the process
