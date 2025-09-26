//! Get command implementation for retrieving specific lines from sources

use anyhow::Result;
use blz_core::Storage;
use colored::Colorize;
use std::collections::BTreeSet;

use crate::commands::FlavorMode;
use crate::output::OutputFormat;
use crate::utils::flavor::resolve_flavor;
use crate::utils::parsing::{LineRange, parse_line_ranges};

/// Execute the get command to retrieve specific lines from a source
pub async fn execute(
    alias: &str,
    lines: &str,
    context: Option<usize>,
    flavor: FlavorMode,
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

    // Determine which flavor to load based on the mode
    let effective_flavor = match flavor {
        FlavorMode::Current => resolve_flavor(&storage, &canonical)?,
        FlavorMode::Auto => {
            // Auto prefers full if available
            let available = storage.available_flavors(&canonical)?;
            if available.iter().any(|f| f == "llms-full") {
                "llms-full".to_string()
            } else {
                resolve_flavor(&storage, &canonical)?
            }
        },
        FlavorMode::Full => {
            let available = storage.available_flavors(&canonical)?;
            if available.iter().any(|f| f == "llms-full") {
                "llms-full".to_string()
            } else {
                // Fall back to resolved default if full not available
                resolve_flavor(&storage, &canonical)?
            }
        },
        FlavorMode::Txt => {
            let available = storage.available_flavors(&canonical)?;
            if available.iter().any(|f| f == "llms") {
                "llms".to_string()
            } else {
                // Fall back to resolved default if txt not available
                resolve_flavor(&storage, &canonical)?
            }
        },
    };

    // Load the appropriate flavor file
    let file_path = storage.flavor_file_path(&canonical, &effective_flavor)?;
    let file_content = std::fs::read_to_string(&file_path)?;
    let all_lines: Vec<&str> = file_content.lines().collect();

    match format {
        OutputFormat::Text => {
            let line_numbers = collect_line_numbers(lines, context, all_lines.len())?;
            display_lines(&line_numbers, &all_lines);
            Ok(())
        },
        OutputFormat::Json | OutputFormat::Jsonl => {
            // Build content for requested ranges and context
            let selected = collect_line_numbers(lines, context, all_lines.len())?;
            let mut body = String::new();
            for (i, &ln) in selected.iter().enumerate() {
                if ln == 0 || ln > all_lines.len() {
                    continue;
                }
                if i > 0 {
                    body.push('\n');
                }
                body.push_str(all_lines[ln - 1]);
            }
            let obj = serde_json::json!({
                "alias": alias,
                "source": canonical,
                "flavor": effective_flavor,
                "lines": lines,
                "context": context,
                "lineNumbers": selected.iter().copied().collect::<Vec<_>>(),
                "content": body,
            });
            if matches!(format, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&obj)?);
            } else {
                println!("{}", serde_json::to_string(&obj)?);
            }
            Ok(())
        },
    }
}

/// Execute get command with a pre-resolved flavor string
/// This avoids re-resolution and ensures we use the exact flavor already determined
pub async fn execute_with_flavor(
    alias: &str,
    canonical: &str,
    lines: &str,
    context: Option<usize>,
    flavor: &str,
    format: OutputFormat,
) -> Result<()> {
    let storage = Storage::new()?;

    // Load the specific flavor file
    let file_path = storage.flavor_file_path(canonical, flavor)?;
    let file_content = std::fs::read_to_string(&file_path)?;
    let all_lines: Vec<&str> = file_content.lines().collect();

    match format {
        OutputFormat::Text => {
            let line_numbers = collect_line_numbers(lines, context, all_lines.len())?;
            display_lines(&line_numbers, &all_lines);
            Ok(())
        },
        OutputFormat::Json | OutputFormat::Jsonl => {
            // Build content for requested ranges and context
            let selected = collect_line_numbers(lines, context, all_lines.len())?;
            let mut body = String::new();
            for (i, &ln) in selected.iter().enumerate() {
                if ln == 0 || ln > all_lines.len() {
                    continue;
                }
                if i > 0 {
                    body.push('\n');
                }
                body.push_str(all_lines[ln - 1]);
            }
            let obj = serde_json::json!({
                "alias": alias,
                "source": canonical,
                "flavor": flavor,
                "lines": lines,
                "context": context,
                "lineNumbers": selected.iter().copied().collect::<Vec<_>>(),
                "content": body,
            });
            if matches!(format, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&obj)?);
            } else {
                println!("{}", serde_json::to_string(&obj)?);
            }
            Ok(())
        },
    }
}

fn collect_line_numbers(
    lines: &str,
    context: Option<usize>,
    total_lines: usize,
) -> Result<BTreeSet<usize>> {
    let ranges = parse_line_ranges(lines).map_err(|_| {
        anyhow::anyhow!("Invalid --lines format. Examples: '120-142', '36+20', '36:43,320:350'.")
    })?;
    let context_lines = context.unwrap_or(0);
    let mut all_line_numbers = BTreeSet::new();

    for range in ranges {
        match range {
            LineRange::Single(line) => {
                add_with_context(&mut all_line_numbers, line, context_lines, total_lines);
            },
            LineRange::Range(start, end) => {
                add_range_with_context(
                    &mut all_line_numbers,
                    start,
                    end,
                    context_lines,
                    total_lines,
                );
            },
            LineRange::PlusCount(start, count) => {
                let end = start + count - 1;
                add_range_with_context(
                    &mut all_line_numbers,
                    start,
                    end,
                    context_lines,
                    total_lines,
                );
            },
        }
    }

    if all_line_numbers.is_empty() {
        return Err(anyhow::anyhow!("No valid line ranges found"));
    }

    Ok(all_line_numbers)
}

fn add_with_context(
    line_numbers: &mut BTreeSet<usize>,
    line: usize,
    context_lines: usize,
    total_lines: usize,
) {
    let start = line.saturating_sub(context_lines + 1);
    let end = (line + context_lines).min(total_lines);

    for i in start..end {
        line_numbers.insert(i + 1);
    }
}

fn add_range_with_context(
    line_numbers: &mut BTreeSet<usize>,
    start: usize,
    end: usize,
    context_lines: usize,
    total_lines: usize,
) {
    let actual_start = start.saturating_sub(context_lines + 1);
    let actual_end = (end + context_lines).min(total_lines);

    for i in actual_start..actual_end {
        line_numbers.insert(i + 1);
    }
}

fn display_lines(line_numbers: &BTreeSet<usize>, all_lines: &[&str]) {
    let mut prev_line = 0;

    for &line_num in line_numbers {
        if line_num == 0 || line_num > all_lines.len() {
            continue;
        }

        // Add separator for gaps > 1
        if prev_line > 0 && line_num > prev_line + 1 {
            println!("{}", "     ┈".bright_black());
        }

        println!("{:4} │ {}", line_num, all_lines[line_num - 1]);
        prev_line = line_num;
    }
}
