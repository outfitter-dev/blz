#[test]
fn compile_fail_ui() {
    // Skip if there are no compile-fail cases yet
    let has_cases = std::fs::read_dir("tests/compile-fail")
        .ok()
        .and_then(|iter| {
            Some(
                iter.filter_map(|e| e.ok())
                    .any(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false)),
            )
        })
        .unwrap_or(false);

    if has_cases {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/compile-fail/*.rs");
    } else {
        eprintln!("No compile-fail test cases found; skipping");
    }
}
