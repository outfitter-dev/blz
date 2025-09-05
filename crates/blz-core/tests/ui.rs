//! Compile-time UI tests for verifying error messages and type safety

#[test]
fn compile_fail_ui() {
    let t = trybuild::TestCases::new();
    // Add cases in a follow-up; running with no cases is OK
    t.compile_fail("tests/compile-fail/*.rs");
}
