use anyhow::{Context, Result, anyhow};
use blz_core::{AnchorsMap, LlmsJson, Storage};
use colored::Colorize;

use crate::commands::RequestSpec;
use crate::output::OutputFormat;
use crate::utils::parsing::{LineRange, parse_line_ranges};

#[allow(dead_code, clippy::unused_async, clippy::too_many_lines)]
pub async fn execute(
    alias: &str,
    output: OutputFormat,
    mappings: bool,
    limit: Option<usize>,
) -> Result<()> {
    let storage = Storage::new()?;
    // Resolve metadata alias to canonical if needed
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .map_or_else(|| alias.to_string(), |c| c);

    if mappings {
        let path = storage.anchors_map_path(&canonical)?;
        if !path.exists() {
            println!("No anchor remap metadata found for '{canonical}'");
            return Ok(());
        }
        let txt = std::fs::read_to_string(&path)?;
        let map: AnchorsMap = serde_json::from_str(&txt)?;
        match output {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&map)?);
            },
            OutputFormat::Jsonl => {
                for m in map.mappings {
                    println!("{}", serde_json::to_string(&m)?);
                }
            },
            OutputFormat::Text => {
                println!(
                    "Anchor remap metadata for {} (updated {})\n",
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
            OutputFormat::Raw => {
                return Err(anyhow!(
                    "Raw output is not supported for toc listings. Use --format json, jsonl, or text instead."
                ));
            },
        }
        return Ok(());
    }

    // Load JSON metadata (Phase 3: always llms.txt)
    let llms: LlmsJson = storage
        .load_llms_json(&canonical)
        .with_context(|| format!("Failed to load TOC for '{canonical}'"))?;
    let mut entries = Vec::new();
    collect_entries(&mut entries, &llms.toc);

    // Apply limit to entries
    if let Some(limit_count) = limit {
        entries.truncate(limit_count);
    }

    // Replace placeholder with actual alias/source for each entry in JSON/JSONL output
    for e in &mut entries {
        if let Some(obj) = e.as_object_mut() {
            // Add alias field (what the user typed)
            obj.insert(
                "alias".to_string(),
                serde_json::Value::String(alias.to_string()),
            );
            // Update source to canonical (the resolved source name)
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
            println!(
                "{}",
                serde_json::to_string_pretty(&entries)
                    .context("Failed to serialize table of contents to JSON")?
            );
        },
        OutputFormat::Jsonl => {
            for e in entries {
                println!(
                    "{}",
                    serde_json::to_string(&e)
                        .context("Failed to serialize table of contents to JSONL")?
                );
            }
        },
        OutputFormat::Text => {
            println!("Table of contents for {}\n", canonical.green());
            let mut count = 0;
            for e in &llms.toc {
                if let Some(limit_count) = limit {
                    if count >= limit_count {
                        break;
                    }
                    count += print_text_with_limit(e, 0, limit_count - count);
                } else {
                    print_text(e, 0);
                }
            }
        },
        OutputFormat::Raw => {
            return Err(anyhow!(
                "Raw output is not supported for toc listings. Use --format json, jsonl, or text instead."
            ));
        },
    }
    Ok(())
}

#[allow(dead_code)]
fn print_text_with_limit(e: &blz_core::TocEntry, depth: usize, remaining: usize) -> usize {
    if remaining == 0 {
        return 0;
    }

    let indent = "  ".repeat(depth);
    let name = display_path(e).last().cloned().unwrap_or_default();
    let anchor = e.anchor.clone().unwrap_or_default();
    println!(
        "{}- {}  {}  {}",
        indent,
        name,
        e.lines,
        anchor.bright_black()
    );

    let mut printed = 1;
    for c in &e.children {
        if printed >= remaining {
            break;
        }
        printed += print_text_with_limit(c, depth + 1, remaining - printed);
    }
    printed
}

#[allow(dead_code, clippy::items_after_statements)]
fn collect_entries(entries: &mut Vec<serde_json::Value>, list: &[blz_core::TocEntry]) {
    for e in list {
        let display_path = display_path(e);
        entries.push(serde_json::json!({
            "source": "__ALIAS__", // placeholder, replaced by caller
            "headingPath": display_path,
            "rawHeadingPath": e.heading_path,
            "headingPathNormalized": e.heading_path_normalized,
            "lines": e.lines,
            "anchor": e.anchor,
        }));
        if !e.children.is_empty() {
            collect_entries(entries, &e.children);
        }
    }
}

#[allow(dead_code)]
fn print_text(e: &blz_core::TocEntry, depth: usize) {
    let indent = "  ".repeat(depth);
    let name = display_path(e).last().cloned().unwrap_or_default();
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

fn display_path(entry: &blz_core::TocEntry) -> Vec<String> {
    entry
        .heading_path_display
        .clone()
        .unwrap_or_else(|| entry.heading_path.clone())
}

/// Get lines by anchor
#[allow(dead_code, clippy::unused_async)]
pub async fn get_by_anchor(
    alias: &str,
    anchor: &str,
    context: Option<usize>,
    output: OutputFormat,
) -> Result<()> {
    let storage = Storage::new()?;
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .map_or_else(|| alias.to_string(), |c| c);
    // Load JSON metadata (Phase 3: always llms.txt)
    let llms: LlmsJson = storage
        .load_llms_json(&canonical)
        .with_context(|| format!("Failed to load TOC for '{canonical}'"))?;

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
        println!("Hint: run 'blz toc {canonical}' to inspect available headings");
        return Ok(());
    };

    match output {
        OutputFormat::Text => {
            // Convert context to ContextMode
            let context_mode = context.map(crate::cli::ContextMode::Symmetric);
            let requests = vec![RequestSpec {
                alias: alias.to_string(),
                line_expression: entry.lines.clone(),
            }];
            crate::commands::get_lines(
                &requests,
                context_mode.as_ref(),
                false,
                None,
                OutputFormat::Text,
                false,
            )
            .await
        },
        OutputFormat::Json | OutputFormat::Jsonl => {
            // Build content string for the range +/- context
            let file_path = storage.llms_txt_path(&canonical)?;
            let file_content = std::fs::read_to_string(&file_path).with_context(|| {
                format!(
                    "Failed to read llms.txt content from {}",
                    file_path.display()
                )
            })?;
            let all_lines: Vec<&str> = file_content.lines().collect();
            let (body, line_numbers) = extract_content(&entry.lines, context, &all_lines)?;
            let display_path = display_path(entry);
            let obj = serde_json::json!({
                "alias": alias,
                "source": canonical,
                "anchor": anchor,
                "headingPath": display_path,
                "rawHeadingPath": entry.heading_path,
                "headingPathNormalized": entry.heading_path_normalized,
                "lines": entry.lines,
                "lineNumbers": line_numbers,
                "content": body,
            });
            if matches!(output, OutputFormat::Json) {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&obj)
                        .context("Failed to serialize anchor content to JSON")?
                );
            } else {
                println!(
                    "{}",
                    serde_json::to_string(&obj)
                        .context("Failed to serialize anchor content to JSONL")?
                );
            }
            Ok(())
        },
        OutputFormat::Raw => Err(anyhow!(
            "Raw output is not supported for toc listings. Use --format json, jsonl, or text instead."
        )),
    }
}

#[allow(dead_code)]
fn extract_content(
    lines_spec: &str,
    context: Option<usize>,
    all_lines: &[&str],
) -> Result<(String, Vec<usize>)> {
    let ranges = parse_line_ranges(lines_spec)
        .map_err(|_| anyhow::anyhow!("Invalid lines format in anchor entry: {lines_spec}"))?;
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
