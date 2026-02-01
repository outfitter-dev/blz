//! CPU profiling utilities for flamegraph generation.
//!
//! This module provides utilities for generating CPU flamegraphs using pprof.
//! All functionality is gated behind the `flamegraph` feature flag.

#[cfg(feature = "flamegraph")]
use crate::cli::Cli;

#[cfg(feature = "flamegraph")]
use blz_core::profiling::{start_profiling, stop_profiling_and_report};

/// Start CPU profiling if the `--flamegraph` flag was passed.
///
/// Returns the profiler guard that must be passed to [`stop_flamegraph_if_started`]
/// to generate the flamegraph.
#[cfg(feature = "flamegraph")]
pub fn start_flamegraph_if_requested(cli: &Cli) -> Option<pprof::ProfilerGuard<'static>> {
    if cli.flamegraph {
        match start_profiling() {
            Ok(guard) => {
                println!("CPU profiling started - flamegraph will be generated");
                Some(guard)
            },
            Err(e) => {
                eprintln!("Failed to start profiling: {e}");
                None
            },
        }
    } else {
        None
    }
}

/// Stop profiling and generate the flamegraph if profiling was started.
#[cfg(feature = "flamegraph")]
pub fn stop_flamegraph_if_started(guard: Option<pprof::ProfilerGuard<'static>>) {
    if let Some(guard) = guard {
        if let Err(e) = stop_profiling_and_report(&guard) {
            eprintln!("Failed to generate flamegraph: {e}");
        }
    }
}
