use anyhow::{Context, Result, anyhow};
use blz_core::Storage;
use colored::Colorize;

use crate::utils::validation::validate_relaxed_alias;

/// Alias management
///
/// Persists relaxed "metadata aliases" in `Source.aliases` (llms.json and metadata.json).
pub enum AliasCommand {
    Add { source: String, alias: String },
    Rm { source: String, alias: String },
}

pub async fn execute(cmd: AliasCommand) -> Result<()> {
    match cmd {
        AliasCommand::Add { source, alias } => add_alias(&source, &alias)?,
        AliasCommand::Rm { source, alias } => remove_alias(&source, &alias)?,
    }
    Ok(())
}

fn add_alias(source: &str, new_alias: &str) -> Result<()> {
    let storage = Storage::new()?;
    if !storage.exists(source) {
        return Err(anyhow!("Source '{source}' not found"));
    }

    validate_relaxed_alias(new_alias)?;

    // Prevent adding the canonical as an alias
    if source.eq_ignore_ascii_case(new_alias) {
        return Err(anyhow!(
            "Alias '{new_alias}' matches the canonical source name; nothing to add"
        ));
    }

    // Enforce uniqueness across all sources
    if let Some(owner) = find_alias_owner(&storage, new_alias) {
        if owner != source {
            return Err(anyhow!(
                "Alias '{new_alias}' is already used by source '{owner}'"
            ));
        }
    }

    // Load, mutate, save
    let mut llms = storage
        .load_llms_json(source)
        .with_context(|| format!("Failed loading llms.json for '{source}'"))?;

    if llms.metadata.aliases.iter().any(|a| a == new_alias) {
        println!(
            "{} '{}' already has alias '{}'.",
            "No-op".bright_black(),
            source.green(),
            new_alias
        );
        return Ok(());
    }
    llms.metadata.aliases.push(new_alias.to_string());
    storage.save_llms_json(source, &llms)?;
    storage.save_source_metadata(source, &llms.metadata)?;

    println!(
        "{} Added alias '{}' to {}",
        "✓".green(),
        new_alias.bold(),
        source.green()
    );
    Ok(())
}

fn remove_alias(source: &str, alias: &str) -> Result<()> {
    let storage = Storage::new()?;
    if !storage.exists(source) {
        return Err(anyhow!("Source '{source}' not found"));
    }

    let mut llms = storage
        .load_llms_json(source)
        .with_context(|| format!("Failed loading llms.json for '{source}'"))?;

    let before = llms.metadata.aliases.len();
    llms.metadata.aliases.retain(|a| a != alias);
    if llms.metadata.aliases.len() == before {
        println!(
            "{} Alias '{}' not found on {}",
            "No-op".bright_black(),
            alias,
            source.green()
        );
        return Ok(());
    }

    storage.save_llms_json(source, &llms)?;
    storage.save_source_metadata(source, &llms.metadata)?;

    println!(
        "{} Removed alias '{}' from {}",
        "✓".green(),
        alias.bold(),
        source.green()
    );
    Ok(())
}

fn find_alias_owner(storage: &Storage, alias: &str) -> Option<String> {
    let mut owner: Option<String> = None;
    for src in storage.list_sources() {
        if let Ok(meta) = storage.load_llms_json(&src) {
            if meta.metadata.aliases.iter().any(|a| a == alias) {
                if owner.is_some() {
                    // Ambiguous across multiple sources; treat as already taken
                    return owner;
                }
                owner = Some(src);
            }
        }
    }
    owner
}
