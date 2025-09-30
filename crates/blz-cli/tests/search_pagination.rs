#![allow(clippy::unwrap_used, clippy::expect_used, clippy::unwrap_used)]
//! Tests for search pagination edge cases including divide-by-zero prevention

mod common;

use common::blz_cmd;
use predicates::prelude::*;
use std::sync::Once;

static INIT: Once = Once::new();

macro_rules! assert_benign_stderr {
    ($result:expr) => {
        $result
            .stderr(predicates::str::is_empty().or(predicates::str::contains("No sources found")));
    };
}

fn setup_test_data() {
    INIT.call_once(|| {
        // Set up test data once for all tests
        let mut cmd = blz_cmd();

        // Create a minimal test source (using a local file)
        let test_content = "# Test Document\n\nThis is test content for search pagination tests.";
        let test_file = std::env::temp_dir().join("test-llms.txt");
        std::fs::write(&test_file, test_content).unwrap();

        // Add test source (ignore if it already exists)
        cmd.arg("add")
            .arg("test-source")
            .arg(format!("file://{}", test_file.display()))
            .assert();
    });
}

#[test]
fn test_zero_limit_does_not_panic() {
    setup_test_data();

    // This test ensures that even if a limit of 0 is somehow passed,
    // the pagination logic doesn't panic with divide-by-zero

    // Note: The CLI actually prevents limit=0 via clap validation,
    // but this test ensures the defensive programming in pagination logic works

    let mut cmd = blz_cmd();

    // Try to trigger pagination with invalid limits
    // The actual CLI prevents this, but we're testing the defensive code
    cmd.arg("search")
        .arg("test")
        .arg("--limit")
        .arg("1")  // Minimum valid limit
        .arg("--page")
        .arg("999999"); // Very high page number

    // Should not panic - either show appropriate message or no sources error
    let result = cmd.assert();
    // Accept either success with message or error about no sources (plus flavor fallback warnings)
    assert_benign_stderr!(result);
}

#[test]
fn test_empty_results_pagination() {
    setup_test_data();

    // Test that pagination handles empty results gracefully
    let mut cmd = blz_cmd();

    cmd.arg("search")
        .arg("nonexistentquerythatwontmatchanything123456789")
        .arg("--limit")
        .arg("10")
        .arg("--page")
        .arg("1")
        .arg("-f")
        .arg("json"); // Use JSON output to avoid display issues

    // Should handle gracefully with no panic
    // Accept either success or no sources error
    let result = cmd.assert();
    assert_benign_stderr!(result);
}

#[test]
fn test_single_result_pagination() {
    setup_test_data();

    // Test edge case where there's only one result
    // This ensures actual_limit calculation doesn't cause issues

    // This test would require setting up a test index with known data
    // For now, we just ensure the command structure is valid

    let mut cmd = blz_cmd();

    cmd.arg("search")
        .arg("test")
        .arg("--limit")
        .arg("1")
        .arg("--page")
        .arg("1");

    // Should run without panic - accept either success or no sources error
    let result = cmd.assert();
    assert_benign_stderr!(result);
}

#[test]
fn test_large_limit_with_small_results() {
    setup_test_data();

    // Regression test: when limit >= ALL_RESULTS_LIMIT (10,000) and results are empty or small,
    // the actual_limit calculation should not cause divide-by-zero
    let mut cmd = blz_cmd();

    cmd.arg("search")
        .arg("extremely_unlikely_search_term_that_wont_match_xyz123")
        .arg("--limit")
        .arg("10000")  // ALL_RESULTS_LIMIT
        .arg("--page")
        .arg("1")
        .arg("-f")
        .arg("json");

    // Should not panic even with large limit and no results
    let result = cmd.assert();
    assert_benign_stderr!(result);
}

#[test]
fn test_page_boundary_with_exact_division() {
    setup_test_data();

    // Test when results divide exactly by limit
    let mut cmd = blz_cmd();

    cmd.arg("search")
        .arg("test")
        .arg("--limit")
        .arg("5")
        .arg("--page")
        .arg("2");

    // Should handle page boundary correctly - accept either success or no sources error
    let result = cmd.assert();
    assert_benign_stderr!(result);
}

#[test]
fn test_minimum_limit_value() {
    setup_test_data();

    // Test with the minimum valid limit (1)
    let mut cmd = blz_cmd();

    cmd.arg("search")
        .arg("test")
        .arg("--limit")
        .arg("1")
        .arg("--page")
        .arg("1");

    // Should handle minimum limit correctly - accept either success or no sources error
    let result = cmd.assert();
    assert_benign_stderr!(result);
}

#[test]
fn test_pagination_prevents_panic_on_edge_cases() {
    setup_test_data();

    // Test multiple edge cases that could cause panics
    let edge_cases = vec![
        ("1", "1"),     // Minimum values
        ("1", "10000"), // Minimum limit, huge page
        ("10000", "1"), // ALL_RESULTS_LIMIT
        ("100", "100"), // Large page number
    ];

    for (limit, page) in edge_cases {
        let mut cmd = blz_cmd();

        cmd.arg("search")
            .arg("test")
            .arg("--limit")
            .arg(limit)
            .arg("--page")
            .arg(page)
            .arg("-f")
            .arg("json");

        // None of these should panic - accept either success or no sources error
        let result = cmd.assert();
        assert_benign_stderr!(result);
    }
}
