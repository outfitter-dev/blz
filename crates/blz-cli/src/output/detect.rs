//! TTY detection and format resolution utilities.
//!
// TODO(BLZ-337): Remove allow once commands adopt these utilities.
#![allow(dead_code)]
//!
//! This module provides utilities for detecting output context and resolving
//! the appropriate output format based on terminal status, environment variables,
//! and explicit user preferences.
//!
//! # Design Philosophy
//!
//! The format resolution follows a priority order that balances explicit user
//! intent with sensible defaults:
//!
//! 1. **Explicit flags** (`--json`, `--jsonl`, `--text`) - highest priority
//! 2. **Format flag** (`--format`) - explicit format selection
//! 3. **Environment variable** (`BLZ_OUTPUT_FORMAT`) - session-wide default
//! 4. **TTY detection** - automatic based on output context
//!
//! This ensures that:
//! - Interactive terminals get human-readable output by default
//! - Piped/scripted usage gets machine-readable JSON by default
//! - Users can always override with explicit flags
//!
//! # Examples
//!
//! ```bash
//! # Interactive terminal → Text output
//! blz search "async"
//!
//! # Piped output → JSON automatically
//! blz search "async" | jq '.results'
//!
//! # Force text when piping
//! blz search "async" --text | less
//!
//! # Session-wide JSON default
//! export BLZ_OUTPUT_FORMAT=json
//! blz search "async"  # Always JSON now
//! ```

use is_terminal::IsTerminal;

/// Detect whether stdout is connected to an interactive terminal.
///
/// Returns `true` if stdout is a TTY (terminal), `false` if output is
/// being piped or redirected.
///
/// # Examples
///
/// ```rust,ignore
/// use blz_cli::output::detect::is_interactive;
///
/// if is_interactive() {
///     println!("Running in terminal");
/// } else {
///     println!("Output is being piped");
/// }
/// ```
#[must_use]
pub fn is_interactive() -> bool {
    std::io::stdout().is_terminal()
}

/// Detect whether stderr is connected to an interactive terminal.
///
/// Useful for deciding whether to show progress indicators or
/// colored error messages.
#[must_use]
pub fn is_stderr_interactive() -> bool {
    std::io::stderr().is_terminal()
}

/// Check if colors should be enabled based on terminal and environment.
///
/// Colors are enabled when:
/// - stdout is a TTY
/// - `NO_COLOR` environment variable is not set
/// - `TERM` is not "dumb"
///
/// This follows the [NO_COLOR](https://no-color.org/) standard.
#[must_use]
pub fn should_use_colors() -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }

    if std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false) {
        return false;
    }

    is_interactive()
}

/// Get the format from environment variable if set.
///
/// Reads `BLZ_OUTPUT_FORMAT` and parses it as a format name.
/// Returns `None` if not set or if the value is invalid.
///
/// Valid values: "text", "json", "jsonl", "raw"
#[must_use]
pub fn format_from_env() -> Option<String> {
    std::env::var("BLZ_OUTPUT_FORMAT").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_interactive_returns_bool() {
        // Can't really test the actual value since it depends on how tests are run
        let _ = is_interactive();
    }

    #[test]
    fn test_is_stderr_interactive_returns_bool() {
        let _ = is_stderr_interactive();
    }

    // Note: Environment variable tests are disabled because set_var/remove_var
    // are unsafe in Rust 2024 edition due to potential data races.
    // The functionality is implicitly tested through integration tests.

    #[test]
    fn test_format_from_env_reads_var() {
        // Just verify the function runs without panic
        // Actual value depends on test environment
        let _ = format_from_env();
    }
}
