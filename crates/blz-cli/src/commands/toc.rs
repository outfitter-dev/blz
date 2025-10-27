use anyhow::{Context, Result, anyhow};
use blz_core::{AnchorsMap, LlmsJson, Storage};
use chrono::Utc;
use colored::Colorize;

use crate::commands::RequestSpec;
use crate::output::OutputFormat;
use crate::utils::parsing::{LineRange, parse_line_ranges};
use crate::utils::preferences::{self, TocHistoryEntry};

/// Serialize a HeadingLevelFilter back to its string representation
fn serialize_heading_level_filter(
    filter: &crate::utils::heading_filter::HeadingLevelFilter,
) -> String {
    use crate::utils::heading_filter::HeadingLevelFilter;
    match filter {
        HeadingLevelFilter::Exact(n) => format!("={n}"),
        HeadingLevelFilter::LessThan(n) => format!("<{n}"),
        HeadingLevelFilter::LessThanOrEqual(n) => format!("<={n}"),
        HeadingLevelFilter::GreaterThan(n) => format!(">{n}"),
        HeadingLevelFilter::GreaterThanOrEqual(n) => format!(">={n}"),
        HeadingLevelFilter::List(levels) => levels
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(","),
        HeadingLevelFilter::Range(start, end) => format!("{start}-{end}"),
    }
}

