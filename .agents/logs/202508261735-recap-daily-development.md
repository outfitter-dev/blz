# blz Development Recap - 2025-08-26

## Current State Overview

### Open Pull Requests

#### PR #35: Remove unused pretty_assertions dependency
- **Status**: ✅ All CI checks passing
- **Purpose**: Fix CI failure from unused dependency
- **Changes**: 
  - Removed `pretty_assertions` from `blz-core/Cargo.toml`
  - Added documentation files (release plan, code review template)
  - Modified workspace `reqwest` to use `rustls-tls` instead of native TLS
  - Added Claude dispatch workflow
- **Issues**: Some scope creep beyond just removing the dependency
- **Recommendation**: Ready to merge after addressing minor scope concerns

#### PR #29: Use rustls instead of native-tls
- **Status**: ❌ Multiple CI failures
- **Purpose**: Better cross-platform portability
- **Changes**: Switch reqwest to use rustls-tls backend
- **Failures**:
  - Check for unused dependencies
  - Dependency Review
  - Dependency validation (bans/licenses/advisories)
- **Next Steps**: Need to fix CI issues before merge

### Priority Work (P0 Issues)

Based on the release plan in `.agents/memory/docs/20250825-blz-release-stack-plan.md`:

1. **Issue #31**: Correct heading-block extraction with exact line slices
   - Critical for accurate search results
   - Parser fix needed for proper line mapping

2. **Issue #32**: Unify storage paths to ~/.outfitter/blz
   - Breaking change requiring migration
   - Important for consistent user experience

3. **Issue #33**: Implement update command with ETag/Last-Modified
   - Essential for efficient cache updates
   - Needs archive support

4. **Issue #34**: Parallel multi-source search
   - Performance improvement for multiple sources
   - Reduce over-fetching

5. **Issue #36**: Tighten lints and improve error messages
   - Hide experimental diff command
   - Better user experience

### Recent Accomplishments

The project has made significant progress with recent merges:
- ✅ PR #17: Added lint target and clean CLI
- ✅ PR #16: Improved error handling and Unicode safety
- ✅ PR #15: Code quality improvements and maintenance
- ✅ PR #14: Standardized terminology and fixed examples
- ✅ PR #13: Comprehensive code improvements for production quality

### Next Most Likely Work

Based on priorities and current state:

1. **Immediate**: Fix PR #29 CI issues
   - Resolve dependency validation problems
   - Ensure all checks pass

2. **High Priority**: Issue #31 (Parser fix)
   - Most critical P0 issue
   - Affects search accuracy
   - Well-scoped technical fix

3. **Then**: Issue #32 (Storage path unification)
   - Important breaking change
   - Needs migration path
   - Sets foundation for other features

4. **Follow-up**: Issue #33 (Update command)
   - Builds on unified storage
   - Critical for usability

## Technical Context

### Architecture
- Rust workspace with 3 crates: `blz-core`, `blz-cli`, `blz-mcp`
- Using Tantivy for search indexing
- Tree-sitter for markdown parsing
- Strict Clippy lints and no unsafe code

### Key Files
- Parser logic: `crates/blz-core/src/parser.rs`
- Storage: `crates/blz-core/src/storage.rs`
- Index management: `crates/blz-core/src/index.rs`
- CLI commands: `crates/blz-cli/src/main.rs`

### Testing Status
- Need comprehensive test infrastructure (Issue #27)
- Performance testing infrastructure needed (Issue #28)
- Current tests passing but coverage could be improved

## Recommendations

1. **Merge PR #35** after confirming scope is acceptable
2. **Fix and merge PR #29** to improve portability
3. **Focus on Issue #31** (parser fix) as next implementation
4. **Plan breaking change** for Issue #32 carefully with migration

## Risks & Mitigations

- **Breaking changes**: Issue #32 requires careful migration path
- **Performance**: Need benchmarks before parallel search implementation
- **Test coverage**: Should improve tests alongside feature work

## Success Metrics

Per the release plan, v0.1 success requires:
- Search latency < 10ms (currently achieving ~6ms)
- Accurate line number citations
- Stable storage format
- Clean error messages
- Hidden experimental features

The project is on track for v0.1 release with clear priorities and good momentum.