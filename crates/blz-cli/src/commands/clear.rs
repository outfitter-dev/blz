//! Cache clearing command implementation

use anyhow::Result;
use blz_core::Storage;
use colored::Colorize;
use std::io::{self, Write};

/// Clears the entire cache directory
pub fn run(force: bool) -> Result<()> {
    let storage = Storage::new()?;

    // Check if there are any sources
    let sources = storage.list_sources();

    if sources.is_empty() {
        println!("{} Cache is already empty", "ℹ".blue());
        return Ok(());
    }

    // Show what will be deleted
    println!(
        "{} This will permanently delete all cached data for {} source(s):",
        "⚠".yellow(),
        sources.len()
    );
    for source in &sources {
        println!("  • {}", source);
    }
    println!();

    // Prompt for confirmation unless --force is used
    if !force {
        print!("Are you sure you want to continue? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let confirmed =
            input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes");

        if !confirmed {
            println!("{} Cancelled", "✗".red());
            return Ok(());
        }
    }

    // Clear the cache
    storage.clear_cache()?;

    println!("{} Cache cleared successfully", "✓".green());
    println!();
    println!("To re-add sources, use:");
    println!("  blz add <alias> <url>");

    Ok(())
}
