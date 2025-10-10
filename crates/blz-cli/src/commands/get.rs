//! Get command implementation for retrieving specific lines from sources

use anyhow::{Context, Result};
use blz_core::Storage;
use colored::Colorize;
use std::collections::BTreeSet;

use crate::output::OutputFormat;
use crate::utils::parsing::{LineRange, parse_line_ranges};
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

fn determine_fallback_bounds(
    ranges: &[LineRange],
    target_line: usize,
    file_len: usize,
) -> (usize, usize) {
    ranges
        .first()
        .map_or((target_line, file_len), |range| match range {
            LineRange::Single(n) => (*n, *n),
            LineRange::Range(start, end) => (*start, *end),
            LineRange::PlusCount(start, count) => {
                let end = start.saturating_add(count.saturating_sub(1));
                (*start, end)
            },
        })
}

fn adjust_span_for_heading(file_lines: &[String], span: &mut (usize, usize)) {
    if span.0 <= 1 {
        return;
    }

    let start_idx = span.0.saturating_sub(1);
    if start_idx < file_lines.len() && heading_level_from_line(&file_lines[start_idx]).is_some() {
        return;
    }

    let mut idx = start_idx;
    while idx > 0 {
        idx -= 1;
        let line = &file_lines[idx];
        if heading_level_from_line(line).is_some() {
            span.0 = idx + 1;
            break;
        }
        if !line.trim().is_empty() {
            break;
        }
    }
}

fn strip_heading_if_present(
    line_numbers: &mut Vec<usize>,
    content_lines: &[String],
) -> Option<usize> {
    if let (Some(&first_line), Some(first_content)) = (line_numbers.first(), content_lines.first())
    {
        if heading_level_from_line(first_content).is_some() {
            line_numbers.remove(0);
            return Some(first_line);
        }
    }
    None
}

fn compute_block_result(
    storage: &Storage,
    canonical: &str,
    file_lines: &[String],
    ranges: &[LineRange],
    max_block_lines: Option<usize>,
) -> BlockResult {
    let target_line = ranges.first().map_or(1, |range| match range {
        LineRange::Single(n) => *n,
        LineRange::Range(start, _) | LineRange::PlusCount(start, _) => *start,
    });

    let llms = storage.load_llms_json(canonical).ok();
    let file_len = file_lines.len();
    let (fallback_start, user_end) = determine_fallback_bounds(ranges, target_line, file_len);
    let fallback_span = (fallback_start, file_len);

    let toc_span = llms
        .as_ref()
        .and_then(|doc| find_heading_for_line(&doc.toc, target_line).map(|(_, span)| span));
    let using_toc_span = toc_span.is_some();
    let mut span = toc_span.unwrap_or(fallback_span);
    adjust_span_for_heading(file_lines, &mut span);
    let (start, end) = span;

    if file_lines.is_empty() {
        return BlockResult {
            heading_line: 0,
            line_numbers: Vec::new(),
            render_lines: format!("{start}-{start}"),
            truncated: false,
            content_lines: Vec::new(),
        };
    }

    let safe_start = start.max(1).min(file_len);
    let safe_end = end.max(safe_start).min(file_len);
    let fallback_end = user_end.max(safe_start).min(file_len);

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

    let block_line_numbers = block.line_numbers.clone();
    let finalized = finalize_block_slice(block);
    let heading_candidate = finalized.heading_line;
    let truncated = finalized.truncated;
    let mut line_numbers = if using_toc_span {
        finalized.content_line_numbers
    } else {
        block_line_numbers
    };
    let mut content_lines = finalized.content_lines;

    if !using_toc_span && line_numbers.len() > content_lines.len() {
        line_numbers.truncate(content_lines.len());
    }

    let mut heading_line = if using_toc_span { heading_candidate } else { 0 };

    if !using_toc_span {
        if let Some(heading) = strip_heading_if_present(&mut line_numbers, &content_lines) {
            heading_line = heading;
        }
        if heading_line == 0 {
            line_numbers.retain(|line| *line <= user_end);
            if line_numbers.len() < content_lines.len() {
                content_lines.truncate(line_numbers.len());
            }
        }
    }

    let render_start = if using_toc_span {
        heading_line
    } else {
        safe_start
    };
    let render_end = line_numbers.last().copied().unwrap_or(render_start);

    BlockResult {
        heading_line,
        line_numbers,
        render_lines: format!("{render_start}-{render_end}"),
        truncated,
        content_lines,
    }
}

