//! Output format argument groups for CLI commands.
//!
//! This module provides reusable output format arguments that can be composed
//! across multiple commands using clap's `#[command(flatten)]` attribute.
//!
//! # Examples
//!
//! ```bash
//! blz search "async" --format json
//! blz search "async" --json        # Shorthand
//! blz list --format text
//! ```

use clap::{Args, ValueEnum};
use is_terminal::IsTerminal;
use serde::{Deserialize, Serialize};

/// Output format for CLI results.
///
/// Determines how command output is formatted and displayed.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Human-readable formatted text (default for terminals).
    #[default]
    Text,
    /// JSON format for machine consumption (default for pipes).
    Json,
    /// JSON Lines format (one JSON object per line).
    Jsonl,
    /// Raw content without any formatting.
    Raw,
}

impl OutputFormat {
    /// Check if this format is machine-readable (JSON or JSONL).
    #[must_use]
    pub const fn is_machine_readable(self) -> bool {
        matches!(self, Self::Json | Self::Jsonl)
    }

    /// Check if this format is human-readable (Text).
    #[must_use]
    pub const fn is_human_readable(self) -> bool {
        matches!(self, Self::Text)
    }

    /// Detect the best format based on terminal status.
    ///
    /// Returns `Text` for interactive terminals, `Json` for pipes/redirects.
    #[must_use]
    pub fn detect() -> Self {
        if std::io::stdout().is_terminal() {
            Self::Text
        } else {
            Self::Json
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Json => write!(f, "json"),
            Self::Jsonl => write!(f, "jsonl"),
            Self::Raw => write!(f, "raw"),
        }
    }
}

/// Shared output format arguments for commands that produce formatted output.
///
/// This group provides consistent format selection across commands with
/// automatic TTY detection for sensible defaults.
///
/// # Default Behavior
///
/// When no format is explicitly specified:
/// - Interactive terminal: `Text` (human-readable)
/// - Piped/redirected output: `Json` (machine-readable)
///
/// # Usage
///
/// Flatten into command structs:
///
/// ```ignore
/// #[derive(Args)]
/// struct SearchArgs {
///     #[command(flatten)]
///     output: OutputArgs,
///     // ... other args
/// }
/// ```
///
/// Then resolve to an `OutputFormat`:
///
/// ```ignore
/// let format = args.output.resolve();
/// ```
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct OutputArgs {
    /// Output format (text, json, jsonl, raw).
    ///
    /// Defaults to text for terminals, json for pipes.
    #[arg(
        short = 'f',
        long = "format",
        value_enum,
        env = "BLZ_OUTPUT_FORMAT",
        display_order = 44
    )]
    pub format: Option<OutputFormat>,

    /// Output as JSON (shorthand for --format json).
    #[arg(long, conflicts_with = "format", display_order = 40)]
    pub json: bool,

    /// Output as JSON Lines (shorthand for --format jsonl).
    #[arg(long, conflicts_with_all = ["format", "json"], display_order = 41)]
    pub jsonl: bool,

    /// Output as plain text (shorthand for --format text).
    #[arg(long, conflicts_with_all = ["format", "json", "jsonl"], display_order = 42)]
    pub text: bool,
}

impl OutputArgs {
    /// Create output args with a specific format.
    #[must_use]
    pub const fn with_format(format: OutputFormat) -> Self {
        Self {
            format: Some(format),
            json: false,
            jsonl: false,
            text: false,
        }
    }

    /// Resolve the output arguments to a concrete format.
    ///
    /// Priority order:
    /// 1. Shorthand flags (--json, --jsonl, --text)
    /// 2. Explicit --format flag
    /// 3. Automatic TTY detection
    #[must_use]
    pub fn resolve(&self) -> OutputFormat {
        // Shorthand flags take priority
        if self.json {
            return OutputFormat::Json;
        }
        if self.jsonl {
            return OutputFormat::Jsonl;
        }
        if self.text {
            return OutputFormat::Text;
        }

        // Explicit format flag
        if let Some(format) = self.format {
            return format;
        }

        // Automatic TTY detection
        OutputFormat::detect()
    }

    /// Check if any format was explicitly specified.
    #[must_use]
    pub const fn is_explicit(&self) -> bool {
        self.format.is_some() || self.json || self.jsonl || self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod output_format {
        use super::*;

        #[test]
        fn test_default_is_text() {
            assert_eq!(OutputFormat::default(), OutputFormat::Text);
        }

        #[test]
        fn test_is_machine_readable() {
            assert!(OutputFormat::Json.is_machine_readable());
            assert!(OutputFormat::Jsonl.is_machine_readable());
            assert!(!OutputFormat::Text.is_machine_readable());
            assert!(!OutputFormat::Raw.is_machine_readable());
        }

        #[test]
        fn test_is_human_readable() {
            assert!(OutputFormat::Text.is_human_readable());
            assert!(!OutputFormat::Json.is_human_readable());
            assert!(!OutputFormat::Jsonl.is_human_readable());
            assert!(!OutputFormat::Raw.is_human_readable());
        }

        #[test]
        fn test_display() {
            assert_eq!(OutputFormat::Text.to_string(), "text");
            assert_eq!(OutputFormat::Json.to_string(), "json");
            assert_eq!(OutputFormat::Jsonl.to_string(), "jsonl");
            assert_eq!(OutputFormat::Raw.to_string(), "raw");
        }
    }

    mod output_args {
        use super::*;

        #[test]
        fn test_default() {
            let args = OutputArgs::default();
            assert_eq!(args.format, None);
            assert!(!args.json);
            assert!(!args.jsonl);
            assert!(!args.text);
            assert!(!args.is_explicit());
        }

        #[test]
        fn test_with_format() {
            let args = OutputArgs::with_format(OutputFormat::Json);
            assert_eq!(args.format, Some(OutputFormat::Json));
            assert!(args.is_explicit());
        }

        #[test]
        fn test_resolve_json_flag() {
            let args = OutputArgs {
                format: None,
                json: true,
                jsonl: false,
                text: false,
            };
            assert_eq!(args.resolve(), OutputFormat::Json);
        }

        #[test]
        fn test_resolve_jsonl_flag() {
            let args = OutputArgs {
                format: None,
                json: false,
                jsonl: true,
                text: false,
            };
            assert_eq!(args.resolve(), OutputFormat::Jsonl);
        }

        #[test]
        fn test_resolve_text_flag() {
            let args = OutputArgs {
                format: None,
                json: false,
                jsonl: false,
                text: true,
            };
            assert_eq!(args.resolve(), OutputFormat::Text);
        }

        #[test]
        fn test_resolve_explicit_format() {
            let args = OutputArgs {
                format: Some(OutputFormat::Raw),
                json: false,
                jsonl: false,
                text: false,
            };
            assert_eq!(args.resolve(), OutputFormat::Raw);
        }

        #[test]
        fn test_json_flag_takes_precedence_over_format() {
            // This shouldn't happen due to conflicts_with, but test the logic
            let args = OutputArgs {
                format: Some(OutputFormat::Text),
                json: true,
                jsonl: false,
                text: false,
            };
            // Shorthand flags take priority
            assert_eq!(args.resolve(), OutputFormat::Json);
        }
    }
}
