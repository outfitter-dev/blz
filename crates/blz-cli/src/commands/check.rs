//! Check command implementation - validate source integrity
//!
//! This module provides the `blz check` command for validating documentation
//! source integrity and availability.
//!
//! # Examples
//!
//! ```bash
//! blz check bun                  # Check single source
//! blz check --all                # Check all sources
//! blz check bun --json           # JSON output for scripting
//! ```

use anyhow::Result;

use crate::output::OutputFormat;

/// Execute the check command to validate sources
///
/// This command validates documentation source integrity and availability.
/// It delegates to the internal validate implementation.
///
/// # Arguments
///
/// * `alias` - Source to validate (validates all if not specified with --all)
/// * `all` - Validate all sources
/// * `format` - Output format (text, json, jsonl)
pub async fn execute(alias: Option<String>, all: bool, format: OutputFormat) -> Result<()> {
    super::validate::execute(alias, all, format).await
}
