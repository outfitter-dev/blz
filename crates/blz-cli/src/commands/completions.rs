//! Shell completions generation

use clap::CommandFactory;
use clap_complete::Shell;
use serde_json::json;

/// Generate shell completions for the specified shell
pub fn generate(shell: Shell) {
    let mut cmd = crate::cli::Cli::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
}

/// List supported shells as text/JSON/NDJSON
pub fn list_supported(format: crate::output::OutputFormat) {
    let shells = vec![
        ("bash", "~/.local/share/bash-completion/completions/blz"),
        ("zsh", "~/.zsh/completions/_blz"),
        ("fish", "~/.config/fish/completions/blz.fish"),
        ("powershell", "$PROFILE"),
        ("elvish", "~/.elvish/lib/blz.elv"),
    ];
    match format {
        crate::output::OutputFormat::Text => {
            println!("Supported shells:\n");
            for (name, path) in &shells {
                println!("  - {name} (install to {path})");
            }
        },
        crate::output::OutputFormat::Json => {
            let arr: Vec<_> = shells
                .iter()
                .map(|(name, path)| json!({"shell": name, "installPath": path}))
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&arr).unwrap_or_else(|_| "[]".to_string())
            );
        },
        crate::output::OutputFormat::Jsonl => {
            for (name, path) in &shells {
                println!("{}", json!({"shell": name, "installPath": path}));
            }
        },
    }
}
