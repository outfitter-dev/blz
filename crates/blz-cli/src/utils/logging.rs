//! Logging initialization and configuration.
//!
//! This module handles setting up the tracing subscriber and color control
//! based on CLI flags and environment variables.

use anyhow::Result;
use colored::control as color_control;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::cli::{Cli, Commands};
use crate::output::OutputFormat;

/// Initialize the logging subsystem based on CLI flags.
///
/// Sets the log level based on verbosity flags and suppresses info logs
/// when machine-readable output (JSON/JSONL) is requested.
///
/// # Errors
///
/// Returns an error if the global tracing subscriber cannot be set.
pub fn initialize_logging(cli: &Cli) -> Result<()> {
    // Base level from global flags
    let mut level = if cli.verbose || cli.debug {
        Level::DEBUG
    } else if cli.quiet {
        Level::ERROR
    } else {
        Level::WARN
    };

    // If the selected command is emitting machine-readable output, suppress info logs
    // to keep stdout/stderr clean unless verbose/debug was explicitly requested.
    let mut machine_output = false;
    if !(cli.verbose || cli.debug) {
        #[allow(deprecated)]
        let command_format = match &cli.command {
            Some(
                Commands::List { format, .. }
                | Commands::Stats { format, .. }
                | Commands::History { format, .. }
                | Commands::Lookup { format, .. }
                | Commands::Get { format, .. }
                | Commands::Info { format, .. }
                | Commands::Completions { format, .. },
            ) => Some(format.resolve(cli.quiet)),
            Some(Commands::Search(args)) => Some(args.format.resolve(cli.quiet)),
            Some(Commands::Find(args)) => Some(args.format.resolve(cli.quiet)),
            Some(Commands::Toc(args)) => Some(args.format.resolve(cli.quiet)),
            Some(Commands::Query(args)) => Some(args.format.resolve(cli.quiet)),
            Some(Commands::Map(args)) => Some(args.format.resolve(cli.quiet)),
            Some(Commands::Check(args)) => Some(args.format.resolve(cli.quiet)),
            _ => None,
        };

        if let Some(fmt) = command_format {
            if matches!(fmt, OutputFormat::Json | OutputFormat::Jsonl) {
                level = Level::ERROR;
                machine_output = true;
            }
        }
    }

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // Color control: disable when requested, NO_COLOR is set, or when emitting machine output
    let env_no_color = std::env::var("NO_COLOR").ok().is_some();
    if cli.no_color || env_no_color || machine_output {
        color_control::set_override(false);
    }
    Ok(())
}
