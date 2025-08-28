//! Progress display utilities

use indicatif::{ProgressBar, ProgressStyle};

/// Progress display utilities for CLI operations
///
/// These methods are preserved for future use when implementing
/// download progress, long-running operations, and batch processing.
#[allow(dead_code)]
pub struct ProgressDisplay;

#[allow(dead_code)]
impl ProgressDisplay {
    /// Create a new spinner with the given message
    ///
    /// Use for operations with unknown duration.
    pub fn spinner(message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        pb.set_message(message.to_string());
        pb
    }

    /// Create a progress bar for downloads or operations with known size
    ///
    /// Use for operations where progress can be measured.
    pub fn bar(total: u64) -> ProgressBar {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("#>-"),
        );
        pb
    }
}