fn gather_requested_lines(
    ranges: &[LineRange],
    before_context: usize,
    after_context: usize,
    file_len: usize,
) -> Vec<usize> {
    let mut requested_lines = BTreeSet::new();

    for range in ranges {
        match range {
            LineRange::Single(n) => {
                requested_lines.insert(*n);
                let start = n.saturating_sub(before_context);
                let end = n + after_context;
                for i in start..=end {
                    if i > 0 && i <= file_len {
                        requested_lines.insert(i);
                    }
                }
            },
            LineRange::Range(start, end) => {
                for i in *start..=*end {
                    requested_lines.insert(i);
                }
                let ctx_start = start.saturating_sub(before_context);
                let ctx_end = end + after_context;
                for i in ctx_start..=ctx_end {
                    if i > 0 && i <= file_len {
                        requested_lines.insert(i);
                    }
                }
            },
            LineRange::PlusCount(start, count) => {
                let end = start.saturating_add(count.saturating_sub(1));
                for i in *start..=end {
                    requested_lines.insert(i);
                }
                let ctx_start = start.saturating_sub(before_context);
                let ctx_end = end + after_context;
                for i in ctx_start..=ctx_end {
                    if i > 0 && i <= file_len {
                        requested_lines.insert(i);
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
    context_mode: Option<&crate::cli::ContextMode>,
    block: bool,
    max_block_lines: Option<usize>,
    format: OutputFormat,
    copy: bool,
) -> Result<()> {
    // Convert ContextMode to before/after context and block flag
    let (before_context, after_context, block) = match context_mode {
        Some(crate::cli::ContextMode::All) => (0, 0, true),
        Some(crate::cli::ContextMode::Symmetric(n)) => (*n, *n, false),
        Some(crate::cli::ContextMode::Asymmetric { before, after }) => (*before, *after, false),
        None => (0, 0, block),
    };
    let storage = Storage::new()?;

    // Resolve metadata alias to canonical if needed
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .map_or_else(|| alias.to_string(), |c| c);

    if !storage.exists(&canonical) {
        let available = storage.list_sources();
        if available.is_empty() {
            anyhow::bail!(
                "Source '{alias}' not found.\n\
                 No sources available. Use 'blz lookup <name>' or 'blz add <alias> <url>' to add one."
            );
        }
        let preview = available.iter().take(8).cloned().collect::<Vec<_>>();
        let preview_str = if available.len() > preview.len() {
            format!(
                "{} (+{} more)",
                preview.join(", "),
                available.len() - preview.len()
            )
        } else {
            preview.join(", ")
        };
        anyhow::bail!(
            "Source '{alias}' not found.\n\
             Available: {preview_str}\n\
             Hint: 'blz list' to see all, or 'blz lookup <name>' to search registries."
        );
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

    // Validate that requested ranges are within bounds
    let max_line = file_lines.len();
    let all_out_of_range = ranges.iter().all(|range| {
        let (start, _end) = match range {
            LineRange::Single(n) => (*n, *n),
            LineRange::Range(s, e) => (*s, *e),
            LineRange::PlusCount(s, count) => (*s, s.saturating_add(count.saturating_sub(1))),
        };
        start > max_line
    });

    if all_out_of_range {
        let first_requested = match ranges.first() {
            Some(LineRange::Single(n)) => *n,
            Some(LineRange::Range(s, _) | LineRange::PlusCount(s, _)) => *s,
            None => 1,
        };
        anyhow::bail!(
            "Line range starts at line {first_requested}, but source '{canonical}' only has {max_line} lines.\n\
             Use 'blz info {canonical}' to see source details."
        );
    }

    // Collect all requested line numbers (1-based) and expand with context
    let block_result = if block {
        Some(compute_block_result(
            &storage,
            &canonical,
            &file_lines,
            &ranges,
            max_block_lines,
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
            let line_numbers =
                gather_requested_lines(&ranges, before_context, after_context, file_lines.len());
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