#[allow(
    dead_code,
    clippy::unused_async,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools,
    clippy::cognitive_complexity
)]
pub async fn execute(
    alias: Option<&str>,
    sources: &[String],
    all: bool,
    output: OutputFormat,
    anchors: bool,
    show_anchors: bool,
    mut limit: Option<usize>,
    max_depth: Option<u8>,
    heading_level: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
    filter_expr: Option<&str>,
    _tree: bool,
    next: bool,
    previous: bool,
    last: bool,
    mut page: usize,
) -> Result<()> {
    let storage = Storage::new()?;
    let all_sources_mode = all && alias.is_none() && sources.is_empty();

    let last_entry = if next || previous || last {
        preferences::load_last_toc_entry()
    } else {
        None
    };

    if last_entry.is_none() && ((next || previous) || (last && limit.is_none())) {
        return Err(anyhow!(
            "No saved pagination state found. Run `blz toc <alias> --limit <COUNT>` first."
        ));
    }

    // Restore limit from history if navigating
    if (next || previous || last) && limit.is_none() {
        if let Some(entry) = &last_entry {
            limit = entry.limit;
        }
        if limit.is_none() {
            return Err(anyhow!(
                "No saved page size found. Run `blz toc <alias> --limit <COUNT>` first."
            ));
        }
    }

    // Restore filter parameters from history if navigating and not explicitly provided
    // Note: heading_level restoration is handled separately since it needs parsing
    let (filter_expr, max_depth) = if next || previous || last {
        let saved_filter = last_entry.as_ref().and_then(|e| e.filter.as_deref());
        let saved_max_depth = last_entry.as_ref().and_then(|e| e.max_depth);

        (filter_expr.or(saved_filter), max_depth.or(saved_max_depth))
    } else {
        (filter_expr, max_depth)
    };

    // Handle heading_level separately - parse from history string if needed
    let heading_level_parsed: Option<crate::utils::heading_filter::HeadingLevelFilter>;
    let heading_level = if heading_level.is_some() {
        // Use provided heading_level
        heading_level
    } else if next || previous || last {
        // Try to restore from history
        if let Some(saved_str) = last_entry.as_ref().and_then(|e| e.heading_level.as_deref()) {
            heading_level_parsed = saved_str.parse().ok();
            heading_level_parsed.as_ref()
        } else {
            None
        }
    } else {
        None
    };

    if next {
        page = last_entry
            .as_ref()
            .and_then(|entry| entry.page)
            .unwrap_or(1)
            .saturating_add(1);
    } else if previous {
        page = last_entry
            .as_ref()
            .and_then(|entry| entry.page)
            .unwrap_or(2)
            .saturating_sub(1)
            .max(1);
    } else if last {
        page = usize::MAX;
    }

    if anchors && filter_expr.is_some() {
        return Err(anyhow!("--filter cannot be combined with --anchors"));
    }

    let filter = filter_expr
        .map(HeadingFilter::parse)
        .transpose()
        .context("Failed to parse filter expression")?;

    // Use provided heading level filter, or convert --max-depth for backward compat
    let level_filter = heading_level.cloned().or_else(|| {
        max_depth.map(crate::utils::heading_filter::HeadingLevelFilter::LessThanOrEqual)
    });

    let mut source_list = if !sources.is_empty() {
        sources.to_vec()
    } else if let Some(single) = alias {
        vec![single.to_string()]
    } else if all_sources_mode {
        storage.list_sources()
    } else {
        Vec::new()
    };

    if source_list.is_empty() && (next || previous || last) {
        if let Some(entry) = &last_entry {
            if let Some(saved_sources) = &entry.source {
                source_list = saved_sources
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }

    if source_list.is_empty() {
        if all_sources_mode {
            return Err(anyhow!(
                "No sources configured. Add one with `blz add <alias> <url>` before running `blz toc --all`."
            ));
        }
        return Err(anyhow!(
            "No source specified. Provide an alias, --source <alias>, or run with --all."
        ));
    }

    if anchors {
        if source_list.len() > 1 {
            return Err(anyhow!("--anchors can only be used with a single source"));
        }
        let canonical = crate::utils::resolver::resolve_source(&storage, &source_list[0])?
            .map_or_else(|| source_list[0].to_string(), |c| c);
        let path = storage.anchors_map_path(&canonical)?;
        if !path.exists() {
            println!("No heading remap metadata found for '{canonical}'");
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
                    "Remap metadata for {} (updated {})
",
                    canonical.green(),
                    map.updated_at
                );
                for m in map.mappings {
                    let path_str = m.heading_path.join(" > ");
                    println!(
                        "  {}
    {} → {}
    {}",
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

    let mut all_entries = Vec::new();

    for source_alias in &source_list {
        let canonical = crate::utils::resolver::resolve_source(&storage, source_alias)?
            .map_or_else(|| source_alias.to_string(), |c| c);

        let llms: LlmsJson = storage
            .load_llms_json(&canonical)
            .with_context(|| format!("Failed to load TOC for '{canonical}'"))?;

        collect_entries(
            &mut all_entries,
            &llms.toc,
            max_depth.map(usize::from),
            0,
            filter.as_ref(),
            level_filter.as_ref(),
            source_alias,
            &canonical,
        );
    }

    let pagination_limit = if all && !all_sources_mode {
        None
    } else {
        limit
    };

    let total_results = all_entries.len();
    let (page_entries, actual_page, total_pages) = pagination_limit.map_or_else(
        || (all_entries.clone(), 1, 1),
        |lim| {
            let total_pages = if total_results == 0 {
                0
            } else {
                total_results.div_ceil(lim)
            };

            let actual_page = if page == usize::MAX {
                total_pages.max(1)
            } else {
                page.clamp(1, total_pages.max(1))
            };

            let start = (actual_page - 1) * lim;
            let end = start.saturating_add(lim).min(total_results);

            let page_entries = if start < total_results {
                all_entries[start..end].to_vec()
            } else {
                Vec::new()
            };

            (page_entries, actual_page, total_pages)
        },
    );

    if pagination_limit.is_some() {
        let source_str = if source_list.len() == 1 {
            Some(source_list[0].clone())
        } else if !source_list.is_empty() {
            Some(source_list.join(","))
        } else {
            None
        };

        let history_entry = TocHistoryEntry {
            timestamp: Utc::now().to_rfc3339(),
            source: source_str,
            format: preferences::format_to_string(output),
            page: Some(actual_page),
            limit: pagination_limit,
            total_pages: Some(total_pages),
            total_results: Some(total_results),
            filter: filter_expr.map(str::to_string),
            max_depth,
            heading_level: heading_level.map(serialize_heading_level_filter),
        };

        if let Err(err) = preferences::save_toc_history(&history_entry) {
            tracing::warn!("failed to save TOC history: {err}");
        }
    }

    match output {
        OutputFormat::Json => {
            // Always return object with pagination metadata for consistency
            let payload = serde_json::json!({
                "entries": page_entries,
                "page": actual_page,
                "total_pages": total_pages.max(1),
                "total_results": total_results,
                "page_size": pagination_limit,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&payload)
                    .context("Failed to serialize table of contents to JSON")?
            );
        },
        OutputFormat::Jsonl => {
            for e in page_entries {
                println!(
                    "{}",
                    serde_json::to_string(&e)
                        .context("Failed to serialize table of contents to JSONL")?
                );
            }
        },
        OutputFormat::Text => {
            // For paginated text output with flat list rendering
            if pagination_limit.is_some() && !_tree {
                // Print header
                if source_list.len() > 1 {
                    println!("Table of contents (showing {} sources)", source_list.len());
                } else if let Some(first) = source_list.first() {
                    let canonical = crate::utils::resolver::resolve_source(&storage, first)?
                        .map_or_else(|| first.to_string(), |c| c);
                    println!("Table of contents for {}\n", canonical.green());
                }

                // Print entries from page_entries (which already has pagination applied)
                for entry in &page_entries {
                    // Extract values from JSON
                    let heading_path = entry["headingPath"]
                        .as_array()
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                        .unwrap_or_default();
                    let name = heading_path.last().unwrap_or(&"");
                    let lines = entry["lines"].as_str().unwrap_or("");
                    let heading_level = entry["headingLevel"].as_u64().unwrap_or(1) as usize;

                    // Print with proper indentation
                    let indent = "  ".repeat(heading_level.saturating_sub(1));
                    let lines_display = format!("[{}]", lines).dimmed();

                    if show_anchors {
                        let anchor = entry["anchor"].as_str().unwrap_or("");
                        println!(
                            "{}- {} {} {}",
                            indent,
                            name,
                            lines_display,
                            anchor.bright_black()
                        );
                    } else {
                        println!("{}- {} {}", indent, name, lines_display);
                    }
                }

                // Print footer with pagination info
                println!(
                    "\nPage {} of {} ({} total results)",
                    actual_page,
                    total_pages.max(1),
                    total_results
                );
            } else {
                // Use original tree/hierarchical rendering for non-paginated or tree mode
                for source_alias in &source_list {
                    let canonical = crate::utils::resolver::resolve_source(&storage, source_alias)?
                        .map_or_else(|| source_alias.to_string(), |c| c);

                    let llms: LlmsJson = storage
                        .load_llms_json(&canonical)
                        .with_context(|| format!("Failed to load TOC for '{canonical}'"))?;

                    if source_list.len() > 1 {
                        println!(
                            "
{}:",
                            canonical.green()
                        );
                    } else {
                        println!(
                            "Table of contents for {}
",
                            canonical.green()
                        );
                    }

                    if _tree {
                        let mut count = 0;
                        let mut prev_depth: Option<usize> = None;
                        let mut prev_h1_had_children = false;
                        for (i, e) in llms.toc.iter().enumerate() {
                            let is_last = i == llms.toc.len() - 1;
                            print_tree(
                                e,
                                0,
                                is_last,
                                "",
                                max_depth.map(usize::from),
                                filter.as_ref(),
                                level_filter.as_ref(),
                                &mut count,
                                None, // No limit for tree mode
                                show_anchors,
                                &mut prev_depth,
                                &mut prev_h1_had_children,
                            );
                        }
                    } else {
                        for e in &llms.toc {
                            print_text(
                                e,
                                0,
                                max_depth.map(usize::from),
                                filter.as_ref(),
                                level_filter.as_ref(),
                                show_anchors,
                            );
                        }
                    }
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
fn print_text_with_limit(
    e: &blz_core::TocEntry,
    depth: usize,
    remaining: usize,
    max_depth: Option<usize>,
    filter: Option<&HeadingFilter>,
    level_filter: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
    show_anchors: bool,
) -> usize {
    if remaining == 0 || exceeds_depth(depth, max_depth) {
        return 0;
    }

    let display_path = display_path(e);
    let name = display_path.last().cloned().unwrap_or_default();
    #[allow(clippy::cast_possible_truncation)] // depth is limited to 1-6 for markdown headings
    let level_matches = level_filter.is_none_or(|f| f.matches((depth + 1) as u8));
    let text_matches = filter.is_none_or(|f| f.matches(&display_path, e.anchor.as_deref()));

    let mut printed = if text_matches && level_matches {
        let indent = "  ".repeat(depth);
        let lines_display = format!("[{}]", e.lines).dimmed();

        if show_anchors {
            let anchor = e.anchor.clone().unwrap_or_default();
            println!(
                "{}- {} {} {}",
                indent,
                name,
                lines_display,
                anchor.bright_black()
            );
        } else {
            println!("{indent}- {name} {lines_display}");
        }

        1
    } else {
        0
    };
    if can_descend(depth, max_depth) {
        for c in &e.children {
            if printed >= remaining {
                break;
            }
            printed += print_text_with_limit(
                c,
                depth + 1,
                remaining - printed,
                max_depth,
                filter,
                level_filter,
                show_anchors,
            );
        }
    }
    printed
}

#[allow(dead_code, clippy::items_after_statements, clippy::too_many_arguments)]
fn collect_entries(
    entries: &mut Vec<serde_json::Value>,
    list: &[blz_core::TocEntry],
    max_depth: Option<usize>,
    depth: usize,
    filter: Option<&HeadingFilter>,
    level_filter: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
    alias: &str,
    canonical: &str,
) {
    for e in list {
        if exceeds_depth(depth, max_depth) {
            continue;
        }
        let display_path = display_path(e);
        #[allow(clippy::cast_possible_truncation)] // depth is limited to 1-6 for markdown headings
        let level_matches = level_filter.is_none_or(|f| f.matches((depth + 1) as u8));
        let text_matches = filter.is_none_or(|f| f.matches(&display_path, e.anchor.as_deref()));

        if text_matches && level_matches {
            entries.push(serde_json::json!({
                "alias": alias,
                "source": canonical,
                "headingPath": display_path,
                "rawHeadingPath": e.heading_path,
                "headingPathNormalized": e.heading_path_normalized,
                "headingLevel": depth + 1,
                "lines": e.lines,
                "anchor": e.anchor,
            }));
        }
        if !e.children.is_empty() && can_descend(depth, max_depth) {
            collect_entries(
                entries,
                &e.children,
                max_depth,
                depth + 1,
                filter,
                level_filter,
                alias,
                canonical,
            );
        }
    }
}

#[allow(dead_code)]
fn print_text(
    e: &blz_core::TocEntry,
    depth: usize,
    max_depth: Option<usize>,
    filter: Option<&HeadingFilter>,
    level_filter: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
    show_anchors: bool,
) {
    if exceeds_depth(depth, max_depth) {
        return;
    }
    let display_path = display_path(e);
    let name = display_path.last().cloned().unwrap_or_default();
    #[allow(clippy::cast_possible_truncation)] // depth is limited to 1-6 for markdown headings
    let level_matches = level_filter.is_none_or(|f| f.matches((depth + 1) as u8));
    let text_matches = filter.is_none_or(|f| f.matches(&display_path, e.anchor.as_deref()));
    if text_matches && level_matches {
        let indent = "  ".repeat(depth);
        let lines_display = format!("[{}]", e.lines).dimmed();

        if show_anchors {
            let anchor = e.anchor.clone().unwrap_or_default();
            println!(
                "{}- {} {} {}",
                indent,
                name,
                lines_display,
                anchor.bright_black()
            );
        } else {
            println!("{indent}- {name} {lines_display}");
        }
    }
    if can_descend(depth, max_depth) {
        for c in &e.children {
            print_text(c, depth + 1, max_depth, filter, level_filter, show_anchors);
        }
    }
}

#[allow(dead_code, clippy::too_many_arguments)]
fn print_tree(
    e: &blz_core::TocEntry,
    depth: usize,
    is_last: bool,
    prefix: &str,
    max_depth: Option<usize>,
    filter: Option<&HeadingFilter>,
    level_filter: Option<&crate::utils::heading_filter::HeadingLevelFilter>,
    count: &mut usize,
    limit: Option<usize>,
    show_anchors: bool,
    prev_depth: &mut Option<usize>,
    prev_h1_had_children: &mut bool,
) -> bool {
    if let Some(limit_count) = limit {
        if *count >= limit_count {
            return false;
        }
    }

    if exceeds_depth(depth, max_depth) {
        return false;
    }

    let display_path = display_path(e);
    let name = display_path.last().cloned().unwrap_or_default();
    #[allow(clippy::cast_possible_truncation)] // depth is limited to 1-6 for markdown headings
    let level_matches = level_filter.is_none_or(|f| f.matches((depth + 1) as u8));
    let text_matches = filter.is_none_or(|f| f.matches(&display_path, e.anchor.as_deref()));

    if text_matches && level_matches {
        // Add blank line when jumping up levels (but not to H1 - H1 handles its own spacing)
        if let Some(prev) = *prev_depth {
            if depth < prev && depth > 0 {
                // Jumping up levels within H2+
                if depth > 1 {
                    // H3+ has continuation pipes
                    let pipe_prefix = prefix.trim_end();
                    println!("{pipe_prefix}");
                } else if depth == 1 {
                    // H2 level: show pipe if not last sibling
                    if is_last {
                        println!();
                    } else {
                        println!("│");
                    }
                }
            }
        }

        let lines_display = format!("[{}]", e.lines).dimmed();

        // H1s (depth 0) are left-aligned with no branch characters
        if depth == 0 {
            // Add blank line before H1 if previous H1 had visible children
            if *prev_h1_had_children {
                println!();
            }
            if show_anchors {
                let anchor = e.anchor.clone().unwrap_or_default();
                println!("{name} {lines_display} {}", anchor.bright_black());
            } else {
                println!("{name} {lines_display}");
            }
        } else {
            // H2+ use tree structure
            let branch = if is_last { "└─ " } else { "├─ " };
            if show_anchors {
                let anchor = e.anchor.clone().unwrap_or_default();
                println!(
                    "{prefix}{branch}{name} {lines_display} {}",
                    anchor.bright_black()
                );
            } else {
                println!("{prefix}{branch}{name} {lines_display}");
            }
        }
        *count += 1;
        *prev_depth = Some(depth);
    }

    let mut had_visible_children = false;

    if can_descend(depth, max_depth) {
        let new_prefix = if depth == 0 {
            // For H1s, children don't get additional prefix since H1 is left-aligned
            String::new()
        } else {
            format!("{}{}  ", prefix, if is_last { " " } else { "│" })
        };

        for (i, c) in e.children.iter().enumerate() {
            if let Some(limit_count) = limit {
                if *count >= limit_count {
                    break;
                }
            }
            let child_is_last = i == e.children.len() - 1;
            let child_printed = print_tree(
                c,
                depth + 1,
                child_is_last,
                &new_prefix,
                max_depth,
                filter,
                level_filter,
                count,
                limit,
                show_anchors,
                prev_depth,
                prev_h1_had_children,
            );
            if child_printed {
                had_visible_children = true;
            }
        }
    }

    // If this is an H1, update the flag for next H1
    if depth == 0 && text_matches && level_matches {
        *prev_h1_had_children = had_visible_children;
    }

    text_matches && level_matches
}

fn display_path(entry: &blz_core::TocEntry) -> Vec<String> {
    entry
        .heading_path_display
        .clone()
        .unwrap_or_else(|| entry.heading_path.clone())
}

fn exceeds_depth(depth: usize, max_depth: Option<usize>) -> bool {
    max_depth.is_some_and(|max| depth + 1 > max)
}

fn can_descend(depth: usize, max_depth: Option<usize>) -> bool {
    max_depth.is_none_or(|max| depth + 1 < max)
}

#[derive(Debug, Default)]
struct HeadingFilter {
    must: Vec<String>,
    any: Vec<String>,
    not: Vec<String>,
}

impl HeadingFilter {
    fn parse(expr: &str) -> Result<Self> {
        let tokens = tokenize_filter(expr)?;
        let mut terms: Vec<(TokenKind, String)> = Vec::new();
        let mut pending: Option<TokenKind> = None;

        for token in tokens {
            if token.trim().is_empty() {
                continue;
            }

            let lower = token.to_ascii_lowercase();
            match lower.as_str() {
                "and" | "&&" => {
                    pending = Some(TokenKind::Must);
                    if let Some(last) = terms.last_mut() {
                        if matches!(last.0, TokenKind::Any) {
                            last.0 = TokenKind::Must;
                        }
                    }
                    continue;
                },
                "or" | "||" => {
                    pending = Some(TokenKind::Any);
                    continue;
                },
                "not" | "!" => {
                    pending = Some(TokenKind::Not);
                    continue;
                },
                _ => {},
            }

            let (kind, value) = classify_token(&token, pending)?;
            terms.push((kind, value.to_ascii_lowercase()));
            pending = None;
        }

        if let Some(kind) = pending {
            return Err(anyhow!(
                "Filter expression ended with operator {}; expected another term",
                match kind {
                    TokenKind::Must => "AND",
                    TokenKind::Any => "OR",
                    TokenKind::Not => "NOT",
                }
            ));
        }

        if terms.is_empty() {
            return Err(anyhow!("Filter expression must include at least one term"));
        }

        let mut filter = Self::default();
        for (kind, value) in terms {
            match kind {
                TokenKind::Must => filter.must.push(value),
                TokenKind::Any => filter.any.push(value),
                TokenKind::Not => filter.not.push(value),
            }
        }
        Ok(filter)
    }

    fn matches(&self, display_path: &[String], anchor: Option<&str>) -> bool {
        let mut haystack = display_path.join(" ").to_ascii_lowercase();
        if let Some(anchor) = anchor {
            if !anchor.is_empty() {
                haystack.push(' ');
                haystack.push_str(&anchor.to_ascii_lowercase());
            }
        }

        if self.must.iter().any(|term| !haystack.contains(term)) {
            return false;
        }
        if !self.any.is_empty() && !self.any.iter().any(|term| haystack.contains(term)) {
            return false;
        }
        if self.not.iter().any(|term| haystack.contains(term)) {
            return false;
        }
        // If the user only provided negative terms, matched entries survive by default.
        true
    }
}

#[derive(Clone, Copy, Debug)]
enum TokenKind {
    Must,
    Any,
    Not,
}

fn tokenize_filter(expr: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = '\0';

    for ch in expr.chars() {
        match ch {
            '"' | '\'' => {
                if in_quote {
                    if ch == quote_char {
                        in_quote = false;
                    } else {
                        current.push(ch);
                    }
                } else {
                    in_quote = true;
                    quote_char = ch;
                }
            },
            c if c.is_whitespace() && !in_quote => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            },
            _ => current.push(ch),
        }
    }

    if in_quote {
        return Err(anyhow!("Unterminated quote in filter expression"));
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

fn classify_token(token: &str, pending: Option<TokenKind>) -> Result<(TokenKind, String)> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Encountered empty term in filter expression"));
    }
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return Err(anyhow!("Encountered empty term in filter expression"));
    };
    let (kind, value) = match first {
        '+' => (TokenKind::Must, trimmed[1..].trim()),
        '-' | '!' => (TokenKind::Not, trimmed[1..].trim()),
        _ => (pending.unwrap_or(TokenKind::Any), trimmed),
    };
    if value.is_empty() {
        return Err(anyhow!("Filter term '{trimmed}' is missing a value"));
    }
    Ok((kind, value.to_string()))
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
