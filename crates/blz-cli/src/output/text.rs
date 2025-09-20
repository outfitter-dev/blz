//! Text output formatting

use super::formatter::FormatParams;
use blz_core::{SearchHit, Storage};
use colored::Colorize;
use std::collections::{BTreeSet, HashMap};

use crate::utils::formatting::{format_heading_path, get_alias_color, terminal_width};

const PATH_PREFIX_WIDTH: usize = 5; // "  in "
const DEFAULT_TERMINAL_WIDTH: usize = 80;

pub struct TextFormatter;

impl TextFormatter {
    /// Format search results in the brief, colorized layout
    #[allow(clippy::too_many_lines)]
    pub fn format_search_results(params: &FormatParams) {
        if params.hits.is_empty() {
            println!("No results found for '{}'", params.query);
            return;
        }

        let mut content_cache: HashMap<String, Vec<String>> = HashMap::new();
        let storage = Storage::new().ok();

        // Assign stable colors to aliases on this page (sorted by alias for determinism)
        let mut alias_colors: HashMap<String, usize> = HashMap::new();
        let mut sorted_aliases: Vec<String> = params
            .hits
            .iter()
            .map(|h| h.alias.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        sorted_aliases.sort();
        for (idx, alias) in sorted_aliases.iter().enumerate() {
            alias_colors.insert(alias.clone(), idx);
        }
        let mut color_index = alias_colors.len();

        // Group contiguous hits with the same alias + heading path
        let mut groups: Vec<(String, Vec<String>, Vec<&SearchHit>)> = Vec::new();
        for hit in params.hits {
            if let Some((last_alias, last_path, grouped_hits)) = groups.last_mut() {
                if *last_alias == hit.alias && *last_path == hit.heading_path {
                    grouped_hits.push(hit);
                    continue;
                }
            }
            groups.push((hit.alias.clone(), hit.heading_path.clone(), vec![hit]));
        }

        let mut rendered_groups: Vec<String> = Vec::with_capacity(groups.len());
        let term_width = terminal_width().unwrap_or(DEFAULT_TERMINAL_WIDTH);
        let path_width = term_width.saturating_sub(PATH_PREFIX_WIDTH);

        for (group_idx, (alias, heading_path, hits)) in groups.iter().enumerate() {
            let global_index = params.start_idx + group_idx + 1;
            let alias_idx = *alias_colors.entry(alias.clone()).or_insert_with(|| {
                let idx = color_index;
                color_index = color_index.saturating_add(1);
                idx
            });
            let alias_colored = get_alias_color(alias, alias_idx);
            let first = hits[0];

            let score_formatted = format_score_value(first.score, params.score_precision);
            let score_colored = score_formatted.bright_blue();
            let mut block: Vec<String> = Vec::new();

            block.push(format!("◆ Rank {global_index} ─ Score {score_colored}"));

            block.push(format!("  {}:{}", alias_colored.bold(), first.lines));

            if params.show_anchor {
                if let Some(anchor) = first.anchor.as_deref() {
                    block.push(format!("  #{}", anchor.bright_black()));
                }
            }

            if !heading_path.is_empty() {
                let path_line = format_heading_path(heading_path, path_width);
                if !path_line.is_empty() {
                    block.push(format!("  in {path_line}"));
                }
            }

            let mut printed: BTreeSet<usize> = BTreeSet::new();
            let mut last_printed: Option<usize> = None;
            for hit in hits {
                for (line_no, line_text) in extract_context_lines(
                    storage.as_ref(),
                    &mut content_cache,
                    hit,
                    params.query,
                    params.snippet_lines,
                ) {
                    if printed.insert(line_no) {
                        if let Some(prev) = last_printed {
                            if line_no > prev + 1 {
                                let gap = line_no - prev - 1;
                                let gap_line = format!("... {gap} more lines").bright_black();
                                block.push(format!("  {gap_line}"));
                            }
                        }
                        if params.show_lines {
                            let label = format!("{line_no:>6}:").bright_black();
                            block.push(format!("  {label} {line_text}"));
                        } else {
                            block.push(format!("  {line_text}"));
                        }
                        last_printed = Some(line_no);
                    }
                }
            }

            if params.show_url {
                // TODO(release-polish): include cached canonical URL without hitting storage (docs/notes/release-polish-followups.md)
                if let Some(url) = resolve_group_url(hits, storage.as_ref(), alias) {
                    block.push(format!("  {}", url.bright_black()));
                }
            }

            rendered_groups.push(block.join("\n"));
        }

        println!("{}", rendered_groups.join("\n\n"));

        if params.no_summary {
            return;
        }

        let shown = params.hits.len();
        let total = params.total_results;
        let lines = params.total_lines_searched;
        let time_ms = params.search_time.as_millis();
        let sources = params.sources.len();

        println!(
            "\n→ {}/{} results shown",
            shown.to_string().green(),
            total.to_string().green()
        );
        println!(
            "  {} lines searched, {} source{}, took {}",
            lines.to_string().cyan(),
            sources,
            if sources == 1 { "" } else { "s" },
            format!("{time_ms}ms").blue()
        );
        if total > shown && params.page < params.total_pages {
            let next_page = params.page.saturating_add(1);
            println!("  Tip: See more with \"blz search --next\" or \"--page {next_page}\"");
        }
    }
}

fn strip_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let bytes: Vec<char> = s.chars().collect();
    while i < bytes.len() {
        match bytes[i] {
            '!' => {
                if i + 1 < bytes.len() && bytes[i + 1] == '[' {
                    i += 2;
                    while i < bytes.len() && bytes[i] != ']' {
                        out.push(bytes[i]);
                        i += 1;
                    }
                    while i < bytes.len() && bytes[i] != ')' {
                        i += 1;
                    }
                    if i < bytes.len() {
                        i += 1;
                    }
                    continue;
                }
            },
            '[' => {
                i += 1;
                while i < bytes.len() && bytes[i] != ']' {
                    out.push(bytes[i]);
                    i += 1;
                }
                while i < bytes.len() && bytes[i] != ')' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
                continue;
            },
            '<' => {
                while i < bytes.len() && bytes[i] != '>' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
                continue;
            },
            '#' | '*' | '_' | '`' => {
                i += 1;
                continue;
            },
            _ => {},
        }
        out.push(bytes[i]);
        i += 1;
    }
    out.trim().to_string()
}

