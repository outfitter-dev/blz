# CI/CD Pipeline

This document describes the continuous integration and deployment setup for the BLZ project.

## Overview

Our CI/CD pipeline is optimized for efficiency and cost-effectiveness, utilizing:
- **GitHub Actions** for automation
- **Graphite CI Optimization** for intelligent test skipping
- **Automated security and dependency checks**
- **Automated code review with Claude**

## Graphite CI Optimization

We use [Graphite's CI optimization](https://graphite.dev/docs/ci-optimization) to intelligently skip unnecessary CI runs, particularly beneficial when working with stacked PRs.

### How It Works

1. Each workflow starts with an `optimize_ci` job that checks with Graphite's API
2. Graphite determines if CI should run based on:
   - Stack position (e.g., only run on bottom PRs)
   - Previous CI results
   - Code changes
3. Subsequent jobs check `needs.optimize_ci.outputs.skip` before running

### Configuration

The optimization requires a `GRAPHITE_TOKEN` secret configured in the repository. This token is obtained from [Graphite's CI settings](https://app.graphite.dev/ci).

### Fail-Safe Design

If the Graphite API is unavailable or returns an error, CI runs normally to ensure nothing is blocked.

## Workflows

### Dependencies Workflow (`dependencies.yml`)

**Triggers:**
- Push to `main` branch (when Cargo files change)
- Pull requests (when Cargo files change)
- Weekly schedule (Mondays at midnight UTC)
- Manual dispatch

**Jobs:**
1. **unused-deps**: Checks for unused dependencies using `cargo-shear`
2. **cargo-deny**: Validates dependencies for security advisories, licenses, and banned crates
3. **security-audit**: Non-blocking security advisory check
4. **dependency-review**: Reviews dependency changes in PRs

### Claude Code Review (`claude-code-review.yml`)

**Triggers:**
- Pull request opened
- Pull request marked ready for review
- Review requested
- Comment with "@claude review"

**Features:**
- Automated code review using Claude AI
- Checks for code quality, security issues, and best practices
- Provides actionable feedback directly on PRs

### Claude Integration (`claude.yml`)

**Triggers:**
- Issue comments mentioning "@claude"
- PR review comments mentioning "@claude"
- PR reviews mentioning "@claude"
- Issues opened/assigned with "@claude" mention

**Access Control:**
- Restricted to repository owner (@galligan) and CODEOWNERS
- Supports Claude Code OAuth for secure API access

## Local Development

### Pre-commit Hooks

We use [Lefthook](https://github.com/evilmartians/lefthook) for Git hooks:

```bash
# Install lefthook
brew install lefthook  # or see other installation methods

# Install hooks
lefthook install

# Run hooks manually
lefthook run pre-commit
```

**Pre-commit checks:**
- Rust formatting (`cargo fmt`)
- Linting (`cargo clippy`)
- Commit message validation (conventional commits)

### Running CI Locally

Simulate CI checks before pushing:

```bash
# Format check
cargo fmt --check

# Linting
cargo clippy --workspace --all-targets -- -D warnings

# Tests
cargo test --workspace

# Security audit
cargo deny check

# Unused dependencies
cargo shear
```

## Best Practices

### For Contributors

1. **Use Stacked PRs with Graphite**:
   ```bash
   # Install Graphite CLI
   brew install withgraphite/tap/graphite
   
   # Create stacked branches
   gt create -m "feat: your feature"
   ```

2. **Keep PRs Small**: Smaller PRs are easier to review and less likely to conflict

3. **Write Descriptive Commit Messages**: Follow [conventional commits](https://www.conventionalcommits.org/):
   - `feat:` New features
   - `fix:` Bug fixes
   - `docs:` Documentation changes
   - `ci:` CI/CD changes
   - `test:` Test additions/changes
   - `refactor:` Code refactoring
   - `perf:` Performance improvements

4. **Run Local Checks**: Use pre-commit hooks or run checks manually before pushing

### For Maintainers

1. **Monitor CI Costs**: Review Graphite's CI analytics to optimize skip rules
2. **Keep Dependencies Updated**: Weekly automated checks help identify outdated packages
3. **Review Security Advisories**: Non-blocking security audit allows awareness without blocking PRs
4. **Leverage Claude Reviews**: Use automated reviews for consistent code quality checks

## Troubleshooting

### CI Skipped Unexpectedly

If Graphite skips CI when it shouldn't:
1. Check Graphite dashboard for skip rules
2. Ensure the PR is properly stacked
3. Force CI run by commenting `/ci run` (if configured)

### Graphite Token Issues

If CI fails with authentication errors:
1. Verify `GRAPHITE_TOKEN` secret is set in repository settings
2. Check token hasn't expired in Graphite dashboard
3. Ensure token has correct permissions

### Pre-commit Hook Failures

If pre-commit hooks fail:
1. Run `cargo fmt` to fix formatting
2. Address clippy warnings with `cargo clippy --fix`
3. For commit message issues, check conventional commit format

## Resources

- [Graphite Documentation](https://graphite.dev/docs)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Cargo Deny](https://github.com/EmbarkStudios/cargo-deny)
- [Lefthook Documentation](https://github.com/evilmartians/lefthook)
- [Conventional Commits](https://www.conventionalcommits.org/)