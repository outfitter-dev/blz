use std::sync::atomic::{AtomicBool, Ordering};

use clap::Args;
use is_terminal::IsTerminal;

use crate::output::OutputFormat;

static OUTPUT_DEPRECATED_WARNED: AtomicBool = AtomicBool::new(false);

/// Shared clap argument for commands that accept an output format.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct FormatArg {
    /// Canonical output format flag (`--format` / `-f`)
    #[arg(
        short = 'f',
        long = "format",
        value_enum,
        env = "BLZ_OUTPUT_FORMAT",
        conflicts_with_all = ["json", "jsonl", "text", "raw"]
    )]
    pub format: Option<OutputFormat>,

    /// Convenience flag for JSON output (equivalent to --format json)
    #[arg(long, conflicts_with_all = ["format", "jsonl", "text", "raw"])]
    pub json: bool,

    /// Convenience flag for JSONL output (equivalent to --format jsonl)
    #[arg(long, conflicts_with_all = ["format", "json", "text", "raw"])]
    pub jsonl: bool,

    /// Convenience flag for text output (equivalent to --format text)
    #[arg(long, conflicts_with_all = ["format", "json", "jsonl", "raw"])]
    pub text: bool,

    /// Convenience flag for raw output (equivalent to --format raw)
    #[arg(long, conflicts_with_all = ["format", "json", "jsonl", "text"])]
    pub raw: bool,

    /// Hidden deprecated alias that maps to `--format`
    #[arg(long = "output", short = 'o', hide = true, value_enum)]
    pub deprecated_output: Option<OutputFormat>,
}

impl FormatArg {
    /// Returns the effective output format, preferring the canonical flag and falling back to
    /// the deprecated alias when necessary.
    ///
    /// # Default Behavior
    ///
    /// When no format is explicitly specified:
    /// - Interactive terminal: Returns `OutputFormat::Text` (human-readable)
    /// - Piped/redirected output: Returns `OutputFormat::Json` (machine-readable)
    ///
    /// This automatic switching optimizes for the most common use cases while allowing
    /// explicit override with `--format text` when needed.
    #[must_use]
    pub fn resolve(&self, quiet: bool) -> OutputFormat {
        // If shortcut flags are set, use them
        if self.json {
            return OutputFormat::Json;
        }
        if self.jsonl {
            return OutputFormat::Jsonl;
        }
        if self.text {
            return OutputFormat::Text;
        }
        if self.raw {
            return OutputFormat::Raw;
        }

        if let Some(deprecated) = self.deprecated_output {
            emit_deprecated_warning(quiet);
            if self.format.is_none() {
                return deprecated;
            }
        }

        // If format is explicitly set, use it
        if let Some(format) = self.format {
            return format;
        }

        // If output is piped (not a terminal), default to JSON for machine readability
        if std::io::stdout().is_terminal() {
            OutputFormat::Text
        } else {
            OutputFormat::Json
        }
    }
}

fn emit_deprecated_warning(quiet: bool) {
    if quiet || std::env::var_os("BLZ_SUPPRESS_DEPRECATIONS").is_some() {
        return;
    }

    if OUTPUT_DEPRECATED_WARNED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        eprintln!(
            "warning: --output/-o is deprecated; use --format/-f. This alias will be removed in a future release."
        );
    }
}

