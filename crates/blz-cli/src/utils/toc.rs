use blz_core::TocEntry;

use super::parsing::parse_line_span;

fn find_entry_by_path<'a>(entries: &'a [TocEntry], target: &[String]) -> Option<&'a TocEntry> {
    for entry in entries {
        if entry.heading_path == target {
            return Some(entry);
        }
        if let Some(found) = find_entry_by_path(&entry.children, target) {
            return Some(found);
        }
    }
    None
}

/// Count all headings within a table of contents, including nested children.
pub fn count_headings(entries: &[TocEntry]) -> usize {
    entries
        .iter()
        .map(|entry| 1 + count_headings(&entry.children))
        .sum()
}

/// Find the line span for a heading path within a TOC.
#[must_use]
pub fn find_heading_span(entries: &[TocEntry], heading_path: &[String]) -> Option<(usize, usize)> {
    if heading_path.is_empty() {
        return None;
    }

    find_entry_by_path(entries, heading_path).and_then(|entry| parse_line_span(&entry.lines))
}

/// Find the most specific heading that contains the provided line number.
#[must_use]
pub fn find_heading_for_line(
    entries: &[TocEntry],
    line: usize,
) -> Option<(Vec<String>, (usize, usize))> {
    fn search(entries: &[TocEntry], line: usize) -> Option<(Vec<String>, (usize, usize))> {
        for entry in entries {
            if let Some((start, end)) = parse_line_span(&entry.lines) {
                if line >= start && line <= end {
                    if let Some(child) = search(&entry.children, line) {
                        return Some(child);
                    }
                    return Some((entry.heading_path.clone(), (start, end)));
                }
            }
        }
        None
    }

    search(entries, line)
}

#[must_use]
pub fn heading_level_from_line(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|&c| c == '#').count();
    if level == 0 {
        return None;
    }
    match trimmed.chars().nth(level) {
        Some(' ' | '\t') => Some(level),
        _ => None,
    }
}

/// Raw block slice extracted from a document.
#[derive(Debug, Clone)]
pub struct BlockSlice {
    /// Starting line number for the block (1-based).
    pub start: usize,
    /// Line numbers included in the block.
    pub line_numbers: Vec<usize>,
    /// Raw lines extracted from the document.
    pub lines: Vec<String>,
    /// Whether the block was truncated to a limit.
    pub truncated: bool,
}

/// Finalized block with cleaned content lines.
#[derive(Debug, Clone)]
pub struct FinalizedBlock {
    /// Line number of the heading.
    pub heading_line: usize,
    /// Line numbers for content lines (excluding heading).
    pub content_line_numbers: Vec<usize>,
    /// Content lines after trimming trailing blanks.
    pub content_lines: Vec<String>,
    /// Whether the block was truncated to a limit.
    pub truncated: bool,
}

#[must_use]
pub fn extract_block_slice(
    file_lines: &[String],
    start: usize,
    end: usize,
    max_lines: Option<usize>,
) -> Option<BlockSlice> {
    if start == 0 || start > file_lines.len() {
        return None;
    }

    let inclusive_end = end.min(file_lines.len()).max(start);
    let total_available = inclusive_end.saturating_sub(start) + 1;
    if total_available == 0 {
        return None;
    }

    let desired_total = max_lines
        .unwrap_or(total_available)
        .max(1)
        .min(total_available);

    let slice_end = start - 1 + desired_total;
    let lines = file_lines[start - 1..slice_end].to_vec();
    let line_numbers = (start..start + desired_total).collect::<Vec<_>>();
    let truncated = desired_total < total_available;
    Some(BlockSlice {
        start,
        line_numbers,
        lines,
        truncated,
    })
}

#[must_use]
pub fn finalize_block_slice(block: BlockSlice) -> FinalizedBlock {
    let heading_line = block.start;
    let truncated = block.truncated;
    let mut line_numbers = block.line_numbers;
    let mut lines = block.lines;

    while let Some(last_idx) = lines.len().checked_sub(1) {
        if line_numbers.get(last_idx) == Some(&heading_line) {
            break;
        }
        if lines[last_idx].trim().is_empty() {
            lines.pop();
            line_numbers.pop();
        } else {
            break;
        }
    }

    let content_line_numbers = line_numbers.first().map_or_else(Vec::new, |first| {
        if *first == heading_line {
            line_numbers[1..].to_vec()
        } else {
            line_numbers.clone()
        }
    });

    FinalizedBlock {
        heading_line,
        content_line_numbers,
        content_lines: lines,
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_nested_headings() {
        let toc = vec![TocEntry {
            heading_path: vec!["Root".into()],
            heading_path_display: Some(vec!["Root".into()]),
            heading_path_normalized: Some(vec!["root".into()]),
            lines: "1-10".into(),
            anchor: None,
            children: vec![TocEntry {
                heading_path: vec!["Root".into(), "Child".into()],
                heading_path_display: Some(vec!["Root".into(), "Child".into()]),
                heading_path_normalized: Some(vec!["root".into(), "child".into()]),
                lines: "2-5".into(),
                anchor: None,
                children: vec![TocEntry {
                    heading_path: vec!["Root".into(), "Child".into(), "Grandchild".into()],
                    heading_path_display: Some(vec![
                        "Root".into(),
                        "Child".into(),
                        "Grandchild".into(),
                    ]),
                    heading_path_normalized: Some(vec![
                        "root".into(),
                        "child".into(),
                        "grandchild".into(),
                    ]),
                    lines: "3-4".into(),
                    anchor: None,
                    children: Vec::new(),
                }],
            }],
        }];

        assert_eq!(count_headings(&toc), 3);
    }

    #[test]
    fn empty_toc_returns_zero() {
        let toc: Vec<TocEntry> = Vec::new();
        assert_eq!(count_headings(&toc), 0);
    }
}
