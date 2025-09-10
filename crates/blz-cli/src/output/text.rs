//! Text output formatting

use super::formatter::FormatParams;
use blz_core::{SearchHit, Storage};
use colored::Colorize;
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

use crate::utils::formatting::get_alias_color;

pub struct TextFormatter;

impl TextFormatter {
    /// Format search results in the brief, colorized layout
    #[allow(clippy::too_many_lines)]
    pub fn format_search_results(params: &FormatParams) {
        if params.hits.is_empty() {
            println!("No results found for '{}'", params.query);
            return;
        }

        // Cache file contents per alias for context line lookup
        let mut content_cache: HashMap<String, Vec<String>> = HashMap::new();
        let storage = Storage::new().ok();

        // Assign stable colors to aliases on this page (sorted by alias)
        let mut alias_colors: HashMap<String, usize> = HashMap::new();
        let mut sorted_aliases: Vec<String> = params
            .hits
            .iter()
            .map(|h| h.alias.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        sorted_aliases.sort();
        for (idx, a) in sorted_aliases.iter().enumerate() {
            alias_colors.insert(a.clone(), idx);
        }
        let mut color_index: usize = alias_colors.len();

        // Optional page sources header when URL modifier is on
        if params.show_url {
            let shown = params.hits.len();
            println!(
                "Results {}/{}:",
                shown.to_string().green(),
                params.total_results.to_string().green()
            );

            // Build alias -> URL map from current page
            let mut alias_url: HashMap<String, String> = HashMap::new();
            for hit in params.hits {
                if let Some(u) = &hit.source_url {
                    alias_url
                        .entry(hit.alias.clone())
                        .or_insert_with(|| u.clone());
                }
            }
            // Fallback to storage metadata URL if needed
            if let Some(s) = &storage {
                for alias in &sorted_aliases {
                    if !alias_url.contains_key(alias) {
                        if let Ok(j) = s.load_llms_json(alias) {
                            alias_url.insert(alias.clone(), j.source.url);
                        }
                    }
                }
            }

            for alias in &sorted_aliases {
                if let Some(url) = alias_url.get(alias) {
                    let idx = alias_colors.get(alias).copied().unwrap_or(0);
                    let alias_colored = get_alias_color(alias, idx);
                    println!("[{}] {}", alias_colored, url.bright_black());
                }
            }
            // No extra blank line to keep output tight
        }

        // Group by (alias, heading_path)
        let mut groups: Vec<(String, Vec<String>, Vec<&SearchHit>)> = Vec::new();
        for hit in params.hits {
            if let Some((last_alias, last_path, hits)) = groups.last_mut() {
                if *last_alias == hit.alias && *last_path == hit.heading_path {
                    hits.push(hit);
                    continue;
                }
            }
            groups.push((hit.alias.clone(), hit.heading_path.clone(), vec![hit]));
        }

        for (g_idx, (alias, heading_path, hits)) in groups.iter().enumerate() {
            let global_index = params.start_idx + g_idx + 1;

            // Determine consistent color for alias
            let alias_idx = *alias_colors.entry(alias.clone()).or_insert_with(|| {
                let idx = color_index;
                color_index = color_index.saturating_add(1);
                idx
            });
            let alias_colored = get_alias_color(alias, alias_idx);

            // Header line: optional rank, alias:lines (score: N)
            let mut header = String::new();
            if params.show_rank {
                let _ = write!(&mut header, "{global_index}. ");
            }
            let first = hits[0];
            // Bold alias, keep line range standard foreground
            let _ = write!(&mut header, "{}:{} ", alias_colored.bold(), first.lines);
            // Score label dim, value standard foreground
            #[allow(clippy::cast_possible_truncation)]
            let score_int = first.score.round() as i32;
            let _ = write!(&mut header, "({} {})", "score:".bright_black(), score_int);
            println!("{header}");

            // Heading path line: bold, no extra color
            let path = heading_path
                .iter()
                .map(|s| strip_markdown(s))
                .collect::<Vec<_>>()
                .join(" > ");
            if !path.is_empty() {
                println!("{}", path.bold());
            }

            // Merge context lines across hits
            let mut printed: BTreeSet<usize> = BTreeSet::new();
            let mut last_printed: Option<usize> = None;
            for hit in hits {
                for (ln, line) in
                    extract_context_lines(storage.as_ref(), &mut content_cache, hit, params.query)
                {
                    if printed.insert(ln) {
                        if let Some(prev) = last_printed {
                            if ln > prev + 1 {
                                let gap = ln - prev - 1;
                                println!("{}", format!("... {gap} more lines").bright_black());
                            }
                        }
                        println!("{line}");
                        last_printed = Some(ln);
                    }
                }
            }

            // No blank line between groups to avoid visual separation
        }

        // Summary line
        let shown = params.hits.len();
        let total = params.total_results;
        let lines = params.total_lines_searched;
        let time_ms = params.search_time.as_millis();

        if params.no_stats {
            // Suppressed
        } else if total == shown {
            println!(
                "\n{} results found, {} lines searched, took {}",
                shown.to_string().green(),
                lines.to_string().cyan(),
                format!("{time_ms}ms").blue()
            );
        } else {
            println!(
                "\n{}/{} results shown, {} lines searched, took {}",
                shown.to_string().green(),
                total.to_string().green(),
                lines.to_string().cyan(),
                format!("{time_ms}ms").blue()
            );
            let next_page = params.page.saturating_add(1);
            println!(
                "Tip: use \"blz search --next\" to see the next page (or \"--page {next_page}\" in a full query)"
            );
        }
    }
}

fn strip_markdown(s: &str) -> String {
    // Simple markdown stripper: links [text](url) -> text; remove *, _, `, leading #, images ![alt](url) -> alt; remove HTML tags <...>
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let bytes: Vec<char> = s.chars().collect();
    while i < bytes.len() {
        match bytes[i] {
            '!' => {
                // image ![alt](url)
                if i + 1 < bytes.len() && bytes[i + 1] == '[' {
                    // consume until closing ] then until )
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
                // link [text](url)
                i += 1;
                while i < bytes.len() && bytes[i] != ']' {
                    out.push(bytes[i]);
                    i += 1;
                }
                // skip ]( ... )
                while i < bytes.len() && bytes[i] != ')' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
                continue;
            },
            '<' => {
                // html tag
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

fn extract_context_lines(
    storage: Option<&Storage>,
    cache: &mut HashMap<String, Vec<String>>,
    hit: &SearchHit,
    query: &str,
) -> Vec<(usize, String)> {
    let mut result = Vec::new();
    let (start, end) = parse_line_range(&hit.lines);

    // Get or load lines without cloning - borrow from cache
    let lines = match storage {
        Some(s) => cache
            .entry(hit.alias.clone())
            .or_insert_with(|| load_llms_lines(s, &hit.alias)),
        None => return Vec::new(), // Early return if no storage
    };

    let total = lines.len();
    if total == 0 {
        // Fallback to existing snippet if we can't load content
        for (i, ln) in hit.snippet.lines().take(3).enumerate() {
            result.push((i + 1, ln.to_string()));
        }
        return result;
    }

    let start_idx = start.saturating_sub(1).min(total.saturating_sub(1));
    let end_idx = end.saturating_sub(1).min(total.saturating_sub(1));

    let center = find_best_match_line(lines, start_idx, end_idx, query).unwrap_or(start_idx);
    // Determine last heading segment (normalized) to avoid duplicating it in snippet
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
                return false; // markdown heading line
            }
            let cleaned = strip_markdown(raw);
            let norm = normalize(&cleaned);
            if !last_seg_norm.is_empty() && norm == last_seg_norm {
                return false; // duplicate of heading
            }
            return true;
        }
        false
    };
    // Gather up to 3 non-blank lines around center, prioritizing center, then below, then above
    let mut candidates = Vec::new();
    let mut below = center;
    let mut above = center;
    while candidates.len() < 3 {
        if candidates.is_empty() {
            // center
            if should_include(center) {
                candidates.push(center);
            }
        } else {
            // alternate below then above
            if below + 1 < total && candidates.len() < 3 {
                below += 1;
                if should_include(below) {
                    candidates.push(below);
                }
            }
            if above > 0 && candidates.len() < 3 {
                above = above.saturating_sub(1);
                if should_include(above) {
                    candidates.push(above);
                }
            }
            if (below + 1 >= total) && (above == 0) {
                break;
            }
        }
        if (below + 1 >= total) && (above == 0) {
            break;
        }
    }
    candidates.sort_unstable();

    let tokens: Vec<String> = query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(std::string::ToString::to_string)
        .collect();

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
    for i in range.clone() {
        if let Some(line) = lines.get(i) {
            if line.to_lowercase().contains(&query_lower) {
                return Some(i);
            }
        }
    }
    // Fallback: token-based
    let tokens: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .collect();
    let mut best: Option<(usize, usize)> = None; // (idx, matches)
    for i in range {
        if let Some(line) = lines.get(i) {
            let l = line.to_lowercase();
            let count = tokens.iter().filter(|t| l.contains(*t)).count();
            if count > 0 {
                if let Some((_, best_count)) = best {
                    if count > best_count {
                        best = Some((i, count));
                    }
                } else {
                    best = Some((i, count));
                }
            }
        }
    }
    best.map(|(i, _)| i)
}

#[allow(clippy::similar_names, clippy::many_single_char_names)]
fn highlight_matches(line: &str, full_query: &str, tokens: &[String]) -> String {
    let original = line.to_string();
    let lower_line = original.to_lowercase();
    let query_lower = full_query.to_lowercase();
    if !query_lower.is_empty() && lower_line.contains(&query_lower) {
        if let Some(pos) = lower_line.find(&query_lower) {
            let (a, rest) = original.split_at(pos);
            let (b, c) = rest.split_at(query_lower.len());
            return format!("{}{}{}", a, b.red().bold(), c);
        }
    }
    // Token highlights (dim red)
    let mut out = String::new();
    let mut i = 0;
    let chars: Vec<char> = original.chars().collect();
    let lower: Vec<char> = lower_line.chars().collect();
    while i < chars.len() {
        let mut matched = None;
        for t in tokens {
            let len = t.chars().count();
            if len > 0 && i + len <= chars.len() {
                let slice = &lower[i..i + len];
                if slice.iter().collect::<String>() == *t {
                    matched = Some(len);
                    break;
                }
            }
        }
        if let Some(len) = matched {
            let seg: String = chars[i..i + len].iter().collect();
            out.push_str(&seg.red().to_string());
            i += len;
        } else {
            out.push(chars[i]);
            i += 1;
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
        } else if !ch.is_whitespace() {
            // skip other punctuation
        }
    }
    out.trim().to_string()
}
