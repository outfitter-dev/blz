//! Get command implementation for retrieving specific lines from sources

use anyhow::{Context, Result};
use blz_core::Storage;
use colored::Colorize;
use std::collections::BTreeSet;

use crate::output::OutputFormat;
use crate::utils::parsing::{LineRange, parse_line_ranges, parse_line_span};
use crate::utils::toc::{
    BlockSlice, extract_block_slice, finalize_block_slice, find_heading_for_line,
    heading_level_from_line,
};

struct BlockResult {
    heading_line: usize,
    line_numbers: Vec<usize>,
    render_lines: String,
    truncated: bool,
    content_lines: Vec<String>,
}

fn compute_block_result(
    storage: &Storage,
    canonical: &str,
    file_lines: &[String],
    ranges: &[LineRange],
    max_block_lines: Option<usize>,
    line_spec: &str,
) -> BlockResult {
    let target_line = ranges.first().map_or(1, |range| match range {
        LineRange::Single(n) => *n,
        LineRange::Range(start, _) | LineRange::PlusCount(start, _) => *start,
    });

    let llms = storage.load_llms_json(canonical).ok();
    let (start, end) = llms
        .as_ref()
        .and_then(|doc| find_heading_for_line(&doc.toc, target_line).map(|(_, span)| span))
        .or_else(|| parse_line_span(line_spec))
        .unwrap_or((target_line, target_line));

    if file_lines.is_empty() {
        return BlockResult {
            heading_line: 0,
            line_numbers: Vec::new(),
            render_lines: format!("{start}-{start}"),
            truncated: false,
            content_lines: Vec::new(),
        };
    }

    let safe_start = start.max(1).min(file_lines.len());
    let safe_end = end.max(safe_start);
    let fallback_end = safe_start;

    let adjusted_max = max_block_lines.map(|limit| limit.saturating_add(1));
    let mut block = extract_block_slice(file_lines, safe_start, safe_end, adjusted_max)
        .or_else(|| extract_block_slice(file_lines, safe_start, fallback_end, adjusted_max))
        .unwrap_or_else(|| BlockSlice {
            start: safe_start,
            line_numbers: vec![safe_start],
            lines: vec![file_lines[safe_start - 1].clone()],
            truncated: false,
        });

    if let Some(level) = heading_level_from_line(&file_lines[safe_start - 1]) {
        let mut inferred_end = safe_start;
        for idx in (safe_start + 1)..=file_lines.len() {
            if let Some(next_level) = heading_level_from_line(&file_lines[idx - 1]) {
                if next_level <= level {
                    break;
                }
            }
            inferred_end = idx;
        }

        if inferred_end > safe_start {
            if let Some(extended) =
                extract_block_slice(file_lines, safe_start, inferred_end, adjusted_max)
            {
                block = extended;
            }
        }
    }

    let finalized = finalize_block_slice(block);
    let render_end = finalized
        .content_line_numbers
        .last()
        .copied()
        .unwrap_or(finalized.heading_line);

    BlockResult {
        heading_line: finalized.heading_line,
        line_numbers: finalized.content_line_numbers,
        render_lines: format!(
            "{start}-{end}",
            start = finalized.heading_line,
            end = render_end
        ),
        truncated: finalized.truncated,
        content_lines: finalized.content_lines,
    }
}

