# Act Validation for CI/CD

Use `act` to validate GitHub Actions workflows locally before pushing changes.

## When to Use Act

### ALWAYS Run Act Validation

- Before pushing any branch with CI-related changes
- After modifying `.github/workflows/*.yml` files  
- When fixing CI failures
- Before creating pull requests

### Validation Levels

1. **During Development** (every few commits)
   ```bash
   ./scripts/act-validate.sh fast
   ```
   Time: <30 seconds
   Validates: format, basic clippy

2. **Before Push** (automatic via pre-push hook)
   ```bash
   ./scripts/act-validate.sh fast rust-ci-local
   ```
   Time: <30 seconds
   Validates: format, clippy essentials

3. **Before PR Creation** (manual)
   ```bash
   ./scripts/act-validate.sh full
   ```
   Time: 2-5 minutes
   Validates: complete CI pipeline

## Debugging CI Failures

When GitHub Actions fail remotely:

1. **Reproduce Locally**
   ```bash
   # Same as GitHub Actions
   ./scripts/act-validate.sh full
   
   # With debug output
   VERBOSE=1 ./scripts/act-validate.sh full
   ```

2. **Check Specific Jobs**
   ```bash
   # Run specific workflow
   act -W .github/workflows/rust-ci.yml
   
   # Run specific job
   act -j rust -W .github/workflows/rust-ci.yml
   ```

3. **Interactive Debugging**
   ```bash
   # Drop into container shell
   act -j rust -W .github/workflows/rust-ci.yml --container-architecture linux/amd64 -s GITHUB_TOKEN=$GITHUB_TOKEN --privileged --container-options "--entrypoint /bin/bash"
   ```

## Common Patterns

### Working on CI Workflows

```bash
# 1. Make workflow changes
vim .github/workflows/rust-ci.yml

# 2. Test locally with act
./scripts/act-validate.sh fast

# 3. If good, test full workflow
./scripts/act-validate.sh full

# 4. Commit and push (pre-push will validate again)
gt create -am "ci: improve workflow performance"
gt submit --stack
```

### Fixing Failed CI

```bash
# 1. Check current CI status
gh run list --limit 5

# 2. Reproduce failure locally
./scripts/act-validate.sh full

# 3. Fix issues
cargo fmt
cargo clippy --fix

# 4. Validate fix
./scripts/act-validate.sh full

# 5. Push fix
gt modify -am "fix: resolve CI failures"
gt submit --stack
```

### Performance Testing

```bash
# Measure workflow performance
time ./scripts/act-validate.sh fast
time ./scripts/act-validate.sh full

# Compare with GitHub Actions
gh run view --log | grep "Run time"
```

## Performance Expectations

| Mode | Local (M1/M2) | Local (Intel) | GitHub Actions |
|------|---------------|---------------|----------------|
| Fast | 15-25s | 20-30s | N/A |
| Full | 2-3min | 3-5min | 1-2min |
| Test only | 30-45s | 45-60s | 30s |

## Optimization Tips

### Speed Up Act

1. **Keep containers running**
   ```bash
   # Don't use --rm flag
   export ACT_REUSE=1
   ```

2. **Allocate more resources**
   ```bash
   # Edit .actrc
   --container-options "--memory=8g --cpus=4"
   ```

3. **Use fast mode during development**
   ```bash
   # Only run full before push
   ./scripts/act-validate.sh fast  # Development
   ./scripts/act-validate.sh full  # Pre-PR
   ```

### Skip Act When Appropriate

- Documentation-only changes
- Non-code changes (README, LICENSE)
- Emergency hotfixes (use `--no-verify`)

## Troubleshooting

### Common Issues

**"Docker daemon not running"**
- Start Docker Desktop
- Or: `sudo systemctl start docker`

**"Out of memory"**
- Close other applications
- Reduce memory in `.actrc`
- Use fast mode instead of full

**"Container not found"**
- Run: `docker pull catthehacker/ubuntu:act-latest`

**"Workflow not found"**
- Check: `ls .github/workflows/`
- Use: `./scripts/act-validate.sh fast rust-ci-local`

### Reset Act Environment

```bash
# Clean up containers
docker container prune -f

# Remove act images
docker image prune -f

# Reset act cache
rm -rf ~/.cache/act

# Reinstall hooks
lefthook uninstall
lefthook install
```

## Integration with Graphite

When working with stacked PRs:

```bash
# 1. Sync your stack
gt sync --no-interactive

# 2. Make changes
# ... edit files ...

# 3. Validate with act
./scripts/act-validate.sh fast

# 4. Create commit
gt create -am "feat: add new feature"

# 5. Full validation before submit
./scripts/act-validate.sh full

# 6. Submit stack
gt submit --stack --no-interactive
```

## Key Commands Reference

```bash
# Installation check
act --version

# Quick validation
./scripts/act-validate.sh fast

# Full validation  
./scripts/act-validate.sh full

# Specific modes
./scripts/act-validate.sh format
./scripts/act-validate.sh clippy
./scripts/act-validate.sh test

# Debug mode
VERBOSE=1 ./scripts/act-validate.sh fast

# Skip reuse (fresh container)
ACT_REUSE=0 ./scripts/act-validate.sh fast

# Custom workflow
act -W .github/workflows/my-workflow.yml
```

## Remember

- Act runs in Docker containers, not your local environment
- First run is slow (downloading images), subsequent runs are fast
- Fast mode for development iteration, full mode for PR readiness
- Pre-push hook runs automatically - don't skip unless necessary
- Act can't access GitHub secrets - some workflows may fail locally