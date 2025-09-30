# Migration Notes (Pre-Release)

This file tracks breaking changes and deprecations that will matter once BLZ is publicly released, but are currently internal-only since there are no external users yet.

## Purpose

- Track API/behavior changes for future reference
- Help agents understand the evolution of features
- Document "deprecated" features that were never actually used externally
- Keep this information out of user-facing documentation until it's relevant

## v0.5.0 - Flavor Simplification (2025-09-30)

### What Changed
BLZ now automatically prefers `llms-full.txt` over `llms.txt` when both are available. The dual-flavor system that required user configuration has been simplified.

### Implementation Details
- Feature flag: `FORCE_PREFER_FULL = true` in `crates/blz-cli/src/utils/flavor.rs`
- New command: `blz upgrade` to migrate sources from llms.txt to llms-full.txt
- Hidden flags: `--flavor` flags tombstoned across add/search/update commands

### "Deprecated" Items (Never Externally Used)
These are marked as deprecated in v0.5.0 docs, but were never used by external users:

1. **`BLZ_PREFER_LLMS_FULL` environment variable**
   - Was: Toggle to prefer llms-full.txt
   - Now: Ignored (always prefers llms-full.txt)
   - Status: Can be removed entirely in future cleanup

2. **`prefer_llms_full` config setting**
   - Was: Boolean in config.toml
   - Now: Ignored
   - Status: Can be removed in future cleanup

3. **Per-source flavor overrides** in `blz.json`
   - Was: `sources[alias].preferred_flavor` setting
   - Now: Use `blz upgrade` command instead
   - Status: Can be removed in future cleanup

4. **`--flavor` CLI flags**
   - Was: User-facing flags on add/search/update
   - Now: Hidden but still work for backward compat
   - Status: Can be removed entirely in future cleanup

### Why This Matters Later
When BLZ is publicly released and gains users:
- Users upgrading from theoretical pre-v0.5.0 versions will see these deprecation warnings
- At that point, the warnings will be meaningful
- For now, they're just documentation overhead

### Future Cleanup (Post-Public Release)
Once BLZ is public and v0.5.0+ is the baseline:
1. Remove `FlavorMode` enum entirely (~15 storage functions)
2. Remove flavor-aware index schema (~2,400 lines)
3. Remove 3 flavor-specific test files
4. Remove `BLZ_PREFER_LLMS_FULL` env var handling
5. Remove deprecated config settings
6. Remove tombstoned `--flavor` flags

### Files with "Deprecated" Markers (v0.5.0)
These files contain deprecation notices that are technically premature:
- `docs/configuration/env-vars.md:14` - BLZ_PREFER_LLMS_FULL
- `docs/configuration/global-config.md:12` - prefer_llms_full
- `docs/configuration/defaults.md:11-13` - Both settings
- `CHANGELOG.md:20-23` - Deprecation section

### Agent Guidance
When working on v0.5.0+:
- Don't be confused by "DEPRECATED since v0.5.0" - it's forward-looking
- Assume llms-full.txt preference everywhere
- `upgrade` command is the primary migration path
- Old flavor infrastructure exists for backward compat but isn't actively used

## Future Sections (Template)

### v0.X.0 - Feature Name (Date)

#### What Changed
Brief description.

#### Implementation Details
- Key changes
- New commands/flags
- Behavior modifications

#### "Deprecated" Items (Never Externally Used)
List of things marked deprecated but never used externally.

#### Why This Matters Later
Context for post-release users.

#### Future Cleanup
What can be removed once there's a stable user base.

---

**Note**: This file is for internal/agent reference only. Don't link to it from user-facing documentation.