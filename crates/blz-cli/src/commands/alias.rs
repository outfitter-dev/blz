use anyhow::Result;
use colored::Colorize;

/// Alias management scaffolding.
///
/// TODO: Implement persistence of aliases in source metadata.
/// - Preferred approach: add `aliases: Vec<String>` (#[serde(default)]) to blz_core::types::Source
///   and update save/load paths in storage. Maintain canonical `source` while allowing alternates.
/// - Allow alias formats like `@scope/package` (relax validation for aliases only).
/// - Add a resolver: id > source > alias. On ambiguous alias across sources, prompt or require
///   explicit `--source`.
/// - Update list/status JSON to include `aliases` once persisted.
pub enum AliasCommand {
    Add { source: String, alias: String },
    Rm { source: String, alias: String },
}

pub async fn execute(cmd: AliasCommand) -> Result<()> {
    match cmd {
        AliasCommand::Add { source, alias } => {
            println!(
                "{} {} {}\n\n{}\n  - {}\n  - {}\n  - {}\n",
                "[scaffold]".bright_black(),
                "alias add".green(),
                format!("{source} {alias}").bold(),
                "TODO: Persist alias to metadata and validate uniqueness.",
                "Add `aliases: Vec<String>` to Source with #[serde(default)] and reload existing metadata.",
                "Relax alias validation to allow @scope/package; keep canonical source strict.",
                "Consider adding a resolver: id > source > alias (prompt on ambiguous).",
            );
        },
        AliasCommand::Rm { source, alias } => {
            println!(
                "{} {} {}\n\n{}\n  - {}\n  - {}\n",
                "[scaffold]".bright_black(),
                "alias rm".yellow(),
                format!("{source} {alias}").bold(),
                "TODO: Remove alias from metadata and save.",
                "Ensure at least one canonical source remains.",
                "Update list/status JSON to reflect alias removal.",
            );
        },
    }
    Ok(())
}
