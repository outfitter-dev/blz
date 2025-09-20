use anyhow::{Result, anyhow};
use clap::{Subcommand, ValueEnum};
use dialoguer::Select;
use std::fmt::Write as _;

use crate::utils::preferences;
use crate::utils::settings::{self, PreferenceScope};

#[derive(Debug, Clone, Copy)]
pub enum ConfigKey {
    AddPreferFull,
}

impl ConfigKey {
    fn parse(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "add.prefer_full" | "add.prefer-full" | "add.preferfull" => Some(Self::AddPreferFull),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::AddPreferFull => "add.prefer_full",
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ConfigScopeArg {
    Global,
    Local,
    Project,
}

impl ConfigScopeArg {
    const fn to_scope(self) -> PreferenceScope {
        match self {
            Self::Global => PreferenceScope::Global,
            Self::Local => PreferenceScope::Local,
            Self::Project => PreferenceScope::Project,
        }
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigCommand {
    Set {
        key: String,
        value: String,
        #[arg(long = "scope", value_enum, default_value = "global")]
        scope: ConfigScopeArg,
    },
    Get {
        key: Option<String>,
        #[arg(long = "scope", value_enum)]
        scope: Option<ConfigScopeArg>,
    },
}

pub fn run(command: Option<ConfigCommand>) -> Result<()> {
    match command {
        Some(ConfigCommand::Set { key, value, scope }) => set_value(&key, &value, scope),
        Some(ConfigCommand::Get { key, scope }) => get_value(key.as_deref(), scope),
        None => interactive_menu(),
    }
}

fn set_value(raw_key: &str, raw_value: &str, scope_arg: ConfigScopeArg) -> Result<()> {
    let key = ConfigKey::parse(raw_key)
        .ok_or_else(|| anyhow!("unknown configuration key '{raw_key}'"))?;
    let value = parse_bool(raw_value)?;

    match key {
        ConfigKey::AddPreferFull => {
            settings::set_prefer_llms_full(scope_arg.to_scope(), value)?;
            println!(
                "Set {} = {} for {} scope",
                key.as_str(),
                value,
                scope_display(scope_arg)
            );
        },
    }

    Ok(())
}

fn get_value(raw_key: Option<&str>, scope_arg: Option<ConfigScopeArg>) -> Result<()> {
    match (raw_key, scope_arg) {
        (Some(key), Some(scope)) => {
            let parsed = ConfigKey::parse(key)
                .ok_or_else(|| anyhow!("unknown configuration key '{key}'"))?;
            print_specific_value(parsed, scope);
        },
        (Some(key), None) => {
            let parsed = ConfigKey::parse(key)
                .ok_or_else(|| anyhow!("unknown configuration key '{key}'"))?;
            print_all_scopes(parsed)?;
        },
        (None, Some(scope)) => {
            print_all_keys_for_scope(scope);
        },
        (None, None) => {
            print_summary()?;
        },
    }
    Ok(())
}

fn parse_bool(raw: &str) -> Result<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => Err(anyhow!("expected boolean value, got '{other}'")),
    }
}

fn interactive_menu() -> Result<()> {
    loop {
        print_summary()?;
        let options = vec![
            "Set global add.prefer_full",
            "Set local add.prefer_full",
            "Set project add.prefer_full",
            "Exit",
        ];
        let choice = Select::new()
            .with_prompt("Select configuration option")
            .items(&options)
            .default(0)
            .interact()?;

        match choice {
            0 => prompt_set(ConfigScopeArg::Global)?,
            1 => prompt_set(ConfigScopeArg::Local)?,
            2 => prompt_set(ConfigScopeArg::Project)?,
            _ => break,
        }
    }

    Ok(())
}

fn prompt_set(scope: ConfigScopeArg) -> Result<()> {
    let current = settings::get_prefer_llms_full(scope.to_scope()).unwrap_or(false);
    let prompt = format!(
        "Set {} (current: {})",
        ConfigKey::AddPreferFull.as_str(),
        current
    );

    let choices = ["true", "false", "cancel"];
    let default_index = usize::from(!current);
    let selection = Select::new()
        .with_prompt(prompt)
        .items(&choices)
        .default(default_index)
        .interact()?;

    match selection {
        0 => settings::set_prefer_llms_full(scope.to_scope(), true)?,
        1 => settings::set_prefer_llms_full(scope.to_scope(), false)?,
        _ => println!("No changes made."),
    }

    Ok(())
}

fn print_summary() -> Result<()> {
    let mut output = String::new();
    let global = settings::get_prefer_llms_full(PreferenceScope::Global);
    let project = settings::get_prefer_llms_full(PreferenceScope::Project);
    let local = settings::get_prefer_llms_full(PreferenceScope::Local);
    let effective = settings::effective_prefer_llms_full();

    writeln!(output, "{}", ConfigKey::AddPreferFull.as_str())?;
    writeln!(
        output,
        "  global : {}",
        display_option_bool(global, "not set")
    )?;
    writeln!(
        output,
        "  project: {}",
        display_option_bool(project, "not set")
    )?;
    writeln!(
        output,
        "  local  : {}",
        display_option_bool(local, "not set")
    )?;
    writeln!(output, "  effective: {effective}")?;

    println!("{output}");
    Ok(())
}

fn print_specific_value(key: ConfigKey, scope: ConfigScopeArg) {
    match key {
        ConfigKey::AddPreferFull => {
            let value = settings::get_prefer_llms_full(scope.to_scope());
            println!(
                "{} [{}] = {}",
                key.as_str(),
                scope_display(scope),
                display_option_bool(value, "not set")
            );
        },
    }
}

fn print_all_scopes(key: ConfigKey) -> Result<()> {
    match key {
        ConfigKey::AddPreferFull => {
            print_summary()?;
        },
    }
    Ok(())
}

fn print_all_keys_for_scope(scope: ConfigScopeArg) {
    let value = settings::get_prefer_llms_full(scope.to_scope());
    println!(
        "{} [{}] = {}",
        ConfigKey::AddPreferFull.as_str(),
        scope_display(scope),
        display_option_bool(value, "not set")
    );
}

fn scope_display(scope: ConfigScopeArg) -> String {
    match scope {
        ConfigScopeArg::Global => "global".to_string(),
        ConfigScopeArg::Local => preferences::local_scope_path().map_or_else(
            || "local".to_string(),
            |p| format!("local ({})", p.display()),
        ),
        ConfigScopeArg::Project => project_scope_display(),
    }
}

fn project_scope_display() -> String {
    if let Ok(dir) = std::env::var("BLZ_CONFIG_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return format!("project ({trimmed})");
        }
    }
    if let Ok(file) = std::env::var("BLZ_CONFIG") {
        if !file.trim().is_empty() {
            return format!("project ({file})");
        }
    }
    "project".to_string()
}

fn display_option_bool(value: Option<bool>, empty: &str) -> String {
    match value {
        Some(true) => "true".to_string(),
        Some(false) => "false".to_string(),
        None => empty.to_string(),
    }
}
