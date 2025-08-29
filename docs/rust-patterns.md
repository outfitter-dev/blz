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
```

## Avoid borrows across `.await`
- Clone `Arc` and move owned data into tasks
- Extract smaller async steps to reduce borrow lifetimes

## Error context conventions
- Use `anyhow::Context` when calling fallible operations
- Prefer `thiserror` for typed error surfaces at crate boundaries

## Macro debugging tips
- Use `cargo expand` to inspect generated code
- Add minimal repros to `tests/` with clear compile errors

