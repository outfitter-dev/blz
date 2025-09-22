#![allow(missing_docs)]

use std::str;

use anyhow::Result;
use assert_cmd::Command;

#[test]
fn deprecated_output_alias_warns_and_matches_format() -> Result<()> {
    let canonical_stdout = Command::cargo_bin("blz")?
        .env("BLZ_DISABLE_GUARD", "1")
        .args(["completions", "--format", "json", "--list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    for flag in ["--output", "-o"] {
        let assertion = Command::cargo_bin("blz")?
            .env("BLZ_DISABLE_GUARD", "1")
            .args(["completions", flag, "json", "--list"])
            .assert()
            .success();

        let output = assertion.get_output();
        assert_eq!(
            output.stdout, canonical_stdout,
            "{flag} should match --format output"
        );

        let stderr = str::from_utf8(&output.stderr)?;
        assert!(
            stderr.contains("warning: --output/-o is deprecated; use --format/-f. This alias will be removed in a future release."),
            "expected deprecation warning for {flag}"
        );
        assert_eq!(
            stderr
                .matches("warning: --output/-o is deprecated; use --format/-f. This alias will be removed in a future release.")
                .count(),
            1,
            "warning should be emitted exactly once for {flag}"
        );
    }

    Ok(())
}

#[test]
fn deprecated_output_respects_quiet_flag() -> Result<()> {
    let assertion = Command::cargo_bin("blz")?
        .env("BLZ_DISABLE_GUARD", "1")
        .args(["-q", "completions", "--output", "json", "--list"])
        .assert()
        .success();

    let stderr = std::str::from_utf8(&assertion.get_output().stderr)?;
    assert!(
        stderr.trim().is_empty(),
        "quiet mode should suppress warnings"
    );

    Ok(())
}
