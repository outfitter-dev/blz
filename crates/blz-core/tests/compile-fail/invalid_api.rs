fn main() {
    // Intentionally references a non-existent symbol to force a compile error
    let _ = blz_core::this_function_does_not_exist();
}

