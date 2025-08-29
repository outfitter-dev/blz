//! Test case to verify compile-time API validation
//! This test ensures that non-existent functions are caught at compile time

fn main() {
    // Intentionally references a non-existent symbol to force a compile error
    // This validates that the API surface is properly exposed and typos are caught
    let _ = blz_core::this_function_does_not_exist();
}

