# Security Policy

## Dependency Management

This project uses several tools to ensure dependency security and quality:

### cargo-deny
- **Purpose**: Comprehensive dependency validation including license compliance, security advisories, and dependency bans
- **Configuration**: `deny.toml`
- **Usage**: `cargo deny check` or `make deny`

### cargo-shear
- **Purpose**: Detect and remove unused dependencies
- **Usage**: `cargo shear` or `make unused`
- **Auto-fix**: `cargo shear --fix` or `just fix-unused`

## Security Checks

Run these checks before submitting PRs:

```bash
# Full security and dependency validation
make check-deps

# Or individually:
cargo deny check advisories  # Security advisories
cargo deny check licenses    # License compliance
cargo deny check bans        # Banned dependencies
cargo shear                  # Unused dependencies
```

## Reporting Security Vulnerabilities

If you discover a security vulnerability in `blz`, please:

1. **DO NOT** create a public GitHub issue
2. Email security details to the maintainers
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

## Known Security Advisories

Current advisories being tracked:

| Advisory | Package | Status | Notes |
|----------|---------|--------|-------|
| RUSTSEC-2024-0384 | instant | Monitoring | Used by tantivy, awaiting upstream fix |

To add exceptions for advisories that cannot be immediately fixed, update the `[advisories]` section in `deny.toml` with justification.

## License Policy

We maintain a strict license policy for dependencies:

### Allowed Licenses
- MIT, Apache-2.0, BSD variants (permissive)
- MPL-2.0 (weak copyleft, allows static linking)
- See `deny.toml` for complete list

### Prohibited Licenses
- GPL-3.0, AGPL-3.0 (strong copyleft)
- Any license not explicitly allowed

## Automated Checks

GitHub Actions runs security checks on:
- Every push to main
- All pull requests
- Weekly schedule (to catch new advisories)

See `.github/workflows/dependencies.yml` for CI configuration.