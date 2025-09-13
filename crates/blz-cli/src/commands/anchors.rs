use anyhow::Result;
use blz_core::{AnchorsMap, LlmsJson, Storage};
use colored::Colorize;

use crate::commands::get_lines;
use crate::output::OutputFormat;

pub async fn execute(alias: &str, output: OutputFormat, mappings: bool) -> Result<()> {
    let storage = Storage::new()?;

    if mappings {
        let path = storage.anchors_map_path(alias)?;
        if !path.exists() {
            println!("No anchors mappings found for '{alias}'");
            return Ok(());
        }
        let txt = std::fs::read_to_string(&path)?;
        let map: AnchorsMap = serde_json::from_str(&txt)?;
        match output {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&map)?);
            },
            OutputFormat::Ndjson => {
                for m in map.mappings {
                    println!("{}", serde_json::to_string(&m)?);
                }
            },
            OutputFormat::Text => {
                println!(
                    "Anchors remap for {} (updated {})\n",
                    alias.green(),
                    map.updated_at
                );
                for m in map.mappings {
                    let path_str = m.heading_path.join(" > ");
                    println!(
                        "  {}\n    {} → {}\n    {}",
                        path_str,
                        m.old_lines,
                        m.new_lines,
                        m.anchor.bright_black()
                    );
                }
            },
        }
        return Ok(());
    }

    let llms: LlmsJson = storage.load_llms_json(alias)?;
    let mut entries = Vec::new();
    collect_entries(&mut entries, &llms.toc);

    match output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&entries)?);
        },
        OutputFormat::Ndjson => {
            for e in entries {
                println!("{}", serde_json::to_string(&e)?);
            }
        },
        OutputFormat::Text => {
            println!("Anchors for {}\n", alias.green());
            for e in &llms.toc {
                print_text(e, 0);
            }
        },
    }
    Ok(())
}

#[allow(clippy::items_after_statements)]
fn collect_entries(entries: &mut Vec<serde_json::Value>, list: &[blz_core::TocEntry]) {
    for e in list {
        entries.push(serde_json::json!({
            "headingPath": e.heading_path,
            "lines": e.lines,
            "anchor": e.anchor,
        }));
        if !e.children.is_empty() {
            collect_entries(entries, &e.children);
        }
    }
}

fn print_text(e: &blz_core::TocEntry, depth: usize) {
    let indent = "  ".repeat(depth);
    let name = e.heading_path.last().cloned().unwrap_or_default();
    let anchor = e.anchor.clone().unwrap_or_default();
    println!(
        "{}- {}  {}  {}",
        indent,
        name,
        e.lines,
        anchor.bright_black()
    );
    for c in &e.children {
        print_text(c, depth + 1);
    }
}

/// Get lines by anchor
pub async fn get_by_anchor(alias: &str, anchor: &str, context: Option<usize>) -> Result<()> {
    let storage = Storage::new()?;
    let llms: LlmsJson = storage.load_llms_json(alias)?;

    #[allow(clippy::items_after_statements)]
    fn find<'a>(list: &'a [blz_core::TocEntry], a: &str) -> Option<&'a blz_core::TocEntry> {
        for e in list {
            if e.anchor.as_deref() == Some(a) {
                return Some(e);
            }
            if let Some(f) = find(&e.children, a) {
                return Some(f);
            }
        }
        None
    }

    let Some(entry) = find(&llms.toc, anchor) else {
        println!("Anchor not found for '{anchor}' in '{alias}'");
        println!("Hint: run 'blz anchor list {alias}' to see available anchors");
        return Ok(());
    };

    // Use existing 'get' implementation to print lines with context
    get_lines(alias, &entry.lines, context).await
}
