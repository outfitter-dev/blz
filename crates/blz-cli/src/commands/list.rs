//! List command implementation

use anyhow::{Context, Result};
use blz_core::{LlmsJson, Storage, TocEntry};
use colored::Colorize;
use serde_json::Value;
use std::fs;

use crate::output::OutputFormat;
use crate::utils::flavor::resolve_flavor;
use crate::utils::formatting::get_alias_color;

struct FlavorSummary {
    flavor: String,
    display_name: String,
    is_default: bool,
    data: Option<LlmsJson>,
    size_bytes: Option<u64>,
    headings: Option<usize>,
}

impl FlavorSummary {
    fn new(
        flavor: String,
        display_name: String,
        is_default: bool,
        data: Option<LlmsJson>,
        size_bytes: Option<u64>,
    ) -> Self {
        let headings = data.as_ref().map(|json| count_headings(&json.toc));
        Self {
            flavor,
            display_name,
            is_default,
            data,
            size_bytes,
            headings,
        }
    }

    fn lines(&self) -> Option<usize> {
        self.data.as_ref().map(|json| json.line_index.total_lines)
    }

    fn source(&self) -> Option<&blz_core::Source> {
        self.data.as_ref().map(|json| &json.source)
    }

    fn to_json(&self, status: bool) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("flavor".to_string(), Value::String(self.flavor.clone()));
        obj.insert(
            "displayName".to_string(),
            Value::String(self.display_name.clone()),
        );
        obj.insert("default".to_string(), Value::Bool(self.is_default));
        if let Some(lines) = self.lines() {
            obj.insert("lines".to_string(), serde_json::json!(lines));
        }
        if let Some(headings) = self.headings {
            obj.insert("headings".to_string(), serde_json::json!(headings));
        }
        if let Some(size) = self.size_bytes {
            obj.insert("sizeBytes".to_string(), serde_json::json!(size));
        }
        if let Some(src) = self.source() {
            obj.insert("url".to_string(), Value::String(src.url.clone()));
            obj.insert("fetchedAt".to_string(), serde_json::json!(src.fetched_at));
            obj.insert("sha256".to_string(), Value::String(src.sha256.clone()));
            obj.insert(
                "aliases".to_string(),
                serde_json::json!(src.aliases.clone()),
            );
            if status {
                if let Some(etag) = &src.etag {
                    obj.insert("etag".to_string(), Value::String(etag.clone()));
                }
                if let Some(last_modified) = &src.last_modified {
                    obj.insert(
                        "lastModified".to_string(),
                        Value::String(last_modified.clone()),
                    );
                }
            }
        }
        Value::Object(obj)
    }
}

struct SourceSummary {
    alias: String,
    flavors: Vec<FlavorSummary>,
    search_flavor: String, // The flavor that will actually be used for search
}

impl SourceSummary {
    fn default_flavor(&self) -> Option<&FlavorSummary> {
        self.flavors.iter().find(|flavor| flavor.is_default)
    }

    fn to_json(&self, status: bool) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("alias".to_string(), Value::String(self.alias.clone()));
        obj.insert("source".to_string(), Value::String(self.alias.clone()));

        // Always include searchFlavor - this is what will be used for searching
        obj.insert(
            "searchFlavor".to_string(),
            Value::String(self.search_flavor.clone()),
        );

        if let Some(default) = self.default_flavor() {
            if let Some(src) = default.source() {
                obj.insert("url".to_string(), Value::String(src.url.clone()));
                obj.insert("fetchedAt".to_string(), serde_json::json!(src.fetched_at));
                if let Some(lines) = default.lines() {
                    obj.insert("lines".to_string(), serde_json::json!(lines));
                }
                obj.insert("sha256".to_string(), Value::String(src.sha256.clone()));
                obj.insert(
                    "aliases".to_string(),
                    serde_json::json!(src.aliases.clone()),
                );
                if status {
                    if let Some(etag) = &src.etag {
                        obj.insert("etag".to_string(), Value::String(etag.clone()));
                    }
                    if let Some(last_modified) = &src.last_modified {
                        obj.insert(
                            "lastModified".to_string(),
                            Value::String(last_modified.clone()),
                        );
                    }
                }
            }
            // Keep defaultFlavor for backwards compatibility but deprecated
            obj.insert(
                "defaultFlavor".to_string(),
                Value::String(default.flavor.clone()),
            );
        }

        let flavors: Vec<Value> = self
            .flavors
            .iter()
            .map(|flavor| flavor.to_json(status))
            .collect();
        obj.insert("flavors".to_string(), Value::Array(flavors));

        Value::Object(obj)
    }
}

