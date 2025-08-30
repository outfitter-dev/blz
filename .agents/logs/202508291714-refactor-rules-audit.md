# Rules Audit for Cache Project (Rust)

## Overview

This document audits existing rules from TypeScript/JavaScript projects (rulesets, monorepo, carabiner) to determine what can be adapted for the Rust-based Cache project.

## Source Repositories Analyzed

1. **rulesets/** - AI rules compiler (TypeScript, Bun, Turbo)
2. **monorepo/** - Core shared libraries (TypeScript, Bun)
3. **carabiner/** - TypeScript monorepo for Claude Code hooks (Bun, Turbo, Ultracite)

## Rules Classification

### 1. Directly Applicable (Can Copy As-Is)

These rules are language-agnostic and apply equally to Rust projects:

#### Version Control & Git

- **create-pr.md** (rulesets) - PR creation workflow
- **commits.md** (carabiner) - Conventional commits standard
- Git workflow patterns (branch naming, commit messages)

#### Development Philosophy

- **CORE.md** (carabiner) - Engineering principles (Claude's identity)
- **mode-max-eng.md** (monorepo) - Max's engineering principles
- **IMPORTANT.md** (carabiner) - Priority rules aggregator

#### Project Organization

- **MONOREPO.md** (carabiner) - Monorepo structure concepts
- **preferred-tech-stack.md** (monorepo) - Tech stack philosophy (needs adaptation)
- Directory structure patterns (though specific to language)

#### Documentation Standards

- **GREPABLE.md** (rulesets) - Version markers strategy (mixd-*)
- README structure and documentation patterns
- Changelog conventions

#### CI/CD & DevOps

- GitHub Actions patterns
- Quality gates philosophy
- Testing pipeline structure (adapt for Rust)

### 2. Needs Major Adaptation (TypeScript → Rust)

These rules contain valuable principles but need significant translation:

#### Code Quality & Linting

- **ultracite.md** (monorepo/carabiner) - TypeScript linting rules
  - Needs: Rust clippy rules, rustfmt configuration
  - Principles: Zero-tolerance for warnings, strict checking

#### Testing

- **TESTING.md** (carabiner) - Comprehensive testing strategy
  - Needs: Rust test framework (built-in), criterion for benchmarks
  - Keep: Coverage targets, test categories, performance requirements

#### Type Safety

- **typescript.md** (carabiner) - TypeScript conventions
  - Needs: Rust type system patterns, Result<T, E>, Option<T>
  - Keep: Make illegal states unrepresentable

#### Error Handling

- **ERRORS.md** (carabiner) - Error handling patterns
  - Needs: Rust Result pattern, thiserror/anyhow crates
  - Keep: Structured errors, error codes, logging

#### Security

- **SECURITY.md** (carabiner) - Security practices
  - Needs: Rust-specific security (memory safety, unsafe blocks)
  - Keep: Environment variables, dependency auditing, input validation

#### Performance

- **PERFORMANCE.md** (carabiner) - Performance optimization
  - Needs: Rust profiling tools (perf, flamegraph), cargo bench
  - Keep: Benchmarking philosophy, caching strategies

#### Architecture

- **ARCHITECTURE.md** (carabiner) - System architecture
  - Needs: Rust module system, workspace structure
  - Keep: Separation of concerns, dependency management

### 3. Not Applicable (TypeScript/JS Specific)

These rules don't translate to Rust:

- **bun.md** - Bun runtime specifics
- React/JSX rules from ultracite
- Next.js specific rules
- JavaScript-specific patterns (void operators, etc.)
- npm/pnpm/yarn package management

### 4. New Rules Needed for Rust

Areas where we need Rust-specific rules:

#### Rust Language Conventions

- Ownership and borrowing patterns
- Lifetime annotations best practices
- Safe vs unsafe code guidelines
- Trait design and implementation
- Module organization (mod.rs vs named files)

#### Rust Toolchain

- Cargo workspace configuration
- Cargo features and conditional compilation
- Cross-compilation targets
- cargo-deny for dependency auditing
- rustup toolchain management

#### Tantivy-Specific (Search Engine)

- Index configuration patterns
- Query building best practices
- Schema design principles
- Performance tuning for search

#### Rust Testing

- Unit tests in same file vs separate
- Integration tests structure
- Documentation tests
- Property-based testing (proptest/quickcheck)
- Benchmark with criterion

#### Rust Performance

- Zero-copy patterns
- SIMD optimizations
- Memory pool strategies
- Async/await best practices
- Profile-guided optimization

## Adaptation Strategy

### Phase 1: Direct Copies
Copy these files as-is:

- create-pr.md → cache/.agents/rules/version-control/create-pr.md
- commits.md → cache/.agents/rules/conventions/commits.md
- GREPABLE.md → cache/.agents/rules/conventions/grepable.md

### Phase 2: Core Philosophy
Adapt engineering principles:

- CORE.md + mode-max-eng.md → cache/.agents/rules/CORE.md (merged and adapted)

### Phase 3: Rust-Specific Conventions
Create new with AI assistance:

- cache/.agents/rules/conventions/rust.md (language conventions)
- cache/.agents/rules/conventions/cargo.md (build system)
- cache/.agents/rules/conventions/tantivy.md (search engine)
- cache/.agents/rules/conventions/clippy.md (linting)

### Phase 4: Adapted Rules
Transform TypeScript rules to Rust:

- TESTING.md → cache/.agents/rules/TESTING.md (Rust testing)
- ERRORS.md → cache/.agents/rules/ERRORS.md (Result pattern)
- SECURITY.md → cache/.agents/rules/SECURITY.md (Rust security)
- PERFORMANCE.md → cache/.agents/rules/PERFORMANCE.md (Rust perf)

## Key Principles to Maintain

Regardless of language, these principles remain:

1. **Strict Standards**: Zero warnings, comprehensive linting
2. **Test-Driven**: High coverage, fast tests, comprehensive scenarios
3. **Type Safety**: Make illegal states unrepresentable
4. **Performance**: Measure first, optimize with data
5. **Security**: Defense in depth, validate inputs
6. **Documentation**: Self-documenting code, comprehensive comments
7. **Error Handling**: Explicit, recoverable, logged
8. **Version Control**: Conventional commits, clean history

## Next Steps

1. Copy directly applicable rules
2. Use @agent-docs-librarian to research Rust best practices
3. Use @agent-research-engineer for modern Rust patterns
4. Use @agent-senior-engineer to implement conventions
5. Create Rust-specific rule files
6. Review and validate with subagents

## Notes for Subagents

When working on adaptations, reference this audit to understand:

- Which principles to preserve from TypeScript world
- What needs Rust-specific implementation
- Where to find original examples for reference
- Key differences between ecosystems

The goal is to maintain the same level of rigor and quality standards while embracing Rust's unique strengths (memory safety, zero-cost abstractions, powerful type system).
