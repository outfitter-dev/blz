//! Remove command implementation

use anyhow::Result;
use blz_core::Storage;
use colored::Colorize;
use std::fs;

/// Execute the remove command to delete a source
pub async fn execute(alias: &str) -> Result<()> {
    let storage = Storage::new()?;

    if !storage.exists(alias) {
        println!("Source '{alias}' not found");
        return Ok(());
    }

    display_removal_info(&storage, alias)?;

    // Remove the entire source directory and all its contents
    let source_dir = storage.tool_dir(alias)?;

    match fs::remove_dir_all(&source_dir) {
        Ok(()) => {
            println!(
                "✓ Successfully removed source '{}' and all associated files",
                alias.green()
            );
        },
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Failed to remove source directory '{}': {}",
                source_dir.display(),
                e
            ));
        },
    }

    Ok(())
}

fn display_removal_info(storage: &Storage, alias: &str) -> Result<()> {
    if let Ok(llms_json) = storage.load_llms_json(alias) {
        println!(
            "Removing source '{}' ({})",
            alias.red(),
            llms_json.source.url
        );
        println!("  {} lines", llms_json.line_index.total_lines);
        println!(
            "  Fetched: {}",
            llms_json.source.fetched_at.format("%Y-%m-%d %H:%M:%S")
        );
    }
    Ok(())
}
