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
use clap::Args;

use crate::output::OutputFormat;
use crate::utils::cli_args::FormatArg;

/// Arguments for `blz check` (validate sources)
#[derive(Args, Clone, Debug)]
pub struct CheckArgs {
    /// Source to validate (validates all if not specified)
    pub alias: Option<String>,

    /// Validate all sources
    #[arg(long)]
    pub all: bool,

    /// Output format
    #[command(flatten)]
    pub format: FormatArg,
}

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
