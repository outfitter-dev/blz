//! UI compile-fail test harness for blz-core
//!
//! These tests verify compile-time API constraints (not graphical UI).
//! They are marked `#[ignore]` by default due to slow compilation time (120+ seconds).
//! Run explicitly with: `cargo test --ignored compile_fail_ui`

#[test]
#[ignore = "Slow compile-time test. Run with: cargo test --ignored compile_fail_ui"]
fn compile_fail_ui() {
    let t = trybuild::TestCases::new();
    // Add cases in a follow-up; running with no cases is OK
    t.compile_fail("tests/compile-fail/*.rs");
}
