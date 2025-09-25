//! List command implementation

use anyhow::Result;
use blz_core::{LlmsJson, Source, Storage};
use colored::Colorize;

use crate::output::OutputFormat;
use crate::utils::formatting::get_alias_color;
use serde_json::Value;

struct FlavorSummary {
    name: String,
    source: Option<Source>,
    lines: Option<usize>,
}

impl FlavorSummary {
    fn from_llms_json(name: String, data: &LlmsJson) -> Self {
        Self {
            name,
            source: Some(data.source.clone()),
            lines: Some(data.line_index.total_lines),
        }
    }

    fn empty(name: String) -> Self {
        Self {
            name,
            source: None,
            lines: None,
        }
    }

    fn to_json(&self, status: bool) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("flavor".to_string(), Value::String(self.name.clone()));
        if let Some(src) = &self.source {
            obj.insert("url".to_string(), Value::String(src.url.clone()));
            obj.insert("fetchedAt".to_string(), serde_json::json!(src.fetched_at));
            if let Some(lines) = self.lines {
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
        Value::Object(obj)
    }
}

struct SourceSummary {
    alias: String,
    default: FlavorSummary,
    extras: Vec<FlavorSummary>,
}

impl SourceSummary {
    fn all_flavors(&self) -> impl Iterator<Item = &FlavorSummary> {
        std::iter::once(&self.default).chain(self.extras.iter())
    }

    fn to_json(&self, status: bool) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("alias".to_string(), Value::String(self.alias.clone()));
        obj.insert("source".to_string(), Value::String(self.alias.clone()));

        if let Some(src) = &self.default.source {
            obj.insert("url".to_string(), Value::String(src.url.clone()));
            obj.insert("fetchedAt".to_string(), serde_json::json!(src.fetched_at));
            if let Some(lines) = self.default.lines {
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

        let flavors: Vec<Value> = self
            .all_flavors()
            .map(|flavor| flavor.to_json(status))
            .collect();
        obj.insert("flavors".to_string(), Value::Array(flavors));

        Value::Object(obj)
    }
}

/// Execute the list command to show all cached sources
pub async fn execute(format: OutputFormat, status: bool, quiet: bool) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources();

    if sources.is_empty() {
        match format {
            OutputFormat::Json => {
                println!("[]");
            },
            OutputFormat::Jsonl => {
                // No output when empty
            },
            OutputFormat::Text => {
                if !quiet {
                    println!("No sources found. Use 'blz add' to add sources.");
                }
            },
        }
        return Ok(());
    }

    let mut summaries = Vec::new();
    for source in &sources {
        let default_json = match storage.load_llms_json(source) {
            Ok(json) => json,
            Err(_) => continue,
        };

        let default = FlavorSummary::from_llms_json("llms".to_string(), &default_json);
        let mut extras = Vec::new();

        if let Ok(mut flavors) = storage.available_flavors(source) {
            flavors.sort();
            flavors.dedup();
            for flavor in flavors {
                if flavor.eq_ignore_ascii_case("llms") {
                    continue;
                }
                match storage.load_flavor_json(source, &flavor) {
                    Ok(Some(json)) => {
                        extras.push(FlavorSummary::from_llms_json(flavor.clone(), &json))
                    },
                    _ => extras.push(FlavorSummary::empty(flavor)),
                }
            }
        }

        extras.sort_by(|a, b| a.name.cmp(&b.name));
        summaries.push(SourceSummary {
            alias: source.clone(),
            default,
            extras,
        });
    }

    if summaries.is_empty() {
        match format {
            OutputFormat::Json => println!("[]"),
            OutputFormat::Jsonl => {},
            OutputFormat::Text => {
                if !quiet {
                    println!("No sources found. Use 'blz add' to add sources.");
                }
            },
        }
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let payload: Vec<Value> = summaries
                .iter()
                .map(|summary| summary.to_json(status))
                .collect();
            println!("{}", serde_json::to_string_pretty(&payload)?);
        },
        OutputFormat::Jsonl => {
            for summary in &summaries {
                println!("{}", serde_json::to_string(&summary.to_json(status))?);
            }
        },
        OutputFormat::Text => {
            display_sources_text(&summaries, status, quiet);
        },
    }

    Ok(())
}

fn display_sources_text(summaries: &[SourceSummary], status: bool, quiet: bool) {
    if !quiet {
        println!("\nCached sources:\n");
    }

    for (index, summary) in summaries.iter().enumerate() {
        let alias_colored = get_alias_color(&summary.alias, index);
        if let Some(src) = &summary.default.source {
            println!("  {} {}", alias_colored, src.url.bright_black());
            println!(
                "    Fetched: {}",
                src.fetched_at.format("%Y-%m-%d %H:%M:%S")
            );
            if let Some(lines) = summary.default.lines {
                println!("    Lines: {}", lines);
            }
            if status {
                if let Some(etag) = &src.etag {
                    println!("    ETag: {etag}");
                }
                if let Some(lm) = &src.last_modified {
                    println!("    Last-Modified: {lm}");
                }
                println!("    Checksum: {}", src.sha256);
            }
        } else {
            println!("  {}", alias_colored);
        }

        let default_name = summary.default.name.clone();
        let flavor_labels: Vec<String> = summary
            .all_flavors()
            .map(|flavor| {
                let mut label = flavor.name.clone();
                if flavor.name.eq_ignore_ascii_case(&default_name) {
                    label.push_str(" (default)");
                }
                label
            })
            .collect();
        println!("    Flavors: {}", flavor_labels.join(", "));

        if !quiet {
            println!();
        }
    }
}
