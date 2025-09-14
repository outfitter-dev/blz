//! Remove command implementation

use anyhow::Result;
use blz_core::Storage;
use colored::Colorize;
use std::fs;

/// Execute the remove command to delete a source
pub async fn execute(alias: &str) -> Result<()> {
    let storage = Storage::new()?;

    // Resolve metadata alias to canonical if needed
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .unwrap_or_else(|| alias.to_string());

    if !storage.exists(&canonical) {
        println!("Source '{alias}' not found");
        return Ok(());
    }

    display_removal_info(&storage, &canonical);

    // Remove the entire source directory and all its contents
    let source_dir = storage.tool_dir(&canonical)?;

    match fs::remove_dir_all(&source_dir) {
        Ok(()) => {
            println!(
                "✓ Successfully removed source '{}' and all associated files",
                canonical.green()
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

fn display_removal_info(storage: &Storage, alias: &str) {
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
}
