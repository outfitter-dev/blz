//! Claude plugin install helpers for local development.

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use std::path::{Path, PathBuf};
use std::process::Command;

const INSTALL_SCRIPT: &str = "scripts/install-claude-plugin-local.sh";

/// Installation scope for Claude plugin installs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum ClaudePluginScope {
    /// Install for the current user.
    #[value(name = "user")]
    User,
    /// Install for the current project.
    #[value(name = "project")]
    Project,
}

/// Subcommands for `blz claude-plugin`.
#[derive(Subcommand, Clone, Copy, Debug)]
pub enum ClaudePluginCommands {
    /// Install the local Claude plugin from this repository.
    Install {
        /// Installation scope for Claude Code.
        #[arg(long, value_enum, default_value = "user")]
        scope: ClaudePluginScope,
    },
}

impl ClaudePluginScope {
    const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }
}

/// Dispatch a `ClaudePlugin` command.
pub fn dispatch(command: ClaudePluginCommands) -> Result<()> {
    match command {
        ClaudePluginCommands::Install { scope } => {
            install_local_plugin(scope)?;
        },
    }
    Ok(())
}

pub fn install_local_plugin(scope: ClaudePluginScope) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to resolve current directory")?;
    let repo_root = find_repo_root(&cwd).ok_or_else(|| {
        anyhow::anyhow!(
            "Could not locate the blz repo root. Run this command from inside the repo."
        )
    })?;
    let script_path = repo_root.join(INSTALL_SCRIPT);

    if !script_path.is_file() {
        bail!(
            "Local plugin installer not found at {}",
            script_path.display()
        );
    }

    let mut command = Command::new("bash");
    command.arg(&script_path);
    command.arg("--install");
    command.arg("--scope");
    command.arg(scope.as_str());

    let status = command
        .current_dir(&repo_root)
        .status()
        .with_context(|| format!("Failed to run {}", script_path.display()))?;

    if !status.success() {
        let code = status
            .code()
            .map_or_else(|| "signal".to_string(), |value| value.to_string());
        bail!("Local plugin install failed (exit code: {code})");
    }

    Ok(())
}

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        let plugin_manifest = dir.join(".claude-plugin").join("plugin.json");
        let agents_dir = dir.join("packages").join("agents");
        let script_path = dir.join(INSTALL_SCRIPT);
        if plugin_manifest.is_file() && agents_dir.is_dir() && script_path.is_file() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}
