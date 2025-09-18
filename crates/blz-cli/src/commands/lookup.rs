//! Lookup command implementation for searching registries

use anyhow::Result;
use blz_core::{Fetcher, PerformanceMetrics, Registry};
use colored::Colorize;
use dialoguer::{Input, Select};
use std::io::IsTerminal;

use crate::commands::add_source;
use crate::output::OutputFormat;
use crate::utils::validation::validate_alias;

/// Execute the lookup command to search registries
pub async fn execute(
    query: &str,
    metrics: PerformanceMetrics,
    quiet: bool,
    format: OutputFormat,
) -> Result<()> {
    let registry = Registry::new();

    if matches!(format, OutputFormat::Text) && !quiet {
        println!("Searching registries...");
    }
    let results = registry.search(query);

    if results.is_empty() {
        if matches!(format, OutputFormat::Text) && !quiet {
            println!("No matches found for '{query}'");
        }
        if matches!(format, OutputFormat::Json) {
            println!("[]");
        }
        return Ok(());
    }

    if matches!(format, OutputFormat::Text) && !quiet {
        display_results_with_health(&results).await?;
    }

    // Non-interactive JSON output for agents
    if !matches!(format, OutputFormat::Text) {
        let fetcher = Fetcher::new()?;
        let mut out = Vec::new();
        for r in &results {
            let mut head = serde_json::json!({});
            if let Ok(meta) = fetcher.head_metadata(&r.entry.llms_url).await {
                head = serde_json::json!({
                    "status": meta.status,
                    "contentLength": meta.content_length,
                    "etag": meta.etag,
                    "lastModified": meta.last_modified,
                });
            }
            let obj = serde_json::json!({
                "name": r.entry.name,
                "slug": r.entry.slug,
                "aliases": r.entry.aliases,
                "description": r.entry.description,
                "llmsUrl": r.entry.llms_url,
                "score": r.score,
                "matchField": r.match_field,
                "head": head,
            });
            out.push(obj);
        }
        if matches!(format, OutputFormat::Json) {
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            for o in out {
                println!("{}", serde_json::to_string(&o)?);
            }
        }
        return Ok(());
    }

    // Try interactive selection
    let Some(selected_entry) = try_interactive_selection(&results).ok() else {
        // Not interactive, show instructions
        if matches!(format, OutputFormat::Text) && !quiet {
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

    if matches!(format, OutputFormat::Text) && !quiet {
        println!(
            "Adding {} from {}...",
            final_alias.green(),
            selected_entry.llms_url.bright_black()
        );
    }

    add_source(final_alias, &selected_entry.llms_url, false, metrics).await
}

async fn display_results_with_health(
    results: &[blz_core::registry::RegistrySearchResult],
) -> Result<()> {
    println!(
        "Found {} match{}:\n",
        results.len(),
        if results.len() == 1 { "" } else { "es" }
    );

    let fetcher = Fetcher::new()?;
    for (i, result) in results.iter().enumerate() {
        let mut health = String::new();
        if let Ok(meta) = fetcher.head_metadata(&result.entry.llms_url).await {
            let ok = (200..300).contains(&i32::from(meta.status));
            let size = meta
                .content_length
                .map_or_else(|| "unknown size".to_string(), |n| format!("{n} bytes"));
            let status = if ok { "OK" } else { "ERR" };
            health = format!(" [{status} • {size}]");
        }

        println!("{}. {}{}", i + 1, result.entry, health.bright_black());
        println!("   {}\n", result.entry.llms_url.bright_black());
    }
    Ok(())
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
