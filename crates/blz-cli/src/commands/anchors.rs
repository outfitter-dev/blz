use anyhow::Result;
use blz_core::{AnchorsMap, LlmsJson, Storage};
use colored::Colorize;

use crate::commands::get_lines;
use crate::output::OutputFormat;
use crate::utils::parsing::{LineRange, parse_line_ranges};

pub async fn execute(alias: &str, output: OutputFormat, mappings: bool) -> Result<()> {
    let storage = Storage::new()?;
    // Resolve metadata alias to canonical if needed
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .map_or_else(|| alias.to_string(), |c| c);

    if mappings {
        let path = storage.anchors_map_path(&canonical)?;
        if !path.exists() {
            println!("No anchors mappings found for '{canonical}'");
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
                    canonical.green(),
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

    let llms: LlmsJson = storage.load_llms_json(&canonical)?;
    let mut entries = Vec::new();
    collect_entries(&mut entries, &llms.toc);
    // Replace placeholder with actual alias for each entry in JSON/NDJSON
    for e in &mut entries {
        if let Some(obj) = e.as_object_mut() {
            if obj.get("source").is_some() {
                obj.insert(
                    "source".to_string(),
                    serde_json::Value::String(canonical.clone()),
                );
            }
        }
    }

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
            println!("Anchors for {}\n", canonical.green());
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
            "source": "__ALIAS__", // placeholder, replaced by caller
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
pub async fn get_by_anchor(
    alias: &str,
    anchor: &str,
    context: Option<usize>,
    output: OutputFormat,
) -> Result<()> {
    let storage = Storage::new()?;
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .map_or_else(|| alias.to_string(), |c| c);
    let llms: LlmsJson = storage.load_llms_json(&canonical)?;

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
        println!("Anchor not found for '{anchor}' in '{canonical}'");
        println!("Hint: run 'blz anchor list {canonical}' to see available anchors");
        return Ok(());
    };

    match output {
        OutputFormat::Text => {
            // Use existing 'get' implementation to print lines with context
            get_lines(&canonical, &entry.lines, context, OutputFormat::Text).await
        },
        OutputFormat::Json | OutputFormat::Ndjson => {
            // Build content string for the range +/- context
            let file_content = storage.load_llms_txt(&canonical)?;
            let all_lines: Vec<&str> = file_content.lines().collect();
            let (body, line_numbers) = extract_content(&entry.lines, context, &all_lines)?;
            let obj = serde_json::json!({
                "alias": canonical,
                "source": canonical,
                "anchor": anchor,
                "headingPath": entry.heading_path,
                "lines": entry.lines,
                "lineNumbers": line_numbers,
                "content": body,
            });
            if matches!(output, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&obj)?);
            } else {
                println!("{}", serde_json::to_string(&obj)?);
            }
            Ok(())
        },
    }
}

fn extract_content(
    lines_spec: &str,
    context: Option<usize>,
    all_lines: &[&str],
) -> Result<(String, Vec<usize>)> {
    let ranges = parse_line_ranges(lines_spec)
        .map_err(|_| anyhow::anyhow!("Invalid lines format in anchor entry: {}", lines_spec))?;
    let ctx = context.unwrap_or(0);
    let mut selected: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for r in ranges {
        match r {
            LineRange::Single(line) => add_with_context(&mut selected, line, ctx, all_lines.len()),
            LineRange::Range(start, end) => {
                add_range_with_context(&mut selected, start, end, ctx, all_lines.len());
            },
            LineRange::PlusCount(start, count) => {
                let end = start + count - 1;
                add_range_with_context(&mut selected, start, end, ctx, all_lines.len());
            },
        }
    }
    let mut out = String::new();
    for (i, &ln) in selected.iter().enumerate() {
        if ln == 0 || ln > all_lines.len() {
            continue;
        }
        if i > 0 {
            out.push('\n');
        }
        out.push_str(all_lines[ln - 1]);
    }
    Ok((out, selected.into_iter().collect()))
}

fn add_with_context(
    set: &mut std::collections::BTreeSet<usize>,
    line: usize,
    ctx: usize,
    total: usize,
) {
    let start = line.saturating_sub(ctx + 1);
    let end = (line + ctx).min(total);
    for i in start..end {
        set.insert(i + 1);
    }
}

fn add_range_with_context(
    set: &mut std::collections::BTreeSet<usize>,
    start: usize,
    end: usize,
    ctx: usize,
    total: usize,
) {
    let s = start.saturating_sub(ctx + 1);
    let e = (end + ctx).min(total);
    for i in s..e {
        set.insert(i + 1);
    }
}
