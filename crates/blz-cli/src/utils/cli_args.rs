use std::sync::atomic::{AtomicBool, Ordering};

use clap::Args;

use crate::output::OutputFormat;

static OUTPUT_DEPRECATED_WARNED: AtomicBool = AtomicBool::new(false);

/// Shared clap argument for commands that accept an output format.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct FormatArg {
    /// Canonical output format flag (`--format` / `-f`)
    #[arg(short = 'f', long = "format", value_enum, env = "BLZ_OUTPUT_FORMAT")]
    pub format: Option<OutputFormat>,

    /// Hidden deprecated alias that maps to `--format`
    #[arg(long = "output", short = 'o', hide = true, value_enum)]
    pub deprecated_output: Option<OutputFormat>,
}

impl FormatArg {
    /// Returns the effective output format, preferring the canonical flag and falling back to
    /// the deprecated alias when necessary.
    #[must_use]
    pub fn resolve(&self, quiet: bool) -> OutputFormat {
        if let Some(deprecated) = self.deprecated_output {
            emit_deprecated_warning(quiet);
            if self.format.is_none() {
                return deprecated;
            }
        }

        self.format.unwrap_or(OutputFormat::Text)
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
