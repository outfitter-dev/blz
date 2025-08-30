# blz-core AGENTS

- Focus: performance‑critical library code; unsafe policy applies.
- Read first: @/.agents/rules/CORE.md, SECURITY.md, TESTING.md, ASYNC-PATTERNS.md.
- Unsafe:
  - Workspace lint: deny(unsafe_code) unless explicitly allowed here.
  - All unsafe blocks require `// SAFETY:` comments (invariants, aliasing, lifetimes).
- Patterns:
  - Avoid borrows across .await; use Arc + owned data.
  - Prefer `anyhow::Context` for error chains; `thiserror` for typed errors.
- Quick checks:
  - `./scripts/lint.sh`
  - `cargo miri test -p blz-core`

