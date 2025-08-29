//! Text output formatting

use super::formatter::FormatParams;
use blz_core::SearchHit;
use colored::Colorize;
use std::collections::HashMap;

use crate::utils::formatting::get_alias_color;

pub struct TextFormatter;

impl TextFormatter {
    /// Format search results as pretty text
    pub fn format_search_results(params: &FormatParams) {
        if params.hits.is_empty() {
            println!("No results found for '{}'", params.query);
            return;
        }

        // Show pagination info if limited
        if params.show_pagination && params.total_results > params.hits.len() {
            println!(
                "Showing {} of {} results\n",
                params.hits.len(),
                params.total_results
            );
        }

        // Track unique aliases for color cycling
        let mut alias_colors = HashMap::new();
        let mut color_index = 0;

        for (i, hit) in params.hits.iter().enumerate() {
            let global_index = params.start_idx + i + 1;

            // Get color for alias
            let alias_colored = if let Some(&idx) = alias_colors.get(&hit.alias) {
                get_alias_color(&hit.alias, idx)
            } else {
                alias_colors.insert(hit.alias.clone(), color_index);
                let colored = get_alias_color(&hit.alias, color_index);
                color_index += 1;
                colored
            };

            // Format result header
            format_result_header(
                global_index,
                hit,
                &alias_colored,
                params.single_source,
                params.sources,
            );

            // Format score and content
            format_result_content(hit);

            if i < params.hits.len() - 1 {
                println!();
            }
        }

        // Performance stats
        println!(
            "\n{}",
            format!(
                "Searched {} lines in {}ms • Found {} results",
                params.total_lines_searched,
                params.search_time.as_millis(),
                params.total_results
            )
            .bright_black()
        );
    }
}

fn format_result_header(
    index: usize,
    hit: &SearchHit,
    alias_colored: &colored::ColoredString,
    single_source: bool,
    sources: &[String],
) {
    use std::fmt::Write;
    let mut header = format!("{index}. ");

    // Only show alias if not filtering by single source
    if !single_source || sources.len() > 1 {
        let _ = write!(&mut header, "{alias_colored} ");
    }

    let _ = write!(
        &mut header,
        "[{}] {}",
        hit.lines.bright_black(),
        hit.heading_path.join(" > ")
    );

    println!("{header}");
}

fn format_result_content(hit: &SearchHit) {
    // Score line
    println!("   Score: {:.2}", hit.score.to_string().bright_blue());

    // Divider
    println!("   {}", "---".bright_black());

    // Content snippet
    let content_lines: Vec<&str> = hit.snippet.lines().collect();
    for line in content_lines.iter().take(5) {
        println!("   {line}");
    }

    if content_lines.len() > 5 {
        println!("   {}", "...".bright_black());
    }

    // Bottom divider
    println!("   {}", "---".bright_black());
}
