# Create Pull Request

When the user asks to create a pull request, follow these steps in sequence:

## Steps

### 1. Prepare Branch and Changes

- First check if all changes are committed
- List all staged and unstaged changes
- Commit all staged changes by grouping them logically. Use conventional commit messages.
- If there are unstaged changes, analyze them and ask the user if they should be committed and included in the PR
- Do not create new branches - use the current branch

### 2. Generate PR Content

- Use the git log to understand all commits since diverged from main branch
- Look at ALL commits in the branch, not just the latest one
- Analyze changes to:
  - Summarize the nature of changes (feature, bug fix, documentation, etc.)
  - Identify the purpose and motivation behind changes
  - Assess impact on the overall project
  - Ensure no sensitive information is included
- Look for any open issues that may be related or resolved by the changes in the PR. When referencing these issues in the PR body, use keywords like `Fixes #<issue>` to enable automatic issue closure.

### 3. Analyze PR Requirements

- Check project documentation for any PR specific formatting requirements
- Follow conventional commit standards
- Use this format for PR titles:
  - `type(scope): description`
  - Types: feat, fix, docs, style, refactor, perf, test, chore
  - Use present tense, imperative mood

### 4. Create PR

- Generate a proper PR title if none provided
- Follow the template and create a PR body with these sections:
  - **Summary**: Use bullet points to explain key changes
    - Use sublist items for any further details
  - **[Review|Test] Plan** (if applicable): Checklist of verification steps
    - Use checklist items to indicate verification steps
    - Use sublist items under the checklist items if further details are necessary
    - Consider specific changes that happened in the PR, and provide a detailed strategy for verifying they were implemented correctly
- Push any remaining changes to remote if needed
- Use GitHub CLI to create the PR: `gh pr create`

### 5. Validate

- Verify the PR was created successfully
- Return the PR URL to the user

## PR Body Template

```
## Summary

- [Concise description of changes]
  - [Additional details for individual changes]
- [Additional key changes if needed]

## [Review|Test] Plan

- [ ] [Verification steps]
  - [Additional details]
- [ ] [Testing instructions] (if applicable)
```