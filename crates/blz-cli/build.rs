//! Build script for blz-cli
//!
//! This script handles build-time configuration and setup,
//! including shell completion generation preparation.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() {
    // Tell Cargo to rerun this script if the CLI structure changes
    println!("cargo:rerun-if-changed=src/main.rs");
    if let Err(err) = watch_command_help_files() {
        println!("cargo:warning=Failed to watch command help files: {err}");
    }

    if let Err(err) = prepare_bundled_docs() {
        println!(
            "cargo:warning=Failed to stage bundled docs for embedding: {err}. Ensure docs/llms/blz/llms-full.txt or crates/blz-cli/bundled-docs/llms-full.txt exists."
        );
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

fn prepare_bundled_docs() -> io::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").map_err(|err| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("CARGO_MANIFEST_DIR not set: {err}"),
        )
    })?);
    let out_dir = PathBuf::from(env::var("OUT_DIR").map_err(|err| {
        io::Error::new(io::ErrorKind::NotFound, format!("OUT_DIR not set: {err}"))
    })?);
    let workspace_doc = manifest_dir.join("../../docs/llms/blz/llms-full.txt");
    let fallback_doc = manifest_dir.join("bundled-docs/llms-full.txt");

    let source = if workspace_doc.exists() {
        println!("cargo:rerun-if-changed={}", workspace_doc.display());

        if fallback_doc.exists() {
            if fs::read(&workspace_doc)? != fs::read(&fallback_doc)? {
                println!(
                    "cargo:warning=Bundled docs fallback at {} is out of sync with workspace copy. \
                     Please update the fallback to keep release artifacts consistent.",
                    fallback_doc.display()
                );
            }
        } else {
            println!(
                "cargo:warning=Missing fallback bundled docs at {}; using workspace copy.",
                fallback_doc.display()
            );
        }

        workspace_doc
    } else if fallback_doc.exists() {
        println!("cargo:rerun-if-changed={}", fallback_doc.display());
        fallback_doc
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Bundled docs not found at {} or {}",
                workspace_doc.display(),
                fallback_doc.display()
            ),
        ));
    };

    fs::create_dir_all(&out_dir)?;
    let bundled_path = out_dir.join("bundled_llms_full.txt");
    fs::copy(&source, &bundled_path)?;
    println!("cargo:rustc-env=BLZ_BUNDLED_DOC={}", bundled_path.display());

    Ok(())
}
