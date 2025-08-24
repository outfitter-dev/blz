//! Shell completions generation

use clap::CommandFactory;
use clap_complete::Shell;

/// Generate shell completions for the specified shell
pub fn generate(shell: Shell) {
    let mut cmd = crate::cli::Cli::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
}
