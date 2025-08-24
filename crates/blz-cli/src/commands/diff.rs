//! Diff command implementation

use anyhow::Result;

/// Show diffs for a source
pub async fn show(_alias: &str, _since: Option<&str>) -> Result<()> {
    println!("Diff functionality not yet implemented");
    Ok(())
}
