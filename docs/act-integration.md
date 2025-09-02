# Act Integration Guide

Local GitHub Actions testing with `act` for faster CI/CD feedback loops.

## Overview

`act` allows running GitHub Actions workflows locally in Docker containers, providing immediate CI feedback without pushing to GitHub. This integration optimizes the development workflow by catching CI failures before they reach the remote repository.

## Installation

```bash
# macOS
brew install act

# Linux/Windows
# See: https://github.com/nektos/act#installation
```

## Quick Start

```bash
# Run fast validation (format + clippy)
./scripts/act-validate.sh fast

# Run full CI locally
./scripts/act-validate.sh full

# Run tests only
./scripts/act-validate.sh test

# Verbose output for debugging
VERBOSE=1 ./scripts/act-validate.sh fast
```

## Validation Modes

### Fast Mode (<30s)
- Rust formatting check
- Basic Clippy validation (workspace bins only)
- Ideal for pre-commit hooks
- Command: `./scripts/act-validate.sh fast`

### Full Mode (2-5 minutes)
- Complete rust-ci workflow
- All Clippy checks (bins, examples, tests)
- Full test suite
- Build validation
- Command: `./scripts/act-validate.sh full`

### Specialized Modes
- **Format only**: `./scripts/act-validate.sh format`
- **Clippy only**: `./scripts/act-validate.sh clippy`
- **Tests only**: `./scripts/act-validate.sh test`

## Integration Points

### Pre-Push Hook (Enabled by Default)

The pre-push hook runs fast validation automatically before pushing:

```yaml
# lefthook.yml
pre-push:
  commands:
    act-validation:
      run: ./scripts/act-validate.sh fast rust-ci-local
```

To skip for a single push:
```bash
git push --no-verify
```

### Pre-Commit Hook (Optional)

For even earlier feedback, enable act in pre-commit by uncommenting in `lefthook.yml`:

```yaml
pre-commit:
  commands:
    act-fast:
      run: ./scripts/act-validate.sh fast rust-ci-local
```

### Manual Validation

Run before creating PRs:
```bash
# Quick check
./scripts/act-validate.sh fast

# Thorough validation
./scripts/act-validate.sh full
```

## Performance Optimization

### Container Reuse

Containers are reused by default for faster subsequent runs:
```bash
# First run: ~60s (downloads image, builds cache)
./scripts/act-validate.sh fast

# Subsequent runs: ~15-30s (reuses container)
./scripts/act-validate.sh fast
```

### Resource Limits

Configured in `.actrc`:
```
--container-options "--memory=4g --cpus=2"
```

Adjust based on your system capabilities.

### Workflow Selection

- **rust-ci-local.yml**: Optimized for local execution
- **rust-ci.yml**: Full CI workflow (slower locally)
- **miri.yml**: Not recommended locally (very slow)

## Troubleshooting

### "act: command not found"
Install act: `brew install act`

### Docker not running
Start Docker Desktop or Docker daemon

### Out of memory errors
Reduce memory limit in `.actrc` or close other applications

### Slow first run
Normal - downloading Docker images. Subsequent runs are faster.

### Container cleanup
```bash
# Remove act containers
docker container prune -f

# Remove act images (forces re-download)
docker image rm catthehacker/ubuntu:act-latest
```

## CI/CD Workflow Comparison

| Check | Local (act) | GitHub Actions | Time |
|-------|------------|----------------|------|
| Format | ✅ Fast mode | ✅ Always | <5s |
| Clippy (basic) | ✅ Fast mode | ✅ Always | 10-15s |
| Clippy (full) | ✅ Full mode | ✅ Always | 30-45s |
| Build | ✅ Full mode | ✅ Always | 45-60s |
| Tests | ✅ Full/test mode | ✅ Always | 30-45s |
| Miri | ❌ Too slow | ✅ Nightly | 30-60min |
| Coverage | ❌ Not local | ✅ PR only | 2-3min |

## Best Practices

1. **Use fast mode during development** - Quick feedback loop
2. **Run full mode before pushing** - Catch all issues
3. **Keep containers running** - Faster subsequent runs
4. **Update act regularly** - Bug fixes and improvements
5. **Don't skip on main/release branches** - Always validate critical branches

## Agent Instructions

For AI agents working with this repository:

### Before Making Changes
1. Run format check: `cargo fmt --check`
2. Run fast validation: `./scripts/act-validate.sh fast`

### Before Creating PRs
1. Run full validation: `./scripts/act-validate.sh full`
2. Fix any issues found
3. Document in PR if act validation passed

### Debugging CI Failures
1. Reproduce locally: `./scripts/act-validate.sh full`
2. Use verbose mode: `VERBOSE=1 ./scripts/act-validate.sh full`
3. Check specific job: `act -j rust -W .github/workflows/rust-ci.yml`

### Performance Expectations
- Fast mode: Should complete in <30 seconds
- Full mode: Should complete in <5 minutes
- If slower, check Docker resources and system load

## Configuration Files

- **`.actrc`**: Act configuration (platform, resources, defaults)
- **`.github/workflows/act-event.json`**: Default event for act
- **`.github/workflows/rust-ci-local.yml`**: Optimized workflow for act
- **`scripts/act-validate.sh`**: Validation script with modes
- **`lefthook.yml`**: Git hooks configuration with act integration

## Further Reading

- [Act Documentation](https://github.com/nektos/act)
- [GitHub Actions Locally](https://docs.github.com/en/actions/hosting-your-own-runners/about-self-hosted-runners)
- [Docker Resource Management](https://docs.docker.com/config/containers/resource_constraints/)