#[cfg(test)]
#[allow(
    unsafe_code,
    clippy::clone_on_copy,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;
    use crate::utils::test_support;
    use std::ffi::OsString;

    struct EnvGuard {
        key: &'static str,
        original: Option<OsString>,
    }

    impl EnvGuard {
        fn new(key: &'static str) -> Self {
            Self {
                key,
                original: std::env::var_os(key),
            }
        }

        fn set<S: AsRef<std::ffi::OsStr>>(&self, value: S) {
            // SAFETY: tests serialise environment access via env_mutex(), ensuring these calls are
            // not concurrent. Rust 1.89 treats env mutations as unsafe for multi-threaded code.
            unsafe {
                std::env::set_var(self.key, value);
            }
        }

        fn remove(&self) {
            unsafe {
                std::env::remove_var(self.key);
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = self.original.clone() {
                unsafe {
                    std::env::set_var(self.key, value);
                }
            } else {
                unsafe {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    fn reset_warning_flag() {
        OUTPUT_DEPRECATED_WARNED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn resolve_prefers_canonical_flag() {
        let _env_guard = test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");

        reset_warning_flag();

        let args = FormatArg {
            format: Some(OutputFormat::Jsonl),
            json: false,
            jsonl: false,
            text: false,
            raw: false,
            deprecated_output: None,
        };

        assert_eq!(args.resolve(false), OutputFormat::Jsonl);
        assert!(!OUTPUT_DEPRECATED_WARNED.load(Ordering::SeqCst));
    }

    #[test]
    fn deprecated_alias_sets_warning_flag_once() {
        let _env_guard = test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");

        reset_warning_flag();
        let suppress_guard = EnvGuard::new("BLZ_SUPPRESS_DEPRECATIONS");
        suppress_guard.remove();

        let args = FormatArg {
            format: None,
            json: false,
            jsonl: false,
            text: false,
            raw: false,
            deprecated_output: Some(OutputFormat::Json),
        };

        assert_eq!(args.resolve(false), OutputFormat::Json);
        assert!(OUTPUT_DEPRECATED_WARNED.load(Ordering::SeqCst));

        // Subsequent invocations should not toggle the flag again.
        assert_eq!(args.resolve(false), OutputFormat::Json);
        assert!(OUTPUT_DEPRECATED_WARNED.load(Ordering::SeqCst));
    }

    #[test]
    fn deprecated_alias_warning_suppressed_when_quiet_or_env_set() {
        let _env_guard = test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");

        reset_warning_flag();
        let suppress_guard = EnvGuard::new("BLZ_SUPPRESS_DEPRECATIONS");
        suppress_guard.set("1");

        let args = FormatArg {
            format: None,
            json: false,
            jsonl: false,
            text: false,
            raw: false,
            deprecated_output: Some(OutputFormat::Json),
        };

        assert_eq!(args.resolve(false), OutputFormat::Json);
        assert!(!OUTPUT_DEPRECATED_WARNED.load(Ordering::SeqCst));

        suppress_guard.remove();
        reset_warning_flag();

        // Quiet mode should also suppress the warning.
        assert_eq!(args.resolve(true), OutputFormat::Json);
        assert!(!OUTPUT_DEPRECATED_WARNED.load(Ordering::SeqCst));
    }

    #[test]
    fn json_flag_returns_json_format() {
        let _env_guard = test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");

        let args = FormatArg {
            format: None,
            json: true,
            jsonl: false,
            text: false,
            raw: false,
            deprecated_output: None,
        };

        assert_eq!(args.resolve(false), OutputFormat::Json);
    }

    #[test]
    fn json_flag_takes_precedence_over_pipe_detection() {
        let _env_guard = test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");

        // Even with format set to None (which would trigger pipe detection),
        // json flag should take precedence
        let args = FormatArg {
            format: None,
            json: true,
            jsonl: false,
            text: false,
            raw: false,
            deprecated_output: None,
        };

        assert_eq!(args.resolve(false), OutputFormat::Json);
    }

    #[test]
    fn jsonl_flag_returns_jsonl_format() {
        let _env_guard = test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");

        let args = FormatArg {
            format: None,
            json: false,
            jsonl: true,
            text: false,
            raw: false,
            deprecated_output: None,
        };

        assert_eq!(args.resolve(false), OutputFormat::Jsonl);
    }

    #[test]
    fn text_flag_returns_text_format() {
        let _env_guard = test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");

        let args = FormatArg {
            format: None,
            json: false,
            jsonl: false,
            text: true,
            raw: false,
            deprecated_output: None,
        };

        assert_eq!(args.resolve(false), OutputFormat::Text);
    }

    #[test]
    fn raw_flag_returns_raw_format() {
        let _env_guard = test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");

        let args = FormatArg {
            format: None,
            json: false,
            jsonl: false,
            text: false,
            raw: true,
            deprecated_output: None,
        };

        assert_eq!(args.resolve(false), OutputFormat::Raw);
    }

    #[test]
    fn explicit_format_takes_precedence_over_shortcuts() {
        let _env_guard = test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");

        // When --format is set, it should take precedence over shortcut flags
        let args = FormatArg {
            format: Some(OutputFormat::Jsonl),
            json: false,
            jsonl: false,
            text: false,
            raw: false,
            deprecated_output: None,
        };

        assert_eq!(args.resolve(false), OutputFormat::Jsonl);
    }

    #[test]
    fn shortcuts_take_precedence_over_deprecated_output() {
        let _env_guard = test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");

        reset_warning_flag();

        let args = FormatArg {
            format: None,
            json: true,
            jsonl: false,
            text: false,
            raw: false,
            deprecated_output: Some(OutputFormat::Text),
        };

        // Shortcut should win, no deprecation warning should be emitted
        assert_eq!(args.resolve(false), OutputFormat::Json);
        assert!(!OUTPUT_DEPRECATED_WARNED.load(Ordering::SeqCst));
    }
}