fn resolve_group_url(
    hits: &[&SearchHit],
    storage: Option<&Storage>,
    alias: &str,
) -> Option<String> {
    if let Some(url) = hits.iter().find_map(|hit| hit.source_url.clone()) {
        return Some(url);
    }
    let storage = storage?;
    storage.load_llms_json(alias).ok().map(|doc| doc.source.url)
}

fn format_score_value(score: f32, precision: u8) -> String {
    let clamped = usize::from(precision.min(4));
    format!("{score:.clamped$}")
}

fn extract_context_lines(
    storage: Option<&Storage>,
    cache: &mut HashMap<String, Vec<String>>,
    hit: &SearchHit,
    query: &str,
    max_lines: usize,
) -> Vec<(usize, String)> {
    let (start, end) = parse_line_range(&hit.lines);
    let lines = match storage {
        Some(storage) => cache
            .entry(hit.alias.clone())
            .or_insert_with(|| load_llms_lines(storage, &hit.alias)),
        None => return Vec::new(),
    };

    let limit = max_lines.max(1);

    if lines.is_empty() {
        return hit
            .snippet
            .lines()
            .take(limit)
            .enumerate()
            .map(|(idx, line)| (idx + 1, line.to_string()))
            .collect();
    }

    let total = lines.len();
    let start_idx = start.saturating_sub(1).min(total.saturating_sub(1));
    let end_idx = end.saturating_sub(1).min(total.saturating_sub(1));

    let center = find_best_match_line(lines, start_idx, end_idx, query).unwrap_or(start_idx);
    let last_seg_norm = hit
        .heading_path
        .last()
        .map(|s| normalize(&strip_markdown(s)))
        .unwrap_or_default();

    let should_include = |idx: usize| -> bool {
        if let Some(raw) = lines.get(idx) {
            if raw.trim().is_empty() {
                return false;
            }
            if raw.trim_start().starts_with('#') {
                return false;
            }
            let cleaned = strip_markdown(raw);
            if !last_seg_norm.is_empty() && normalize(&cleaned) == last_seg_norm {
                return false;
            }
            return true;
        }
        false
    };

    let limit = max_lines.max(1);
    let mut candidates = Vec::with_capacity(limit);
    let mut below = center;
    let mut above = center;
    while candidates.len() < limit {
        if candidates.is_empty() {
            if should_include(center) {
                candidates.push(center);
            }
        } else {
            if below + 1 < total && candidates.len() < limit {
                below += 1;
                if should_include(below) {
                    candidates.push(below);
                }
            }
            if above > 0 && candidates.len() < limit {
                above = above.saturating_sub(1);
                if should_include(above) {
                    candidates.push(above);
                }
            }
            if (below + 1 >= total) && above == 0 {
                break;
            }
        }
        if (below + 1 >= total) && above == 0 {
            break;
        }
    }
    candidates.sort_unstable();

    let tokens: Vec<String> = query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(std::string::ToString::to_string)
        .collect();

    let mut result = Vec::with_capacity(candidates.len());
    for idx in candidates {
        let raw = lines.get(idx).map_or("", |s| s.as_str());
        let cleaned = strip_markdown(raw);
        let highlighted = highlight_matches(&cleaned, query, &tokens);
        result.push((idx + 1, highlighted));
    }
    result
}

