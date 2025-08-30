# GitHub Actions Workflows

This directory contains automated workflows for the blz Rust project.

## Workflows

### claude.yml
**Purpose**: Responds to explicit `@claude` mentions in pull requests
**Triggers**:

- PR comments with `@claude`
- PR review comments with `@claude`
- PR review submissions with `@claude`

**Allowed Tools**:

- `cargo build*` - Build the project
- `cargo test*` - Run tests
- `cargo clippy*` - Lint code
- `cargo fmt*` - Format code
- `cargo deny*` - Security/license checks
- `cargo shear*` - Unused dependency checks
- `make *` - Makefile commands
- `just *` - Justfile commands

**Custom Instructions**:

- Follows `.agents/rules/DEVELOPMENT.md`
- Ensures clippy compliance
- Requires tests for new functionality
- Updates AGENTS.md for architectural changes

### claude-code-review.yml
**Purpose**: Automated code review for pull requests
**Triggers**:

- PR opened (non-draft)
- Draft PR marked as ready
- Review requested on PR
- `@claude review` comment on PR

**Review Focus**:

- Rust idioms and best practices
- Memory safety (no unwrap/expect in production)
- Performance considerations for search operations
- Clippy warnings and code quality
- Test coverage
- Public API documentation
- Compliance with `.agents/rules/`
- AGENTS.md updates for architecture changes

**Review Commands**:

- `@claude review` - Full review of all changes
- `@claude review latest` or `@claude review recent` - Incremental review since last review

### dependencies.yml
**Purpose**: Security and dependency management
**Triggers**:

- Push to main (when Cargo files change)
- Pull requests (when Cargo files change)
- Weekly schedule (Mondays at midnight)
- Manual dispatch

**Jobs**:

1. **unused-deps**: Checks for unused dependencies with cargo-shear
2. **cargo-deny**: Validates dependencies for security, licenses, and bans
3. **security-audit**: Non-blocking security advisory check
4. **dependency-review**: Reviews dependency changes in PRs

## Required Secrets

- `CLAUDE_CODE_OAUTH_TOKEN` - Required for Claude workflows

## Allowed Bot Interactions

The following bots can trigger Claude actions:

- `@copilot[bot]`
- `@devin[bot]`
- `@coderabbitai[bot]`

All other bots are ignored to prevent automation loops.

## Local Testing

To test these workflows locally before pushing:

```bash
# Install act (GitHub Actions local runner)
brew install act  # macOS (or Linux if Homebrew is installed)
# OR download a release: https://github.com/nektos/act/releases
# Prerequisite: Docker must be running
# or
sudo apt install act  # Linux

# Test a specific workflow
act -W .github/workflows/dependencies.yml

# Test with specific event
act pull_request -W .github/workflows/claude-code-review.yml
```

## Maintenance

When updating workflows:

1. Test locally with `act` if possible
2. Create a PR to test the workflow changes
3. Monitor the Actions tab for any failures
4. Check workflow permissions are appropriate
5. Lint workflows with `actionlint` locally and in CI

Example CI job:

```yaml
name: workflow-lint
on: pull_request
jobs:
  actionlint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: raven-actions/actionlint@v1
```

## Notes

- All workflows use `actions/checkout@v4` for consistency
- Concurrency groups prevent duplicate runs
- Dependencies workflow continues on security advisories to avoid blocking
