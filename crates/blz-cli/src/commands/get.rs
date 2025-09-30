//! Get command implementation for retrieving specific lines from sources

use anyhow::{Context, Result};
use blz_core::Storage;
use colored::Colorize;
use std::collections::BTreeSet;

use crate::output::OutputFormat;
use crate::utils::parsing::{LineRange, parse_line_ranges};

/// Execute the get command to retrieve specific lines from a source
pub async fn execute(
    alias: &str,
    lines: &str,
    context: Option<usize>,
    format: OutputFormat,
) -> Result<()> {
    let storage = Storage::new()?;

    // Resolve metadata alias to canonical if needed
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .map_or_else(|| alias.to_string(), |c| c);

    if !storage.exists(&canonical) {
        println!("Source '{alias}' not found.");
        let available = storage.list_sources();
        if available.is_empty() {
            println!(
                "No sources available. Use 'blz lookup <name>' or 'blz add <alias> <url>' to add one."
            );
        } else {
            let preview = available.iter().take(8).cloned().collect::<Vec<_>>();
            println!(
                "Available: {}{}",
                preview.join(", "),
                if available.len() > preview.len() {
                    format!(" (+{} more)", available.len() - preview.len())
                } else {
                    String::new()
                }
            );
            println!("Hint: 'blz list' to see all, or 'blz lookup <name>' to search registries.");
        }
        return Ok(());
    }

    // Always read from llms.txt (simplified from flavor logic)
    let file_path = storage.llms_txt_path(&canonical)?;
    let file_content = std::fs::read_to_string(&file_path).with_context(|| {
        format!(
            "Failed to read llms.txt for source '{}' at {}",
            canonical,
            file_path.display()
        )
    })?;

    let file_lines: Vec<&str> = file_content.lines().collect();
    let ranges = parse_line_ranges(lines)?;

    // Collect all requested line numbers (1-based) and expand with context
    let mut requested_lines = BTreeSet::new();
    for range in &ranges {
        match range {
            LineRange::Single(n) => {
                requested_lines.insert(*n);
                if let Some(ctx) = context {
                    let start = n.saturating_sub(ctx);
                    let end = n + ctx;
                    for i in start..=end {
                        if i > 0 && i <= file_lines.len() {
                            requested_lines.insert(i);
                        }
                    }
                }
            },
            LineRange::Range(start, end) => {
                for i in *start..=*end {
                    requested_lines.insert(i);
                }
                if let Some(ctx) = context {
                    let ctx_start = start.saturating_sub(ctx);
                    let ctx_end = end + ctx;
                    for i in ctx_start..=ctx_end {
                        if i > 0 && i <= file_lines.len() {
                            requested_lines.insert(i);
                        }
                    }
                }
            },
            LineRange::PlusCount(start, count) => {
                let end = start + count;
                for i in *start..=end {
                    requested_lines.insert(i);
                }
                if let Some(ctx) = context {
                    let ctx_start = start.saturating_sub(ctx);
                    let ctx_end = end + ctx;
                    for i in ctx_start..=ctx_end {
                        if i > 0 && i <= file_lines.len() {
                            requested_lines.insert(i);
                        }
                    }
                }
            },
        }
    }

    match format {
        OutputFormat::Text => {
            // Print lines with line numbers
            for line_num in requested_lines {
                if line_num == 0 || line_num > file_lines.len() {
                    continue;
                }
                let line_content = file_lines[line_num - 1];
                println!("{:>5} | {}", line_num.to_string().blue(), line_content);
            }
        },
        OutputFormat::Json => {
            // Output JSON array of line objects
            let lines_json: Vec<serde_json::Value> = requested_lines
                .into_iter()
                .filter(|&n| n > 0 && n <= file_lines.len())
                .map(|n| {
                    serde_json::json!({
                        "line": n,
                        "content": file_lines[n - 1]
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&lines_json)?);
        },
        OutputFormat::Jsonl => {
            // Output newline-delimited JSON
            for line_num in requested_lines {
                if line_num == 0 || line_num > file_lines.len() {
                    continue;
                }
                let line_obj = serde_json::json!({
                    "line": line_num,
                    "content": file_lines[line_num - 1]
                });
                println!("{}", serde_json::to_string(&line_obj)?);
            }
        },
    }

    Ok(())
}