fn parse_line_range(s: &str) -> (usize, usize) {
    let mut parts = s.split('-');
    let start = parts
        .next()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1);
    let end = parts
        .next()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(start);
    (start, end)
}

fn load_llms_lines(storage: &Storage, alias: &str) -> Vec<String> {
    if let Ok(path) = storage.llms_txt_path(alias) {
        if let Ok(content) = std::fs::read_to_string(path) {
            return content
                .lines()
                .map(std::string::ToString::to_string)
                .collect();
        }
    }
    Vec::new()
}

fn find_best_match_line(lines: &[String], start: usize, end: usize, query: &str) -> Option<usize> {
    let query_lower = query.to_lowercase();
    let range = start..=end;
    for idx in range.clone() {
        if let Some(line) = lines.get(idx) {
            if line.to_lowercase().contains(&query_lower) {
                return Some(idx);
            }
        }
    }

    let tokens: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .collect();
    let mut best: Option<(usize, usize)> = None;
    for idx in range {
        if let Some(line) = lines.get(idx) {
            let lower = line.to_lowercase();
            let matches = tokens.iter().filter(|token| lower.contains(*token)).count();
            if matches > 0 {
                match best {
                    Some((_, best_count)) if matches <= best_count => {},
                    _ => best = Some((idx, matches)),
                }
            }
        }
    }
    best.map(|(idx, _)| idx)
}

#[allow(clippy::similar_names, clippy::many_single_char_names)]
fn highlight_matches(line: &str, full_query: &str, tokens: &[String]) -> String {
    let original = line.to_string();
    let lower_line = original.to_lowercase();
    let query_lower = full_query.to_lowercase();
    if !query_lower.is_empty() && lower_line.contains(&query_lower) {
        if let Some(pos) = lower_line.find(&query_lower) {
            let (prefix, rest) = original.split_at(pos);
            let (hit, suffix) = rest.split_at(query_lower.len());
            return format!("{}{}{}", prefix, hit.red().bold(), suffix);
        }
    }

    let mut out = String::new();
    let mut index = 0;
    let chars: Vec<char> = original.chars().collect();
    let lower_chars: Vec<char> = lower_line.chars().collect();
    while index < chars.len() {
        let mut matched = None;
        for token in tokens {
            let len = token.chars().count();
            if len > 0 && index + len <= chars.len() {
                let slice = &lower_chars[index..index + len];
                if slice.iter().collect::<String>() == *token {
                    matched = Some(len);
                    break;
                }
            }
        }
        if let Some(len) = matched {
            let segment: String = chars[index..index + len].iter().collect();
            out.push_str(&segment.red().to_string());
            index += len;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }
    out
}

fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = false;
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_space = false;
        } else if ch.is_whitespace() && !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::format_score_value;

    #[test]
    fn respects_requested_precision() {
        assert_eq!(format_score_value(14.456, 0), "14");
        assert_eq!(format_score_value(14.456, 1), "14.5");
        assert_eq!(format_score_value(14.456, 3), "14.456");
    }

    #[test]
    fn clamps_precision_to_maximum() {
        assert_eq!(format_score_value(3.14159, 8), "3.1416");
    }
}
