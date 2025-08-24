//! Get command implementation for retrieving specific lines from sources

use anyhow::Result;
use blz_core::Storage;
use colored::Colorize;
use std::collections::BTreeSet;

use crate::utils::parsing::{parse_line_ranges, LineRange};

/// Execute the get command to retrieve specific lines from a source
pub async fn execute(alias: &str, lines: &str, context: Option<usize>) -> Result<()> {
    let storage = Storage::new()?;

    if !storage.exists(alias) {
        println!("Source '{alias}' not found");
        return Ok(());
    }

    let content = storage.load_llms_txt(alias)?;
    let all_lines: Vec<&str> = content.lines().collect();

    let line_numbers = collect_line_numbers(lines, context, all_lines.len())?;
    display_lines(&line_numbers, &all_lines);

    Ok(())
}

fn collect_line_numbers(
    lines: &str,
    context: Option<usize>,
    total_lines: usize,
) -> Result<BTreeSet<usize>> {
    let ranges = parse_line_ranges(lines)?;
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
