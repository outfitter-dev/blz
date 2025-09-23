//! Remove command implementation

use anyhow::Result;
use blz_core::Storage;
use colored::Colorize;
use dialoguer::Confirm;
use std::fs;
use std::io::IsTerminal;

/// Execute the remove command to delete a source
pub async fn execute(alias: &str, auto_yes: bool, quiet: bool) -> Result<()> {
    let storage = Storage::new()?;

    // Resolve metadata alias to canonical if needed
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .unwrap_or_else(|| alias.to_string());

    if !storage.exists(&canonical) {
        if !quiet {
            println!("Source '{alias}' not found");
        }
        return Ok(());
    }

    display_removal_info(&storage, &canonical, quiet);

    // Treat auto-yes, explicit non-interactive env, or the absence of a TTY as approval.
    let force_non_interactive = std::env::var_os("BLZ_FORCE_NON_INTERACTIVE").is_some();
    let no_tty = !std::io::stdin().is_terminal();

    if !(auto_yes || force_non_interactive || no_tty) {
        let prompt = format!("Remove source '{canonical}' and all cached data?");
        let confirmed = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?;
        if !confirmed {
            if !quiet {
                println!("Removal cancelled");
            }
            return Ok(());
        }
    }

    // Remove the entire source directory and all its contents
    let source_dir = storage.tool_dir(&canonical)?;

    match fs::remove_dir_all(&source_dir) {
        Ok(()) => {
            if !quiet {
                println!(
                    "✓ Successfully removed source '{}' and all associated files",
                    canonical.green()
                );
            }
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

fn display_removal_info(storage: &Storage, alias: &str, quiet: bool) {
    if quiet {
        return;
    }
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
