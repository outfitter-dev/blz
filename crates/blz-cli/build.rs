//! Build script for blz-cli
//!
//! This script handles build-time configuration and setup,
//! including shell completion generation preparation.

use std::env;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun this script if the CLI structure changes
    println!("cargo:rerun-if-changed=src/main.rs");

    // Set up post-install hook notification
    if let Ok(profile) = env::var("PROFILE") {
        if profile == "release" {
            println!(
                "cargo:warning=After installing, run: blz completions fish > ~/.config/fish/completions/blz.fish"
            );
        }
    }

    // Create a marker file for version tracking
    if let Ok(out_dir) = env::var("OUT_DIR") {
        let version = env!("CARGO_PKG_VERSION");
        std::fs::write(Path::new(&out_dir).join("version.txt"), version).ok();
    }
}
