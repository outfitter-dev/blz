#![allow(missing_docs)]

mod common;

use common::blz_cmd;

#[test]
fn instruct_prints_curated_text_and_cli_docs() -> anyhow::Result<()> {
    let mut cmd = blz_cmd();
    let out = cmd
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
        s.contains("Need full command reference?"),
        "should point to docs command for full reference"
    );
    assert!(
        !s.contains("=== CLI Docs ==="),
        "should not append verbose CLI docs by default"
    );
    Ok(())
}