/// Execute the list command to show all cached sources
pub async fn execute(format: OutputFormat, status: bool, quiet: bool) -> Result<()> {
    let storage = Storage::new()?;
    let aliases = storage.list_sources();

    if aliases.is_empty() {
        emit_empty_list(format, quiet);
        return Ok(());
    }

    let mut summaries = Vec::new();

    for alias in &aliases {
        let mut flavors = storage
            .available_flavors(alias)
            .with_context(|| format!("Failed to list flavors for alias '{alias}'"))?;
        if !flavors.iter().any(|f| f.eq_ignore_ascii_case("llms")) {
            flavors.push("llms".to_string());
        }

        // Use resolve_flavor to determine what will actually be used
        let default_flavor = resolve_flavor(&storage, alias)
            .with_context(|| format!("Failed to resolve flavor for alias '{alias}'"))?;

        flavors.sort();
        flavors.dedup();
        flavors.sort_by(|a, b| {
            if a.eq_ignore_ascii_case(&default_flavor) {
                std::cmp::Ordering::Less
            } else if b.eq_ignore_ascii_case(&default_flavor) {
                std::cmp::Ordering::Greater
            } else {
                a.cmp(b)
            }
        });

        let mut flavor_summaries = Vec::new();
        for flavor in flavors {
            let display_name = flavor_display_name(&flavor);
            let is_default = flavor.eq_ignore_ascii_case(&default_flavor);
            let data = load_flavor_json(&storage, alias, &flavor)?;
            let size_bytes = storage
                .flavor_file_path(alias, &flavor)
                .ok()
                .and_then(|path| fs::metadata(path).ok().map(|meta| meta.len()));

            flavor_summaries.push(FlavorSummary::new(
                flavor,
                display_name,
                is_default,
                data,
                size_bytes,
            ));
        }

        summaries.push(SourceSummary {
            alias: alias.clone(),
            flavors: flavor_summaries,
            search_flavor: default_flavor,
        });
    }

    if summaries.is_empty() {
        emit_empty_list(format, quiet);
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let payload: Vec<Value> = summaries
                .iter()
                .map(|summary| summary.to_json(status))
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&payload)
                    .context("Failed to serialize list payload to JSON")?
            );
        },
        OutputFormat::Jsonl => {
            for summary in &summaries {
                let line = serde_json::to_string(&summary.to_json(status)).with_context(|| {
                    format!(
                        "Failed to serialize list entry for alias '{}'",
                        summary.alias
                    )
                })?;
                println!("{line}");
            }
        },
        OutputFormat::Text => {
            display_sources_text(&summaries, status, quiet);
        },
    }

    Ok(())
}

fn emit_empty_list(format: OutputFormat, quiet: bool) {
    match format {
        OutputFormat::Json => println!("[]"),
        OutputFormat::Jsonl => {},
        OutputFormat::Text => {
            if !quiet {
                println!("No sources found. Use 'blz add' to add sources.");
            }
        },
    }
}

fn display_sources_text(summaries: &[SourceSummary], status: bool, quiet: bool) {
    if !quiet {
        println!("\nCached sources:\n");
    }

    for (index, summary) in summaries.iter().enumerate() {
        let alias_colored = get_alias_color(&summary.alias, index);
        println!("{alias_colored}");

        for flavor in &summary.flavors {
            let bullet = if flavor.is_default { "◆" } else { "◇" };
            let mut label = flavor.display_name.clone();
            if flavor.is_default {
                label.push_str(" (default)");
            }

            let lines_display = flavor
                .lines()
                .map_or_else(|| "?".to_string(), |v| v.to_string());
            let headings_display = flavor
                .headings
                .map_or_else(|| "?".to_string(), |v| v.to_string());
            let size_display = flavor
                .size_bytes
                .map_or_else(|| "?".to_string(), format_size);

            println!(
                "{} {}  {}  {}  {}",
                bullet,
                label,
                format!("Lines: {lines_display}").bright_black(),
                format!("Headings: {headings_display}").bright_black(),
                format!("Size: {size_display}").bright_black(),
            );

            if let Some(src) = flavor.source() {
                println!("  {}", src.url.bright_black());
                let fetched = src.fetched_at.format("%Y-%m-%d %H:%M:%S");
                let updated = src.last_modified.as_deref().unwrap_or("(unknown)");
                let fetched_text = format!("Fetched: {fetched}").bright_black();
                let updated_text = format!("Updated: {updated}").bright_black();
                if status {
                    println!("  {fetched_text}  {updated_text}");
                } else {
                    println!("  {fetched_text}");
                }
            }
        }

        if !quiet {
            println!();
        }
    }
}

fn flavor_display_name(flavor: &str) -> String {
    if flavor.eq_ignore_ascii_case("llms") {
        "llms.txt".to_string()
    } else {
        format!("{flavor}.txt")
    }
}

fn load_flavor_json(storage: &Storage, alias: &str, flavor: &str) -> Result<Option<LlmsJson>> {
    // Treat all flavors uniformly - return Ok(None) if the file doesn't exist
    storage
        .load_flavor_json(alias, flavor)
        .map_err(anyhow::Error::from)
}

fn count_headings(entries: &[TocEntry]) -> usize {
    entries
        .iter()
        .map(|entry| 1 + count_headings(&entry.children))
        .sum()
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    #[allow(clippy::cast_precision_loss)]
    let mut size = bytes as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}
