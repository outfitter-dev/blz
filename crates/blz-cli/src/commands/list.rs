//! List command implementation

use anyhow::{Context, Result};
use blz_core::Storage;
use colored::Colorize;
use serde_json::Value;

use crate::output::OutputFormat;
use crate::utils::count_headings;
use crate::utils::formatting::get_alias_color;

/// Execute the list command
pub async fn execute(format: OutputFormat, status: bool) -> Result<()> {
    let storage = Storage::new()?;
    let aliases = storage.list_sources();

    if aliases.is_empty() {
        if matches!(format, OutputFormat::Text) {
            println!("No sources configured. Use 'blz add' to add sources.");
        } else {
            println!("[]");
        }
        return Ok(());
    }

    let mut sources = Vec::new();
    for alias in aliases {
        // Load metadata for this source
        let metadata = storage
            .load_source_metadata(&alias)?
            .with_context(|| format!("Failed to load metadata for '{}'", alias))?;

        // Load JSON for line count and headings
        let json_data = storage
            .load_llms_json(&alias)
            .with_context(|| format!("Failed to load JSON for '{}'", alias))?;

        sources.push(SourceInfo {
            source: alias.clone(),
            url: metadata.url,
            tags: metadata.tags,
            aliases: metadata.aliases,
            fetched_at: metadata.fetched_at.to_rfc3339(),
            sha256: metadata.sha256,
            etag: metadata.etag,
            last_modified: metadata.last_modified,
            lines: json_data.line_index.total_lines,
            headings: count_headings(&json_data.toc),
        });
    }

    match format {
        OutputFormat::Text => print_text_format(&sources, status),
        OutputFormat::Json => print_json_format(&sources, status)?,
        OutputFormat::Jsonl => print_jsonl_format(&sources, status)?,
    }

    Ok(())
}

struct SourceInfo {
    source: String,
    url: String,
    tags: Vec<String>,
    aliases: Vec<String>,
    fetched_at: String,
    sha256: String,
    etag: Option<String>,
    last_modified: Option<String>,
    lines: usize,
    headings: usize,
}

fn print_text_format(sources: &[SourceInfo], status: bool) {
    for (idx, source) in sources.iter().enumerate() {
        let colored_alias = get_alias_color(&source.source, idx);
        println!("{} - {}", colored_alias, source.url.bright_black());
        println!("  {} lines, {} headings", source.lines, source.headings);

        if !source.tags.is_empty() {
            println!("  Tags: {}", source.tags.join(", "));
        }

        if status {
            println!("  Last updated: {}", source.fetched_at);
            if let Some(etag) = &source.etag {
                println!("  ETag: {}", etag);
            }
            if let Some(last_modified) = &source.last_modified {
                println!("  Last-Modified: {}", last_modified);
            }
        }
        println!();
    }
}

fn print_json_format(sources: &[SourceInfo], status: bool) -> Result<()> {
    let json_sources: Vec<Value> = sources
        .iter()
        .map(|source| source_to_json(source, status))
        .collect();
    println!("{}", serde_json::to_string_pretty(&json_sources)?);
    Ok(())
}

fn print_jsonl_format(sources: &[SourceInfo], status: bool) -> Result<()> {
    for source in sources {
        println!(
            "{}",
            serde_json::to_string(&source_to_json(source, status))?
        );
    }
    Ok(())
}

fn source_to_json(source: &SourceInfo, status: bool) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert("alias".to_string(), Value::String(source.source.clone()));
    obj.insert("url".to_string(), Value::String(source.url.clone()));
    obj.insert("lines".to_string(), serde_json::json!(source.lines));
    obj.insert("headings".to_string(), serde_json::json!(source.headings));
    obj.insert("tags".to_string(), serde_json::json!(source.tags.clone()));
    obj.insert(
        "aliases".to_string(),
        serde_json::json!(source.aliases.clone()),
    );
    obj.insert(
        "fetchedAt".to_string(),
        Value::String(source.fetched_at.clone()),
    );
    obj.insert("sha256".to_string(), Value::String(source.sha256.clone()));

    if status {
        if let Some(etag) = &source.etag {
            obj.insert("etag".to_string(), Value::String(etag.clone()));
        }
        if let Some(last_modified) = &source.last_modified {
            obj.insert(
                "lastModified".to_string(),
                Value::String(last_modified.clone()),
            );
        }
    }

    Value::Object(obj)
}
