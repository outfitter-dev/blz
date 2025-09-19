use anyhow::Result;
use colored::Colorize;
use serde_json::json;

use crate::output::OutputFormat;
use crate::utils::preferences::{self, CliPreferences};

pub fn show(prefs: &CliPreferences, limit: usize, format: OutputFormat) -> Result<()> {
    let limit = limit.max(1);
    match format {
        OutputFormat::Text => {
            render_text(prefs, limit);
        },
        OutputFormat::Json => {
            let entries: Vec<_> = prefs.history().iter().rev().take(limit).cloned().collect();
            println!("{}", serde_json::to_string_pretty(&json!(entries))?);
        },
        OutputFormat::Jsonl => {
            for entry in prefs.history().iter().rev().take(limit) {
                println!("{}", serde_json::to_string(entry)?);
            }
        },
    }
    Ok(())
}

fn render_text(prefs: &CliPreferences, limit: usize) {
    let defaults = prefs.default_show_components();
    println!(
        "{} {}",
        "Default show:".bright_black(),
        preferences::format_show_components(&defaults)
    );
    println!(
        "{} {}",
        "Default snippet lines:".bright_black(),
        prefs.default_snippet_lines()
    );
    println!(
        "{} {}",
        "Default score precision:".bright_black(),
        prefs.default_score_precision()
    );
    println!();

    let entries: Vec<_> = prefs.history().iter().rev().take(limit).collect();
    if entries.is_empty() {
        println!("{}", "No recent searches recorded.".bright_black());
        return;
    }

    for (idx, entry) in entries.iter().enumerate() {
        println!(
            "{} {}",
            format!("{}.", idx + 1).green(),
            entry.query.clone()
        );
        if let Some(alias) = &entry.alias {
            println!("   {} {}", "alias:".bright_black(), alias);
        }
        println!("   {} {}", "format:".bright_black(), &entry.format);
        if !entry.show.is_empty() {
            println!("   {} {}", "show:".bright_black(), entry.show.join(", "));
        }
        println!(
            "   {} {} (score precision {})",
            "snippet lines:".bright_black(),
            entry.snippet_lines,
            entry.score_precision
        );
        println!("   {} {}", "timestamp:".bright_black(), entry.timestamp);
        println!();
    }
}
