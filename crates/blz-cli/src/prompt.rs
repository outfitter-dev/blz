use crate::cli::Commands;
use crate::output::OutputFormat;
use serde_json::json;

const GLOBAL_PROMPT: &str = include_str!("prompts/blz.prompt.json");
const ADD_PROMPT: &str = include_str!("prompts/add.prompt.json");
const SEARCH_PROMPT: &str = include_str!("prompts/search.prompt.json");
const GET_PROMPT: &str = include_str!("prompts/get.prompt.json");
const LIST_PROMPT: &str = include_str!("prompts/list.prompt.json");
const UPDATE_PROMPT: &str = include_str!("prompts/update.prompt.json");
const REMOVE_PROMPT: &str = include_str!("prompts/remove.prompt.json");
const LOOKUP_PROMPT: &str = include_str!("prompts/lookup.prompt.json");
const DOCS_PROMPT: &str = include_str!("prompts/docs.prompt.json");
const HISTORY_PROMPT: &str = include_str!("prompts/history.prompt.json");
const COMPLETIONS_PROMPT: &str = include_str!("prompts/completions.prompt.json");
const ALIAS_PROMPT: &str = include_str!("prompts/alias.prompt.json");
const REGISTRY_PROMPT: &str = include_str!("prompts/registry.prompt.json");
const CLEAR_PROMPT: &str = include_str!("prompts/clear.prompt.json");
const INFO_PROMPT: &str = include_str!("prompts/info.prompt.json");
const STATS_PROMPT: &str = include_str!("prompts/stats.prompt.json");
const VALIDATE_PROMPT: &str = include_str!("prompts/validate.prompt.json");
const DOCTOR_PROMPT: &str = include_str!("prompts/doctor.prompt.json");
const DIFF_PROMPT: &str = include_str!("prompts/diff.prompt.json");

#[derive(Clone, Copy)]
pub enum NoteChannel {
    Auto,
    ForceStderr,
}

pub fn emit(target: &str, command: Option<&Commands>) -> anyhow::Result<()> {
    let normalized = normalize_target(target, command);
    let prompt = match normalized.as_str() {
        "blz" | "global" => Some(GLOBAL_PROMPT),
        "add" => Some(ADD_PROMPT),
        "search" => Some(SEARCH_PROMPT),
        "get" => Some(GET_PROMPT),
        "list" | "sources" => Some(LIST_PROMPT),
        "update" => Some(UPDATE_PROMPT),
        "remove" | "rm" | "delete" => Some(REMOVE_PROMPT),
        "lookup" => Some(LOOKUP_PROMPT),
        "docs" => Some(DOCS_PROMPT),
        "history" => Some(HISTORY_PROMPT),
        "completions" => Some(COMPLETIONS_PROMPT),
        "alias" | "alias.add" | "alias.remove" | "alias.list" => Some(ALIAS_PROMPT),
        "registry" | "registry.create-source" | "registry.update-source" => Some(REGISTRY_PROMPT),
        "clear" => Some(CLEAR_PROMPT),
        "info" => Some(INFO_PROMPT),
        "stats" => Some(STATS_PROMPT),
        "validate" => Some(VALIDATE_PROMPT),
        "doctor" => Some(DOCTOR_PROMPT),
        "diff" => Some(DIFF_PROMPT),
        _ => None,
    };

    if let Some(content) = prompt {
        println!("{}", content.trim());
        return Ok(());
    }

    let error = json!({
        "error": "unknown_prompt_target",
        "target": normalized,
        "available": available_targets(),
    });
    eprintln!("{}", serde_json::to_string_pretty(&error)?);
    Err(anyhow::anyhow!("unknown_prompt_target"))
}

fn normalize_target(target: &str, command: Option<&Commands>) -> String {
    if target == "__global__" {
        return "blz".into();
    }

    if target == "__auto__" {
        if let Some(cmd) = command {
            return match cmd {
                Commands::Completions { .. } => "completions".into(),
                Commands::Alias { .. } => "alias".into(),
                Commands::Docs { .. } => "docs".into(),
                Commands::Registry { .. } => "registry".into(),
                Commands::Search { .. } => "search".into(),
                Commands::Instruct => "blz".into(),
                Commands::Add(_) => "add".into(),
                Commands::Update { .. } => "update".into(),
                Commands::Remove { .. } => "remove".into(),
                Commands::List { .. } => "list".into(),
                Commands::Find { .. } => "find".into(),
                Commands::Get { .. } => "get".into(),
                Commands::Lookup { .. } => "lookup".into(),
                Commands::History { .. } => "history".into(),
                Commands::Info { .. } => "info".into(),
                Commands::Stats { .. } => "stats".into(),
                Commands::Validate { .. } => "validate".into(),
                Commands::Doctor { .. } => "doctor".into(),
                Commands::Clear { .. } => "clear".into(),
                Commands::Diff { .. } => "diff".into(),
                Commands::Mcp => "mcp".into(),
                Commands::Anchor { .. } | Commands::Anchors { .. } => "anchor".into(),
            };
        }
        return "blz".into();
    }

    target
        .trim()
        .trim_matches('"')
        .replace(['/', ':'], ".")
        .to_ascii_lowercase()
}

fn available_targets() -> Vec<&'static str> {
    vec![
        "blz",
        "add",
        "search",
        "get",
        "list",
        "update",
        "remove",
        "lookup",
        "docs",
        "history",
        "anchor",
        "completions",
        "alias",
        "registry",
        "clear",
        "info",
        "stats",
        "validate",
        "doctor",
        "diff",
    ]
}

pub fn emit_registry_note(format: OutputFormat, quiet: bool, channel: NoteChannel) {
    const NOTE: &str = "Note: BLZ's built-in llms.txt registry is nascent. For now you can still take any llms-full.txt you find and add it locally. If you want to submit one to the BLZ registry, just file a PR at https://github.com/outfitter-dev/blz!";
    match channel {
        NoteChannel::ForceStderr => eprintln!("{NOTE}"),
        NoteChannel::Auto => {
            if matches!(format, OutputFormat::Text) {
                if quiet {
                    eprintln!("{NOTE}");
                } else {
                    println!("\n{NOTE}");
                }
            } else {
                eprintln!("{NOTE}");
            }
        },
    }
}
