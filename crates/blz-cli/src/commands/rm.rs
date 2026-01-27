//! Rm command implementation - remove a source
//!
//! This module provides the `blz rm` command as an alias for `remove`.
//! The "rm" name follows Unix convention.
//!
//! # Examples
//!
//! ```bash
//! blz rm bun                      # Remove a source
//! blz rm bun --yes                # Skip confirmation
//! ```

use anyhow::Result;

/// Execute the rm command to remove a source
///
/// This command removes a documentation source from the local cache.
/// It delegates to the internal remove implementation.
///
/// # Arguments
///
/// * `aliases` - Source alias to remove (wrapped in Vec for internal API compatibility)
/// * `yes` - Skip confirmation prompts
///
/// # Errors
///
/// Returns an error if any source removal fails.
pub async fn execute(aliases: Vec<String>, yes: bool) -> Result<()> {
    for alias in aliases {
        super::remove::execute(&alias, yes, false).await?;
    }
    Ok(())
}
