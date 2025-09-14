#![allow(missing_docs)]

#[test]
fn instruct_prints_curated_text_and_cli_docs() -> anyhow::Result<()> {
    let out = assert_cmd::Command::cargo_bin("blz")?
        .arg("instruct")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out)?;
    assert!(
        s.contains("BLZ Agent Instructions"),
        "should include curated instructions"
    );
    assert!(
        s.contains("=== CLI Docs ==="),
        "should append CLI docs section"
    );
    assert!(
        s.contains("Subcommands"),
        "should include subcommands in docs"
    );
    Ok(())
}
