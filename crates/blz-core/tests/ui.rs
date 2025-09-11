//! UI compile-fail test harness for blz-core

#[test]
fn compile_fail_ui() {
    let t = trybuild::TestCases::new();
    // Add cases in a follow-up; running with no cases is OK
    t.compile_fail("tests/compile-fail/*.rs");
}
