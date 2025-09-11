# Rust Patterns

## Async task spawning

```rust
pub fn spawn_worker<F, T>(fut: F) -> tokio::task::JoinHandle<anyhow::Result<T>>
where
    F: std::future::Future<Output = anyhow::Result<T>> + Send + 'static,
    T: Send + 'static,
{
    tokio::spawn(async move { fut.await })
}

// Note: awaiting the handle returns Result<anyhow::Result<T>, tokio::task::JoinError>.
// Handle both the join error (panic/cancellation) and the inner result:
// let handle = spawn_worker(work());
// let res = handle.await.context("worker join failed")?;
// let out = res?;

// For Tokio ≥1.37, you can name tasks for better debugging:
// tokio::task::Builder::new().name("worker").spawn(async move { fut.await })
```

## Avoid borrows across `.await`

- Clone `Arc`s and move owned data into the task
- Extract smaller async steps/functions to shorten borrow lifetimes
- Use `tokio::task::spawn_blocking` for blocking CPU/file work to avoid starving the runtime
- Prefer timeouts and cancellation-aware loops (e.g. `tokio::select!`, `CancellationToken`)

## Error context conventions

- Use `anyhow::Context` when calling fallible operations
- Prefer `thiserror` for typed error surfaces at crate boundaries

Example:

```rust
use anyhow::Context as _;
let data = std::fs::read(&path)
    .with_context(|| format!("reading {}", path.display()))?;
```

## Macro debugging tips

- Use `cargo expand` to inspect generated code
- Add minimal repros to `tests/` with clear compile errors
- Use `trybuild` compile-fail tests to lock diagnostics:

```rust
// tests/compile_fail.rs
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/*.rs");
}
```
