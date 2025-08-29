# Improve Rust Development Reliability for AI Agents

Date: 2025-08-29

## Summary

Based on recent research on LLM/agent challenges with Rust development and an audit of the blz codebase, this issue outlines improvements needed to make our Rust development more reliable and agent-friendly.

## Current State Audit

### What We're Doing Right ✅

1. **Modern Rust Setup**
   - Using Edition 2024 (cutting edge!)
   - Stable toolchain with clear version requirements (1.85.0)
   - Workspace structure with resolver = "2"
   - Consistent dependency management through workspace deps

2. **Quality Gates**
   - Clippy with pedantic/nursery lints enabled
   - `cargo-shear` for unused dependency detection
   - `cargo-deny` for security and license checking
   - Automated dependency review in CI
   - Warning on dangerous patterns (`unwrap_used`, `expect_used`, `panic`, `todo`)

3. **Testing Infrastructure**
   - Property testing with `proptest` (in blz-core)
   - Comprehensive documentation structure
   - CI workflows for validation

4. **Documentation**
   - Extensive `.agent/rules/` directory with clear guidelines
   - Detailed STYLEGUIDE.md and development practices
   - Well-organized docs/development/ structure

### Critical Gaps to Address 🔴

1. **No Compiler-in-the-Loop Workflow**
   - Missing JSON diagnostic parsing (`cargo check --message-format=json`)
   - No automated `cargo fix` / `clippy --fix` integration
   - No macro expansion tooling (`cargo expand`)
   - Agents can't iterate on compiler errors effectively

2. **Missing Advanced Testing Tools**
   - No `cargo-nextest` for fast, machine-readable test reports
   - No `trybuild` for compile-fail tests
   - No Miri for unsafe code validation (we have unsafe code in cache.rs!)
   - No benchmark regression detection

3. **Unsafe Code Without Proper Validation**
   - `crates/blz-core/src/cache.rs` contains unsafe code (LRU cache implementation)
   - Only "warn" on unsafe_code, should be "forbid" with exceptions
   - No Miri CI job to validate unsafe code correctness
   - Missing safety documentation requirements

4. **Async/Ownership Patterns Not Codified**
   - No templates for `Send + 'static` task spawning
   - No guidelines on avoiding borrows across `.await`
   - Missing patterns for Arc/clone usage in async contexts

5. **Limited Agent-Friendly Documentation**
   - No local crate documentation cache (DEPS.md)
   - Missing "Rust patterns" guide for common scenarios
   - No macro expansion examples for debugging

### Moderate Improvements Needed 🟡

1. **Testing Coverage**
   - Tests exist but no coverage measurement
   - No integration test directory structure
   - Missing compile-fail tests for API misuse

2. **Error Handling**
   - Good use of `anyhow` and custom errors
   - But missing Result type aliases in some modules
   - Could benefit from more structured error contexts

3. **CI/CD Pipeline**
   - Has dependency checking but missing:
     - Coverage reports
     - Benchmark tracking
     - Miri unsafe validation
     - Automated fix application

## Proposed Implementation Plan

### Phase 1: Compiler-in-the-Loop (Critical)

1. **Create agent tooling script** (`scripts/agent-loop.sh`):
   ```bash
   #!/bin/bash
   # Run cargo check with JSON output
   cargo check --message-format=json 2>&1 | tee check.json

   # Apply automatic fixes
   cargo fix --allow-dirty --allow-staged
   cargo clippy --fix --allow-dirty --allow-staged

   # Expand macros on error
   if grep -q "derive" check.json; then
     cargo expand > expanded.rs
   fi
   ```

2. **Add to CI workflow** for agent validation

### Phase 2: Testing Infrastructure

1. **Add to workspace Cargo.toml**:
   ```toml
   [workspace.dev-dependencies]
   cargo-nextest = "0.9"
   trybuild = "1.0"
   ```

2. **Create integration test structure**:
   ```
   tests/
   ├── integration/
   │   ├── search.rs
   │   └── cache.rs
   └── compile-fail/
       └── api-misuse.rs
   ```

3. **Add Miri CI job** for unsafe validation

### Phase 3: Documentation & Patterns

1. **Create `DEPS.md`** with key crate documentation links
2. **Create `docs/rust-patterns.md`** with:
   - Async task spawning template
   - Error handling patterns
   - Builder pattern examples
   - Common trait bounds

3. **Add macro expansion examples** to troubleshooting guide

### Phase 4: Unsafe Code Hardening

1. **Document all unsafe blocks** with safety requirements
2. **Add Miri to CI pipeline**
3. **Consider replacing unsafe LRU with safe alternative** (e.g., `lru` crate)

### Phase 5: Coverage & Benchmarking

1. **Add coverage tools**:
   ```toml
   [workspace.dev-dependencies]
   cargo-llvm-cov = "0.6"
   ```

2. **Add benchmark regression detection** with criterion

## Success Metrics

- [ ] Agents can iterate on compiler errors without human intervention
- [ ] All unsafe code validated by Miri in CI
- [ ] 80%+ test coverage with automated reporting
- [ ] Compile-fail tests prevent API misuse
- [ ] Agent-friendly documentation reduces improvisation
- [ ] No raw `unwrap()` or `expect()` in production code

## Priority

**High** - These improvements directly address the fundamental challenges agents face with Rust, particularly around the compile-check-fix loop that is essential for reliable Rust development.

## References

- Research document on LLM/agent Rust challenges
- Current `.agent/rules/` guidelines
- Rust 2024 edition features and async improvements

## Labels

- `enhancement`
- `documentation`
- `testing`
- `ci/cd`
- `developer-experience`
- `agent-reliability`