fn gather_requested_lines(
    ranges: &[LineRange],
    context: Option<usize>,
    file_len: usize,
) -> Vec<usize> {
    let mut requested_lines = BTreeSet::new();

    for range in ranges {
        match range {
            LineRange::Single(n) => {
                requested_lines.insert(*n);
                if let Some(ctx) = context {
                    let start = n.saturating_sub(ctx);
                    let end = n + ctx;
                    for i in start..=end {
                        if i > 0 && i <= file_len {
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
                        if i > 0 && i <= file_len {
                            requested_lines.insert(i);
                        }
                    }
                }
            },
            LineRange::PlusCount(start, count) => {
                let end = start.saturating_add(count.saturating_sub(1));
                for i in *start..=end {
                    requested_lines.insert(i);
                }
                if let Some(ctx) = context {
                    let ctx_start = start.saturating_sub(ctx);
                    let ctx_end = end + ctx;
                    for i in ctx_start..=ctx_end {
                        if i > 0 && i <= file_len {
                            requested_lines.insert(i);
                        }
                    }
                }
            },
        }
    }

    requested_lines
        .into_iter()
        .filter(|&n| n > 0 && n <= file_len)
        .collect()
}

/// Execute the get command to retrieve specific lines from a source
#[allow(clippy::too_many_lines)]
pub async fn execute(
    alias: &str,
    lines: &str,
    context: Option<usize>,
    block: bool,
    max_block_lines: Option<usize>,
    format: OutputFormat,
    copy: bool,
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

    let file_lines: Vec<String> = file_content
        .lines()
        .map(std::string::ToString::to_string)
        .collect();
    let ranges =
        parse_line_ranges(lines).map_err(|err| anyhow::anyhow!("Invalid --lines format: {err}"))?;

    // Collect all requested line numbers (1-based) and expand with context
    let block_result = if block {
        Some(compute_block_result(
            &storage,
            &canonical,
            &file_lines,
            &ranges,
            max_block_lines,
            lines,
        ))
    } else {
        None
    };

    let (heading_line, line_numbers, render_lines, truncated, mut content_lines) =
        if let Some(result) = block_result {
            (
                result.heading_line,
                result.line_numbers,
                result.render_lines,
                result.truncated,
                result.content_lines,
            )
        } else {
            let line_numbers = gather_requested_lines(&ranges, context, file_lines.len());
            (0, line_numbers, lines.to_string(), false, Vec::new())
        };

    if !block {
        content_lines = line_numbers
            .iter()
            .filter_map(|&line_idx| file_lines.get(line_idx - 1).cloned())
            .collect();
    }

    match format {
        OutputFormat::Text => {
            // Print lines with line numbers
            if block && heading_line > 0 && heading_line <= file_lines.len() {
                let heading_str = &file_lines[heading_line - 1];
                println!("{:>5} | {}", heading_line.to_string().blue(), heading_str);
            }
            for &line_num in &line_numbers {
                if block && line_num == heading_line {
                    continue;
                }
                if line_num == 0 || line_num > file_lines.len() {
                    continue;
                }
                let line_content = &file_lines[line_num - 1];
                println!("{:>5} | {}", line_num.to_string().blue(), line_content);
            }
        },
        OutputFormat::Json => {
            let joined_content = content_lines.join("\n");
            let mut response = serde_json::json!({
                "alias": alias,
                "source": canonical,
                "lines": render_lines,
                "lineNumbers": line_numbers,
                "content": joined_content,
            });
            if block && truncated {
                if let serde_json::Value::Object(ref mut map) = response {
                    map.insert("truncated".to_string(), serde_json::Value::Bool(true));
                }
            }
            println!("{}", serde_json::to_string_pretty(&response)?);
        },
        OutputFormat::Jsonl => {
            let joined_content = content_lines.join("\n");
            let mut response = serde_json::json!({
                "alias": alias,
                "source": canonical,
                "lines": render_lines,
                "lineNumbers": line_numbers,
                "content": joined_content,
            });
            if block && truncated {
                if let serde_json::Value::Object(ref mut map) = response {
                    map.insert("truncated".to_string(), serde_json::Value::Bool(true));
                }
            }
            println!("{}", serde_json::to_string(&response)?);
        },
        OutputFormat::Raw => {
            // Raw format: just print the content, no line numbers or metadata
            if block && heading_line > 0 && heading_line <= file_lines.len() {
                let heading_str = &file_lines[heading_line - 1];
                println!("{heading_str}");
            }
            for &line_num in &line_numbers {
                if block && line_num == heading_line {
                    continue;
                }
                if line_num == 0 || line_num > file_lines.len() {
                    continue;
                }
                let line_content = &file_lines[line_num - 1];
                println!("{line_content}");
            }
        },
    }

    // Copy to clipboard if --copy flag was set
    if copy && !line_numbers.is_empty() {
        use crate::utils::clipboard;

        let copied_content = content_lines.join("\n");

        clipboard::copy_to_clipboard(&copied_content)
            .context("Failed to copy content to clipboard")?;
    }

    Ok(())
}
