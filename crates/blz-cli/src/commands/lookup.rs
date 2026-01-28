//! Lookup command implementation for searching registries

use anyhow::Result;
use blz_core::{Fetcher, PerformanceMetrics, Registry};
use colored::Colorize;
use inquire::{Select, Text};
use serde_json::json;
use std::fmt;
use std::io::IsTerminal;

use crate::commands::{AddRequest, DescriptorInput, add_source};
use crate::output::OutputFormat;
use crate::prompt::{NoteChannel, emit_registry_note};
use crate::utils::validation::validate_alias;

#[derive(Clone)]
struct SelectionItem {
    index: usize,
    label: String,
}

impl fmt::Display for SelectionItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

/// Handle the case when registry is disabled
fn handle_registry_disabled(format: OutputFormat, quiet: bool) -> Result<()> {
    if matches!(format, OutputFormat::Text) {
        if !quiet {
            println!("Registry lookup is coming soon.");
            println!(
                "In the meantime, search upstream docs for an llms-full.txt (or llms.txt) URL and add it manually:"
            );
            println!("  blz add <alias> <https://example.com/llms-full.txt>");
            println!("Coming soon: automatic registry search with health checks.");
        }
    } else {
        let payload = json!({
            "status": "coming_soon",
            "message": "Registry lookup is temporarily disabled while we finish the new catalog flow.",
            "nextSteps": [
                "Locate an llms-full.txt (or llms.txt) URL for the docs you need.",
                "Add it manually with: blz add <alias> <url>",
                "Agent-compatible registry search will return in an upcoming release."
            ]
        });

        match format {
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&payload)?),
            OutputFormat::Jsonl | OutputFormat::Raw => {
                println!("{}", serde_json::to_string(&payload)?);
            },
            OutputFormat::Text => unreachable!(),
        }
    }

    emit_registry_note(format, quiet, NoteChannel::Auto);
    Ok(())
}

/// Format and output results as JSON for non-text output modes
async fn output_results_json(
    results: &[blz_core::registry::RegistrySearchResult],
    format: OutputFormat,
    quiet: bool,
) -> Result<()> {
    let fetcher = Fetcher::new()?;
    let mut out = Vec::new();

    for r in results {
        let head = if let Ok(meta) = fetcher.head_metadata(&r.entry.llms_url).await {
            json!({
                "status": meta.status,
                "contentLength": meta.content_length,
                "etag": meta.etag,
                "lastModified": meta.last_modified,
            })
        } else {
            json!({})
        };
        let obj = json!({
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

    emit_registry_note(format, quiet, NoteChannel::ForceStderr);
    Ok(())
}

/// Handle interactive selection and add flow
async fn handle_interactive_add(
    selected_entry: &blz_core::registry::RegistryEntry,
    format: OutputFormat,
    quiet: bool,
    metrics: PerformanceMetrics,
) -> Result<()> {
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

    let descriptor = DescriptorInput::from_cli_inputs(
        &[],
        Some(&selected_entry.name),
        Some(&selected_entry.description),
        None,
        &[],
    );

    let request = AddRequest::new(
        final_alias.to_string(),
        selected_entry.llms_url.clone(),
        descriptor,
        false,
        quiet,
        metrics,
        false, // no_language_filter
    );

    add_source(request).await?;
    emit_registry_note(format, quiet, NoteChannel::Auto);
    Ok(())
}

/// Execute the lookup command to search registries
pub async fn execute(
    query: &str,
    metrics: PerformanceMetrics,
    quiet: bool,
    format: OutputFormat,
    limit: Option<usize>,
) -> Result<()> {
    let registry_enabled = std::env::var("BLZ_REGISTRY_ENABLED")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "on" | "yes"
            )
        })
        .unwrap_or(true);

    if !registry_enabled {
        let _ = metrics; // keep signature for future use
        return handle_registry_disabled(format, quiet);
    }

    let registry = Registry::new();

    if matches!(format, OutputFormat::Text) && !quiet {
        println!("Searching registries...");
    }
    let mut results = registry.search(query);

    if results.is_empty() {
        if matches!(format, OutputFormat::Text) && !quiet {
            println!("No matches found for '{query}'");
        }
        if matches!(format, OutputFormat::Json) {
            println!("[]");
        }
        emit_registry_note(format, quiet, NoteChannel::Auto);
        return Ok(());
    }

    if let Some(limit_count) = limit {
        results.truncate(limit_count);
    }

    if matches!(format, OutputFormat::Text) && !quiet {
        display_results_with_health(&results).await?;
    }

    // Non-interactive JSON output for agents
    if !matches!(format, OutputFormat::Text) {
        return output_results_json(&results, format, quiet).await;
    }

    // Try interactive selection
    let Some(selected_entry) = try_interactive_selection(&results).ok() else {
        if matches!(format, OutputFormat::Text) && !quiet {
            display_manual_instructions(&results);
        }
        emit_registry_note(format, quiet, NoteChannel::Auto);
        return Ok(());
    };

    handle_interactive_add(selected_entry, format, quiet, metrics).await
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
        let health = if let Ok(meta) = fetcher.head_metadata(&result.entry.llms_url).await {
            let ok = (200..300).contains(&i32::from(meta.status));
            let size = meta
                .content_length
                .map_or_else(|| "unknown size".to_string(), |n| format!("{n} bytes"));
            let status = if ok { "OK" } else { "ERR" };
            format!(" [{status} • {size}]")
        } else {
            String::new()
        };

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

    let options: Vec<SelectionItem> = results
        .iter()
        .enumerate()
        .map(|(i, result)| SelectionItem {
            index: i,
            label: format!("{}. {}", i + 1, result.entry),
        })
        .collect();

    let selection =
        Select::new("Select documentation to add (↑/↓ to navigate)", options).prompt()?;

    Ok(&results[selection.index].entry)
}

fn try_interactive_alias_input(default_alias: &str) -> Result<String> {
    if !std::io::stderr().is_terminal() {
        return Err(anyhow::anyhow!("Not in interactive terminal"));
    }

    let alias = Text::new("Enter alias")
        .with_default(default_alias)
        .prompt()?;

    if alias.trim().is_empty() {
        return Err(anyhow::anyhow!("Alias cannot be empty"));
    }

    Ok(alias)
}
