//! Progress display utilities

use indicatif::{ProgressBar, ProgressStyle};

#[allow(dead_code)]
pub struct ProgressDisplay;

impl ProgressDisplay {
    /// Create a new spinner with the given message
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
