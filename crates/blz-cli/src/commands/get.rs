//! Get command implementation for retrieving specific lines from sources

use anyhow::{Context, Result};
use blz_core::Storage;
use colored::Colorize;
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::num::NonZeroUsize;
use std::time::Instant;

pub mod json_contract;
use self::json_contract::{
    ExecutionMetadata, GetResponse, SingleSnippet, SnippetPayload, SnippetRange, SnippetRanges,
    SnippetRequest,
};

use crate::output::OutputFormat;
use crate::utils::parsing::{LineRange, parse_line_ranges};
use crate::utils::toc::{
    BlockSlice, extract_block_slice, finalize_block_slice, find_heading_for_line,
    heading_level_from_line,
};

struct BlockResult {
    heading_line: usize,
    line_numbers: Vec<usize>,
    content_lines: Vec<String>,
    truncated: bool,
}

/// Parsed positional input for the `get` command.
#[derive(Debug, Clone)]
pub struct RequestSpec {
    pub alias: String,
    pub line_expression: String,
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
            content_lines: Vec::new(),
            truncated: false,
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

    BlockResult {
        heading_line,
        line_numbers,
        content_lines,
        truncated,
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

struct ProcessedRequest {
    alias: String,
    canonical: String,
    heading: Option<(usize, String)>,
    lines_with_content: Vec<(usize, String)>,
    snippet_ranges: Vec<SnippetRange>,
    checksum: Option<String>,
    file_len: usize,
    truncated: bool,
}

#[allow(clippy::missing_const_for_fn)]
fn should_skip_line(
    line_num: usize,
    heading: Option<&(usize, String)>,
    file_len: usize,
    block_mode: bool,
) -> bool {
    if block_mode {
        if let Some((heading_line, _)) = heading {
            if line_num == *heading_line {
                return true;
            }
        }
    }

    line_num == 0 || line_num > file_len
}

fn range_bounds(range: &LineRange, file_len: usize) -> (usize, usize) {
    let capped_len = file_len.max(1);
    match range {
        LineRange::Single(n) => {
            let value = (*n).clamp(1, capped_len);
            (value, value)
        },
        LineRange::Range(start, end) => {
            let start = (*start).clamp(1, capped_len);
            let end = (*end).clamp(start, capped_len);
            (start, end)
        },
        LineRange::PlusCount(start, count) => {
            let start = (*start).clamp(1, capped_len);
            let raw_end = start.saturating_add(count.saturating_sub(1));
            let end = raw_end.clamp(start, capped_len);
            (start, end)
        },
    }
}

#[allow(clippy::missing_const_for_fn)]
fn nz(value: usize) -> Result<NonZeroUsize> {
    NonZeroUsize::new(value).context("line numbers must be at least 1")
}

#[allow(clippy::too_many_lines)]
fn process_single_request(
    storage: &Storage,
    spec: &RequestSpec,
    before_context: usize,
    after_context: usize,
    block_mode: bool,
    max_block_lines: Option<usize>,
) -> Result<ProcessedRequest> {
    let alias = spec.alias.trim();
    if alias.is_empty() {
        anyhow::bail!("Alias cannot be empty. Use format: alias[:ranges]");
    }
    let alias_string = alias.to_string();

    let canonical = crate::utils::resolver::resolve_source(storage, alias)?
        .unwrap_or_else(|| alias_string.clone());

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

    let ranges = parse_line_ranges(&spec.line_expression)
        .map_err(|err| anyhow::anyhow!("Invalid line specification for '{alias}': {err}"))?;

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

    let block_result = if block_mode {
        Some(compute_block_result(
            storage,
            &canonical,
            &file_lines,
            &ranges,
            max_block_lines,
        ))
    } else {
        None
    };

    let (heading_line, line_numbers, mut content_lines, truncated_flag) =
        if let Some(result) = block_result {
            (
                result.heading_line,
                result.line_numbers,
                result.content_lines,
                result.truncated,
            )
        } else {
            let line_numbers =
                gather_requested_lines(&ranges, before_context, after_context, file_lines.len());
            (0, line_numbers, Vec::new(), false)
        };

    if !block_mode {
        content_lines = line_numbers
            .iter()
            .filter_map(|&line_idx| file_lines.get(line_idx.saturating_sub(1)).cloned())
            .collect();
    }

    let heading = if block_mode && heading_line > 0 && heading_line <= file_lines.len() {
        Some((heading_line, file_lines[heading_line - 1].clone()))
    } else {
        None
    };

    let mut lines_with_content = Vec::new();
    for (idx, &line_num) in line_numbers.iter().enumerate() {
        if line_num == 0 || line_num > file_lines.len() {
            continue;
        }
        let content = content_lines
            .get(idx)
            .cloned()
            .unwrap_or_else(|| file_lines[line_num - 1].clone());
        lines_with_content.push((line_num, content));
    }

    let snippet_ranges = if block_mode {
        let line_start = heading
            .as_ref()
            .map(|(line, _)| *line)
            .or_else(|| lines_with_content.first().map(|(line, _)| *line))
            .unwrap_or(1);
        let line_end = lines_with_content
            .last()
            .map_or(line_start, |(line, _)| *line);
        let snippet = if line_start <= line_end && line_end <= file_lines.len() {
            file_lines[line_start - 1..line_end].join("\n")
        } else {
            lines_with_content
                .iter()
                .map(|(_, text)| text.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };

        let line_start = nz(line_start)?;
        let line_end = nz(line_end)?;
        vec![SnippetRange::try_new(line_start, line_end, snippet).map_err(anyhow::Error::from)?]
    } else {
        let safe_len = file_lines.len().max(1);
        ranges
            .iter()
            .map(|range| {
                let (base_start, base_end) = range_bounds(range, safe_len);
                let context_start = base_start.saturating_sub(before_context).max(1);
                let context_end = (base_end + after_context).min(safe_len);
                let snippet = if context_start <= context_end {
                    file_lines[context_start - 1..context_end.min(file_lines.len())].join("\n")
                } else {
                    String::new()
                };
                let line_start = nz(context_start)?;
                let line_end = nz(context_end)?;
                SnippetRange::try_new(line_start, line_end, snippet).map_err(anyhow::Error::from)
            })
            .collect::<Result<Vec<_>>>()?
    };

    let checksum = match storage.load_source_metadata(&canonical) {
        Ok(Some(metadata)) => Some(metadata.sha256),
        _ => None,
    };

    Ok(ProcessedRequest {
        alias: alias_string,
        canonical,
        heading,
        lines_with_content,
        snippet_ranges,
        checksum,
        file_len: file_lines.len(),
        truncated: truncated_flag,
    })
}

/// Execute the get command to retrieve specific lines from a source
#[allow(clippy::too_many_lines)]
#[allow(clippy::cognitive_complexity)]
pub async fn execute(
    specs: &[RequestSpec],
    context_mode: Option<&crate::cli::ContextMode>,
    block: bool,
    max_block_lines: Option<usize>,
    format: OutputFormat,
    copy: bool,
) -> Result<()> {
    if specs.is_empty() {
        anyhow::bail!("At least one alias is required.");
    }

    let start = Instant::now();

    // Convert ContextMode to before/after context and block flag
    let (before_context, after_context, block_mode) = match context_mode {
        Some(crate::cli::ContextMode::All) => (0, 0, true),
        Some(crate::cli::ContextMode::Symmetric(n)) => (*n, *n, false),
        Some(crate::cli::ContextMode::Asymmetric { before, after }) => (*before, *after, false),
        None => (0, 0, block),
    };
    let storage = Storage::new()?;

    let mut processed = Vec::with_capacity(specs.len());
    let mut clipboard_segments = Vec::new();

    for spec in specs {
        let result = process_single_request(
            &storage,
            spec,
            before_context,
            after_context,
            block_mode,
            max_block_lines,
        )?;
        if copy {
            let clip = result
                .lines_with_content
                .iter()
                .map(|(_, line)| line.clone())
                .collect::<Vec<_>>()
                .join("\n");
            if !clip.is_empty() {
                clipboard_segments.push(clip);
            }
        }
        processed.push(result);
    }

    let context_applied = if block_mode {
        None
    } else if before_context > 0 || after_context > 0 {
        Some(before_context.max(after_context))
    } else {
        None
    };

    let response_payload = if matches!(format, OutputFormat::Json | OutputFormat::Jsonl) {
        let requests = processed
            .iter()
            .map(|result| {
                let payload = match result.snippet_ranges.as_slice() {
                    [] => None,
                    [range] => Some(SnippetPayload::Single(SingleSnippet {
                        snippet: range.snippet.clone(),
                        line_start: range.line_start,
                        line_end: range.line_end,
                    })),
                    ranges => Some(SnippetPayload::Multi(SnippetRanges {
                        ranges: ranges.to_vec(),
                    })),
                };

                SnippetRequest {
                    alias: result.alias.clone(),
                    source: result.canonical.clone(),
                    payload,
                    checksum: result.checksum.clone(),
                    context_applied,
                    truncated: result.truncated.then_some(true),
                }
            })
            .collect();

        let execution_time_ms = u64::try_from(start.elapsed().as_millis()).ok();

        Some(GetResponse {
            requests,
            metadata: ExecutionMetadata {
                execution_time_ms,
                total_sources: Some(specs.len()),
            },
        })
    } else {
        None
    };

    match format {
        OutputFormat::Text => {
            for (idx, result) in processed.iter().enumerate() {
                if idx > 0 {
                    println!();
                }
                if block_mode {
                    if let Some((line_num, heading)) = &result.heading {
                        println!("{:>5} | {}", line_num.to_string().blue(), heading);
                    }
                }
                for (line_num, content) in &result.lines_with_content {
                    if should_skip_line(
                        *line_num,
                        result.heading.as_ref(),
                        result.file_len,
                        block_mode,
                    ) {
                        continue;
                    }
                    println!("{:>5} | {}", line_num.to_string().blue(), content);
                }
            }
        },
        OutputFormat::Raw => {
            for (idx, result) in processed.iter().enumerate() {
                if idx > 0 {
                    println!();
                }
                if block_mode {
                    if let Some((_, heading)) = &result.heading {
                        println!("{heading}");
                    }
                }
                for (line_num, content) in &result.lines_with_content {
                    if should_skip_line(
                        *line_num,
                        result.heading.as_ref(),
                        result.file_len,
                        block_mode,
                    ) {
                        continue;
                    }
                    println!("{content}");
                }
            }
        },
        OutputFormat::Json => {
            if let Some(response) = response_payload {
                println!("{}", serde_json::to_string_pretty(&response)?);
            }
        },
        OutputFormat::Jsonl => {
            if let Some(response) = response_payload {
                println!("{}", serde_json::to_string(&response)?);
            }
        },
    }

    if copy && !clipboard_segments.is_empty() {
        use crate::utils::clipboard;

        let payload = clipboard_segments.join("\n\n");
        clipboard::copy_to_clipboard(&payload).context("Failed to copy content to clipboard")?;
    }

    Ok(())
}
