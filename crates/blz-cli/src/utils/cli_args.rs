use clap::Args;

use crate::output::OutputFormat;

/// Shared clap argument for commands that accept an output format.
#[derive(Args, Clone, Debug)]
pub struct FormatArg {
    /// Output format
    #[arg(
        short = 'f',
        long = "format",
        alias = "output",
        value_enum,
        default_value = "text",
        env = "BLZ_OUTPUT_FORMAT"
    )]
    pub format: OutputFormat,
}
