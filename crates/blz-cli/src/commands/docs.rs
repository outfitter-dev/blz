use anyhow::Result;
use chrono::Utc;
use clap::{Command, CommandFactory, ValueEnum};
use std::fmt::Write as _;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum DocsFormat {
    /// Render CLI documentation as Markdown.
    Markdown,
    /// Render CLI documentation as JSON.
    Json,
}

/// Render CLI documentation in the requested format.
///
/// # Errors
///
/// Returns an error if JSON serialization fails when emitting JSON output.
pub fn execute(format: DocsFormat) -> Result<()> {
    match format {
        DocsFormat::Markdown => {
            let md = generate_markdown::<crate::cli::Cli>();
            println!("{md}");
        },
        DocsFormat::Json => {
            let json = generate_json::<crate::cli::Cli>();
            println!("{}", serde_json::to_string_pretty(&json)?);
        },
    }
    Ok(())
}

fn generate_markdown<C: CommandFactory>() -> String {
    let mut out = String::new();
    let root = C::command();

    let _ = write!(&mut out, "# {}\n\n", root.get_name());
    if let Some(ver) = root.get_version() {
        let _ = write!(&mut out, "Version: {ver}\n\n");
    }
    if let Some(about) = root.get_about() {
        let _ = write!(&mut out, "{about}\n\n");
    }
    if let Some(long) = root.get_long_about() {
        let _ = write!(&mut out, "{long}\n\n");
    }

    // Root help
    let mut buf = Vec::new();
    let _ = root.clone().write_long_help(&mut buf);
    if let Ok(help) = String::from_utf8(buf) {
        out.push_str("## blz --help\n\n");
        out.push_str("```\n");
        out.push_str(&help);
        out.push_str("\n```\n\n");
    }

    // Subcommands
    out.push_str("## Subcommands\n\n");
    for sc in root.get_subcommands() {
        let _ = write!(&mut out, "### {}\n\n", sc.get_name());
        if let Some(about) = sc.get_about() {
            let _ = write!(&mut out, "{about}\n\n");
        }
        let aliases: Vec<String> = sc
            .get_visible_aliases()
            .collect::<Vec<_>>()
            .into_iter()
            .map(std::string::ToString::to_string)
            .collect();
        if !aliases.is_empty() {
            let alias_list = aliases.join(", ");
            let _ = write!(&mut out, "Aliases: {alias_list}\n\n");
        }
        let mut buf = Vec::new();
        let _ = sc.clone().write_long_help(&mut buf);
        if let Ok(help) = String::from_utf8(buf) {
            out.push_str("```\n");
            out.push_str(&help);
            out.push_str("\n```\n\n");
        }
    }

    out
}

fn generate_json<C: CommandFactory>() -> serde_json::Value {
    let root = C::command();
    let commands = root
        .get_subcommands()
        .map(command_to_json)
        .collect::<Vec<_>>();

    serde_json::json!({
        "name": root.get_name(),
        "about": root.get_about().map(std::string::ToString::to_string),
        "longAbout": root.get_long_about().map(std::string::ToString::to_string),
        "usage": root.clone().render_usage().to_string(),
        "version": root.get_version(),
        "schemaVersion": 1,
        "generatedAt": Utc::now().to_rfc3339(),
        "subcommands": commands,
    })
}

fn command_to_json(cmd: &Command) -> serde_json::Value {
    let args = cmd
        .get_arguments()
        .map(|a| {
            let takes_value = a.get_num_args().is_some_and(|n| n.takes_values());
            let num_args_str = a
                .get_num_args()
                .map_or_else(|| "None".to_string(), |n| format!("{n:?}"));
            serde_json::json!({
                "id": a.get_id().as_str(),
                "name": a.get_id().as_str(),
                "help": a.get_help().map(std::string::ToString::to_string),
                "longHelp": a.get_long_help().map(std::string::ToString::to_string),
                "required": a.is_required_set(),
                "takesValue": takes_value,
                "short": a.get_short().map(|c| c.to_string()),
                "long": a.get_long(),
                "default": a.get_default_values().first().map(|v| v.to_string_lossy().to_string()),
                "numArgs": num_args_str,
                "valueNames": a.get_value_names().map(|v| v.iter().map(std::string::ToString::to_string).collect::<Vec<_>>()),
            })
        })
        .collect::<Vec<_>>();

    let aliases: Vec<String> = cmd
        .get_visible_aliases()
        .collect::<Vec<_>>()
        .into_iter()
        .map(std::string::ToString::to_string)
        .collect();

    let subs = cmd
        .get_subcommands()
        .map(command_to_json)
        .collect::<Vec<_>>();
    let usage = {
        let mut c = cmd.clone();
        c.render_usage().to_string()
    };

    serde_json::json!({
        "name": cmd.get_name(),
        "about": cmd.get_about().map(std::string::ToString::to_string),
        "longAbout": cmd.get_long_about().map(std::string::ToString::to_string),
        "usage": usage,
        "aliases": aliases,
        "visibleAliases": aliases,
        "args": args,
        "subcommands": subs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docs_markdown_contains_root_help() {
        let md = generate_markdown::<crate::cli::Cli>();
        assert!(
            md.contains("## blz --help"),
            "markdown should include root help section"
        );
        assert!(
            md.contains("## Subcommands"),
            "markdown should list subcommands"
        );
    }

    #[test]
    fn docs_json_has_expected_top_level_shape() {
        let json = generate_json::<crate::cli::Cli>();
        assert_eq!(json.get("name").and_then(|v| v.as_str()), Some("blz"));
        assert!(json.get("subcommands").and_then(|v| v.as_array()).is_some());
        assert!(json.get("usage").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn docs_markdown_includes_aliases_when_present() {
        let md = generate_markdown::<crate::cli::Cli>();
        // 'list' has alias 'sources'
        assert!(
            md.contains("Aliases:"),
            "markdown should include Aliases section for commands with aliases"
        );
    }

    #[test]
    fn docs_json_includes_aliases_array() {
        let json = generate_json::<crate::cli::Cli>();
        // Ensure at least one subcommand exposes alias 'sources'
        let has_sources_alias = json
            .get("subcommands")
            .and_then(|v| v.as_array())
            .is_some_and(|subs| {
                subs.iter().any(|c| {
                    c.get("aliases")
                        .and_then(|v| v.as_array())
                        .is_some_and(|aliases| {
                            aliases.iter().any(|a| a.as_str() == Some("sources"))
                        })
                })
            });
        assert!(has_sources_alias, "expected at least one 'sources' alias");
    }
}
