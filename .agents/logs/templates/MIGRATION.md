---
date: # e.g. 2025-08-29 23:00 UTC
migration: # e.g. config-format-v2, storage-paths
breaking: # true/false - is this a breaking change?
agent: # e.g. claude, codex, cursor, etc.
---

# Migration - [Name]

## Context

## Scope of Changes

## Migration Steps

### Pre-Migration Checklist

- [ ] …

### Automated Migration

### Manual Steps Required

## Rollback Plan

## Verification

### Automated Checks

### Manual Verification

## Timeline

## Communication Plan

---

## Example

```markdown
---
date: 2025-08-29 23:00 UTC
migration: config-format-v2
breaking: true
agent: claude
---

# Migration - Config Format v2

## Context

Moving from TOML to JSON for configuration files to support richer data structures and better tool integration.

## Scope of Changes

- All `.toml` config files → `.json`
- Settings structure reorganized for clarity
- New fields added for MCP server configuration
- Backwards compatibility maintained for 2 versions

## Migration Steps

### Pre-Migration Checklist

- [x] Backup existing config files
- [x] Document current config structure
- [x] Identify all config file locations
- [x] Test migration script locally

### Automated Migration

1. On first run, detect `.toml` files
2. Parse and validate existing config
3. Transform to new JSON structure
4. Write new `.json` files
5. Rename old files to `.toml.backup`

```rust
pub fn migrate_config() -> Result<()> {
    if legacy_config_exists() {
        let old = parse_toml_config()?;
        let new = transform_to_json(old)?;
        write_json_config(new)?;
        backup_old_config()?;
    }
    Ok(())
}
```

### Manual Steps Required

- Review migrated config for correctness
- Update any scripts referencing config files
- Update environment variable names if used

## Rollback Plan

1. Delete `.json` config files
2. Restore `.toml.backup` files to `.toml`
3. Use previous version binary
4. Report issue with migration details

## Verification

### Automated Checks

- Config loads successfully
- All settings accessible
- No data loss in transformation
- Backwards compat layer works

### Manual Verification

- Test each configuration option
- Verify MCP server starts with new config
- Check custom settings preserved
- Ensure defaults applied correctly

## Timeline

- v0.2.0: Migration introduced, both formats supported
- v0.3.0: JSON becomes default, TOML deprecated
- v0.4.0: TOML support removed

## Communication Plan

- Release notes with migration guide
- README update with new config examples
- GitHub discussion for questions
- Migration success/failure metrics logging

```
