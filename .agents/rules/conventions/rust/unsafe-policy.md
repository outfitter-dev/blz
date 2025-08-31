# Unsafe Code Policy

## Default: Deny Unsafe

This project denies unsafe code by default (allowing scoped exceptions with `#[allow(unsafe_code)]` when strictly justified):

```toml
[workspace.lints.rust]
unsafe_code = "deny"
```

## When Unsafe Might Be Needed

Unsafe code should only be considered in extreme circumstances:

1. **Performance-critical paths** where profiling shows unsafe provides a >2x improvement
2. **FFI requirements** for C library integration
3. **Platform-specific operations** not available through safe Rust

## Review Process

Any unsafe code must:

1. **Document safety requirements** with detailed `# Safety` comments
2. **Get explicit approval** in code review
3. **Have comprehensive tests** covering all safety assumptions
4. **Be isolated** to minimal scopes with safe wrappers

## Example of Justified Unsafe

```rust
/// # Safety
///
/// This function is safe to call when:
/// 1. `ptr` is valid and points to initialized memory
/// 2. The memory region is at least `len` bytes long
/// 3. No other code is accessing this memory concurrently
/// 4. The memory remains valid for the lifetime 'a
#[allow(unsafe_code)]
unsafe fn read_raw_bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    // Safety: Caller guarantees ptr is valid and len is correct
    std::slice::from_raw_parts(ptr, len)
}
```

## Safe Alternatives First

Always prefer safe alternatives:

```rust
// ❌ Unsafe pointer manipulation
unsafe { std::slice::from_raw_parts(ptr, len) }

// ✅ Safe slice operations
let end = offset.checked_add(len).ok_or(Error::BufferOverflow)?;
buffer.get(offset..end).ok_or(Error::BufferOverflow)?
```

## Current Status

As of this writing, the blz project uses **zero unsafe code** and should remain that way unless extraordinary circumstances require it.

Any PR introducing unsafe code will require:
- Detailed justification
- Performance benchmarks (if performance-motivated)
- Alternative approaches considered
- Senior reviewer approval