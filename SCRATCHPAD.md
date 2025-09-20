# Project Scratchpad — Release v0.3

_Last updated: 2025-09-20_

Keep this file current. Update it whenever the branch stack changes or new work is queued up.

## Current Stack

**Status**: Stack split complete - 5 layers created from mega branch
**Stack**:
- `v0.3/1-version-deps` - Version bumps to 0.3.0
- `v0.3/2-storage` - Storage layer with flavor support
- `v0.3/3-core-functionality` - Commands, utils, main integration
- `v0.3/4-tests` - Integration tests
- `v0.3/5-docs-release` - Documentation and release automation

### ✅ Stack Split Complete

Successfully split the `gt-v0.3/mega` branch into 5 compilable layers:
1. **Version & deps** - Clean version bump
2. **Storage** - Core types and index with flavor support
3. **Core functionality** - All commands, utils, and main integration
4. **Tests** - Integration tests for new features
5. **Docs & release** - Documentation updates and automation

Each layer:
- ✅ Compiles independently
- ✅ All tests pass (271 total)
- ✅ Ready for review

## Outstanding Work

- **Submit for review**
  - [ ] Submit stack as draft PRs
  - [ ] Verify CI passes on all PRs
  - [ ] Remove draft status
  - [ ] Request reviews

- **Release readiness**
  - [ ] Produce release notes once approved
  - [ ] Tag v0.3.1 for release
  - [ ] Manual release process (Homebrew/NPM later)

## Tracking

- GitHub issue: [#196 – Release v0.3 tracking](https://github.com/outfitter-dev/blz/issues/196)

## Notes

- Consolidated from planned 19 layers to 5 for better maintainability
- Each layer is atomic and testable
- Fixed test that was failing due to missing `-y` flag

## Handoff — 2025-09-20

- **Current branch:** `v0.3/5-docs-release`
- **Stack status:** Complete and passing all tests
- **Next actions:**
  1. Submit stack for review: `gt submit --stack`
  2. Monitor CI for any issues
  3. Address review feedback if any