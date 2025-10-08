use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct SourceToml {
    id: String,
    name: Option<String>,
    description: Option<String>,
    url: String,
    fallback: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    #[serde(rename = "registeredAt")]
    registered_at: Option<String>,
    #[serde(rename = "verifiedAt")]
    verified_at: Option<String>,
    aliases: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Serialize)]
struct RegistrySource {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fallback: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(rename = "registeredAt")]
    registered_at: String,
    #[serde(rename = "verifiedAt")]
    verified_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    aliases: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Serialize)]
struct Registry {
    version: String,
    updated: String,
    sources: Vec<RegistrySource>,
}

fn title_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> Result<()> {
    let registry_dir = PathBuf::from("registry/sources");
    let output_path = PathBuf::from("registry.json");

    // Get current timestamp
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Read all TOML files
    let mut sources = Vec::new();

    for entry in fs::read_dir(&registry_dir).context("Failed to read registry/sources directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let source: SourceToml = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        // Convert to registry source with defaults
        let name = source.name.unwrap_or_else(|| title_case(&source.id));
        let registered_at = source.registered_at.unwrap_or_else(|| now.clone());
        let verified_at = source.verified_at.unwrap_or_else(|| now.clone());

        sources.push(RegistrySource {
            id: source.id,
            name,
            description: source.description,
            url: source.url,
            fallback: source.fallback,
            category: source.category,
            tags: source.tags,
            registered_at,
            verified_at,
            aliases: source.aliases,
        });
    }

    // Sort by ID
    sources.sort_by(|a, b| a.id.cmp(&b.id));

    // Create registry
    let registry = Registry {
        version: "1.0.0".to_string(),
        updated: now,
        sources,
    };

    // Write to file
    let json = serde_json::to_string_pretty(&registry)?;
    fs::write(&output_path, json)?;

    println!(
        "✓ Generated registry.json with {} sources",
        registry.sources.len()
    );

    Ok(())
}
