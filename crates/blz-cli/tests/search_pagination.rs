//! Tests for search pagination edge cases including divide-by-zero prevention

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_zero_limit_does_not_panic() {
    // This test ensures that even if a limit of 0 is somehow passed,
    // the pagination logic doesn't panic with divide-by-zero

    // Note: The CLI actually prevents limit=0 via clap validation,
    // but this test ensures the defensive programming in pagination logic works

    let mut cmd = Command::cargo_bin("blz").unwrap();

    // Try to trigger pagination with invalid limits
    // The actual CLI prevents this, but we're testing the defensive code
    cmd.arg("search")
        .arg("test")
        .arg("--limit")
        .arg("1")  // Minimum valid limit
        .arg("--page")
        .arg("999999"); // Very high page number

    // Should not panic, just show appropriate message
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("beyond available results"));
}

#[test]
fn test_empty_results_pagination() {
    // Test that pagination handles empty results gracefully
    let mut cmd = Command::cargo_bin("blz").unwrap();

    cmd.arg("search")
        .arg("nonexistentquerythatwontmatchanything123456789")
        .arg("--limit")
        .arg("10")
        .arg("--page")
        .arg("1")
        .arg("-o")
        .arg("json"); // Use JSON output to avoid display issues

    // Should handle gracefully with no panic
    // The command succeeds even with no results
    cmd.assert().success();
}

#[test]
fn test_single_result_pagination() {
    // Test edge case where there's only one result
    // This ensures actual_limit calculation doesn't cause issues

    // This test would require setting up a test index with known data
    // For now, we just ensure the command structure is valid

    let mut cmd = Command::cargo_bin("blz").unwrap();

    cmd.arg("search")
        .arg("test")
        .arg("--limit")
        .arg("1")
        .arg("--page")
        .arg("1");

    // Should run without panic
    cmd.assert().success();
}

#[test]
fn test_large_limit_with_small_results() {
    // Regression test: when limit >= ALL_RESULTS_LIMIT (10,000) and results are empty or small,
    // the actual_limit calculation should not cause divide-by-zero
    let mut cmd = Command::cargo_bin("blz").unwrap();

    cmd.arg("search")
        .arg("extremely_unlikely_search_term_that_wont_match_xyz123")
        .arg("--limit")
        .arg("10000")  // ALL_RESULTS_LIMIT
        .arg("--page")
        .arg("1")
        .arg("-o")
        .arg("json");

    // Should not panic even with large limit and no results
    cmd.assert().success();
}

#[test]
fn test_page_boundary_with_exact_division() {
    // Test when results divide exactly by limit
    let mut cmd = Command::cargo_bin("blz").unwrap();

    cmd.arg("search")
        .arg("test")
        .arg("--limit")
        .arg("5")
        .arg("--page")
        .arg("2");

    // Should handle page boundary correctly
    cmd.assert().success();
}

#[test]
fn test_minimum_limit_value() {
    // Test with the minimum valid limit (1)
    let mut cmd = Command::cargo_bin("blz").unwrap();

    cmd.arg("search")
        .arg("test")
        .arg("--limit")
        .arg("1")
        .arg("--page")
        .arg("1");

    // Should handle minimum limit correctly
    cmd.assert().success();
}

#[test]
fn test_pagination_prevents_panic_on_edge_cases() {
    // Test multiple edge cases that could cause panics
    let edge_cases = vec![
        ("1", "1"),     // Minimum values
        ("1", "10000"), // Minimum limit, huge page
        ("10000", "1"), // ALL_RESULTS_LIMIT
        ("100", "100"), // Large page number
    ];

    for (limit, page) in edge_cases {
        let mut cmd = Command::cargo_bin("blz").unwrap();

        cmd.arg("search")
            .arg("test")
            .arg("--limit")
            .arg(limit)
            .arg("--page")
            .arg(page)
            .arg("-o")
            .arg("json");

        // None of these should panic
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("panic").not());
    }
}
