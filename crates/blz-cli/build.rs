//! Build script for blz-cli
//!
//! This script handles build-time configuration and setup,
//! including shell completion generation preparation.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun this script if the CLI structure changes
    println!("cargo:rerun-if-changed=src/main.rs");
    if let Err(err) = watch_command_help_files() {
        println!("cargo:warning=Failed to watch command help files: {err}");
    }

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
        fs::write(Path::new(&out_dir).join("version.txt"), version).ok();
    }
}

fn watch_command_help_files() -> std::io::Result<()> {
    let commands_dir = Path::new("src/commands");
    if !commands_dir.exists() {
        return Ok(());
    }

    // Watch the directory itself for structural changes
    println!("cargo:rerun-if-changed=src/commands");

    for entry in fs::read_dir(commands_dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_file() {
            let file_name = path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or_default();
            if file_name.ends_with(".help.md") {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }

    Ok(())
}
