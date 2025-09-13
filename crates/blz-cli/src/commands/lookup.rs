//! Lookup command implementation for searching registries

use anyhow::Result;
use blz_core::{PerformanceMetrics, Registry};
use colored::Colorize;
use dialoguer::{Input, Select};
use std::io::IsTerminal;

use crate::commands::add_source;
use crate::utils::validation::validate_alias;

/// Execute the lookup command to search registries
pub async fn execute(query: &str, metrics: PerformanceMetrics, quiet: bool) -> Result<()> {
    let registry = Registry::new();

    if !quiet {
        println!("Searching registries...");
    }
    let results = registry.search(query);

    if results.is_empty() {
        if !quiet {
            println!("No matches found for '{query}'");
        }
        return Ok(());
    }

    if !quiet {
        display_results(&results);
    }

    // Try interactive selection
    let Some(selected_entry) = try_interactive_selection(&results).ok() else {
        // Not interactive, show instructions
        if !quiet {
            display_manual_instructions(&results);
        }
        return Ok(());
    };

    // Prompt for alias
    let default_alias = selected_entry.slug.clone();
    let alias = try_interactive_alias_input(&default_alias).unwrap_or_else(|_| {
        if !quiet {
            println!("Using default alias: {}", default_alias.green());
        }
        default_alias.clone()
    });

    let final_alias = alias.trim();
    validate_alias(final_alias)?;

    if !quiet {
        println!(
            "Adding {} from {}...",
            final_alias.green(),
            selected_entry.llms_url.bright_black()
        );
    }

    add_source(final_alias, &selected_entry.llms_url, false, metrics).await
}

fn display_results(results: &[blz_core::registry::RegistrySearchResult]) {
    println!(
        "Found {} match{}:\n",
        results.len(),
        if results.len() == 1 { "" } else { "es" }
    );

    for (i, result) in results.iter().enumerate() {
        println!("{}. {}", i + 1, result.entry);
        println!("   {}\n", result.entry.llms_url.bright_black());
    }
}

fn display_manual_instructions(results: &[blz_core::registry::RegistrySearchResult]) {
    println!("To add any of these sources, use:");
    for (i, result) in results.iter().enumerate() {
        println!(
            "  {} blz add {} {}",
            format!("{}.", i + 1).bright_black(),
            result.entry.slug.green(),
            result.entry.llms_url.bright_black()
        );
    }
}

fn try_interactive_selection(
    results: &[blz_core::registry::RegistrySearchResult],
) -> Result<&blz_core::registry::RegistryEntry> {
    if !std::io::stderr().is_terminal() {
        return Err(anyhow::anyhow!("Not in interactive terminal"));
    }

    let display_items: Vec<String> = results
        .iter()
        .enumerate()
        .map(|(i, result)| format!("{}. {}", i + 1, result.entry))
        .collect();

    let selection = Select::new()
        .with_prompt("Select documentation to add (↑/↓ to navigate)")
        .items(&display_items)
        .interact()?;

    Ok(&results[selection].entry)
}

fn try_interactive_alias_input(default_alias: &str) -> Result<String> {
    if !std::io::stderr().is_terminal() {
        return Err(anyhow::anyhow!("Not in interactive terminal"));
    }

    let alias: String = Input::new()
        .with_prompt("Enter alias")
        .default(default_alias.to_string())
        .interact_text()?;

    if alias.trim().is_empty() {
        return Err(anyhow::anyhow!("Alias cannot be empty"));
    }

    Ok(alias)
}
