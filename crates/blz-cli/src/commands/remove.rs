//! Remove command implementation

use std::fs;
use std::io::{self, IsTerminal, Write};

use anyhow::Result;
use blz_core::{LlmsJson, Storage};
use colored::Colorize;
use inquire::Confirm;
use serde::Serialize;

/// Abstraction over the storage interactions needed by the remove command.
pub trait RemoveStorage {
    fn exists(&self, alias: &str) -> Result<bool>;
    fn load_removal_info(&self, alias: &str) -> Result<Option<RemovalInfo>>;
    fn delete_source(&self, alias: &str) -> Result<()>;
}

impl RemoveStorage for Storage {
    fn exists(&self, alias: &str) -> Result<bool> {
        Ok(self.exists(alias))
    }

    fn load_removal_info(&self, alias: &str) -> Result<Option<RemovalInfo>> {
        Self::load_llms_json(self, alias)
            .map(|llms| Some(RemovalInfo::from_llms(alias, &llms)))
            .or_else(|_| Ok(None))
    }

    fn delete_source(&self, alias: &str) -> Result<()> {
        let dir = self.tool_dir(alias)?;
        fs::remove_dir_all(&dir).map_err(|e| {
            anyhow::anyhow!("Failed to remove source directory '{}': {e}", dir.display())
        })
    }
}

/// Summary displayed before a source is removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemovalInfo {
    /// Source alias.
    pub alias: String,
    /// Source URL.
    pub url: String,
    /// Total number of lines in the cached document.
    pub total_lines: usize,
    /// Timestamp when the source was fetched.
    pub fetched_at: String,
}

impl RemovalInfo {
    fn from_llms(alias: &str, llms: &LlmsJson) -> Self {
        Self {
            alias: alias.to_string(),
            url: llms.metadata.url.clone(),
            total_lines: llms.line_index.total_lines,
            fetched_at: llms
                .metadata
                .fetched_at
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
        }
    }
}

/// Result of attempting to remove a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveOutcome {
    /// Source was not found in storage.
    NotFound,
    /// User cancelled removal.
    Cancelled,
    /// Source removed successfully.
    Removed { alias: String },
}

/// Core removal logic with injectable dependencies for testing.
///
/// # Errors
///
/// Returns an error if storage access, confirmation, or deletion fails.
pub fn execute_remove<S, W, F>(
    storage: &S,
    alias: &str,
    writer: &mut W,
    require_confirmation: bool,
    mut confirm: F,
) -> Result<RemoveOutcome>
where
    S: RemoveStorage,
    W: Write + ?Sized,
    F: FnMut(&str, Option<&RemovalInfo>) -> Result<bool>,
{
    if !storage.exists(alias)? {
        writeln!(writer, "Source '{alias}' not found")?;
        return Ok(RemoveOutcome::NotFound);
    }

    let info = storage.load_removal_info(alias)?;

    if let Some(details) = &info {
        writeln!(
            writer,
            "Removing source '{}' ({})",
            alias.red(),
            details.url
        )?;
        writeln!(writer, "  {} lines", details.total_lines)?;
        writeln!(writer, "  Fetched: {}", details.fetched_at)?;
    }

    if require_confirmation {
        let confirmed = confirm(alias, info.as_ref())?;
        if !confirmed {
            writeln!(writer, "Removal cancelled")?;
            return Ok(RemoveOutcome::Cancelled);
        }
    }

    storage.delete_source(alias)?;
    writeln!(
        writer,
        "{} Successfully removed source '{}' and all associated files",
        "✓".green(),
        alias.green()
    )?;

    Ok(RemoveOutcome::Removed {
        alias: alias.to_string(),
    })
}

/// Execute the remove command to delete a source.
///
/// # Errors
///
/// Returns an error if storage access, user confirmation, or deletion fails.
#[allow(clippy::unused_async)]
pub async fn execute(alias: &str, auto_yes: bool, quiet: bool) -> Result<()> {
    let storage = Storage::new()?;

    // Resolve metadata alias to canonical if needed
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .unwrap_or_else(|| alias.to_string());

    let force_non_interactive = std::env::var_os("BLZ_FORCE_NON_INTERACTIVE").is_some();
    let no_tty = !std::io::stdin().is_terminal();
    let require_confirmation = !(auto_yes || force_non_interactive || no_tty);

    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();
    let mut sink = io::sink();
    let writer: &mut dyn Write = if quiet { &mut sink } else { &mut stdout_lock };

    let outcome = execute_remove(
        &storage,
        &canonical,
        writer,
        require_confirmation,
        |alias, _info| {
            let prompt_stdout = io::stdout();
            let mut prompt_lock = prompt_stdout.lock();
            let prompt = format!("Remove source '{alias}' and all cached data?");
            if quiet {
                return Ok(true);
            }
            let confirmed = Confirm::new(&prompt).with_default(false).prompt()?;
            if !confirmed {
                writeln!(prompt_lock)?;
            }
            Ok(confirmed)
        },
    )?;

    // Return error for not-found to ensure proper exit code
    match outcome {
        RemoveOutcome::NotFound => {
            anyhow::bail!("Source '{canonical}' not found");
        },
        RemoveOutcome::Cancelled | RemoveOutcome::Removed { .. } => Ok(()),
    }
}

/// Dispatch a deprecated Remove command.
///
/// This function handles the deprecated `remove` command, printing a deprecation
/// warning and delegating to `execute`.
#[deprecated(since = "1.5.0", note = "use 'rm' instead")]
#[allow(deprecated)]
pub async fn dispatch_deprecated(alias: String, yes: bool, quiet: bool) -> Result<()> {
    if !crate::utils::cli_args::deprecation_warnings_suppressed() {
        eprintln!(
            "{}",
            "Warning: 'remove' is deprecated, use 'rm' instead".yellow()
        );
    }
    execute(&alias, yes, quiet).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::Cursor;

    #[derive(Default)]
    struct MockStorage {
        exists: bool,
        info: Option<RemovalInfo>,
        deleted: RefCell<bool>,
        errors: HashMap<&'static str, anyhow::Error>,
    }

    impl RemoveStorage for MockStorage {
        fn exists(&self, _alias: &str) -> Result<bool> {
            if let Some(err) = self.errors.get("exists") {
                return Err(anyhow::anyhow!(err.to_string()));
            }
            Ok(self.exists)
        }

        fn load_removal_info(&self, alias: &str) -> Result<Option<RemovalInfo>> {
            if let Some(err) = self.errors.get("load") {
                return Err(anyhow::anyhow!(err.to_string()));
            }
            Ok(self.info.clone().map(|mut info| {
                info.alias = alias.to_string();
                info
            }))
        }

        fn delete_source(&self, _alias: &str) -> Result<()> {
            if let Some(err) = self.errors.get("delete") {
                return Err(anyhow::anyhow!(err.to_string()));
            }
            *self.deleted.borrow_mut() = true;
            Ok(())
        }
    }

    fn sample_info() -> RemovalInfo {
        RemovalInfo {
            alias: "alpha".into(),
            url: "https://example.com".into(),
            total_lines: 420,
            fetched_at: "2025-10-01 12:00:00".into(),
        }
    }

    #[test]
    fn execute_remove_returns_not_found() -> Result<()> {
        let storage = MockStorage::default();
        let mut buf = Cursor::new(Vec::new());
        let outcome = execute_remove(&storage, "missing", &mut buf, false, |_, _| Ok(true))?;
        assert_eq!(outcome, RemoveOutcome::NotFound);
        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("missing"));
        Ok(())
    }

    #[test]
    fn execute_remove_cancels_when_not_confirmed() -> Result<()> {
        let storage = MockStorage {
            exists: true,
            info: Some(sample_info()),
            ..Default::default()
        };
        let mut buf = Cursor::new(Vec::new());
        let outcome = execute_remove(&storage, "alpha", &mut buf, true, |_, _| Ok(false))?;
        assert_eq!(outcome, RemoveOutcome::Cancelled);
        assert!(!*storage.deleted.borrow());
        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("Removal cancelled"));
        Ok(())
    }

    #[test]
    fn execute_remove_skips_confirmation_when_not_required() -> Result<()> {
        let storage = MockStorage {
            exists: true,
            info: Some(sample_info()),
            ..Default::default()
        };
        let mut buf = Cursor::new(Vec::new());
        let mut confirm_called = false;
        let outcome = execute_remove(&storage, "alpha", &mut buf, false, |_, _| {
            confirm_called = true;
            Ok(true)
        })?;
        assert_eq!(
            outcome,
            RemoveOutcome::Removed {
                alias: "alpha".into()
            }
        );
        assert!(!confirm_called);
        assert!(*storage.deleted.borrow());
        Ok(())
    }

    #[test]
    fn execute_remove_deletes_when_confirmed() -> Result<()> {
        let storage = MockStorage {
            exists: true,
            info: Some(sample_info()),
            ..Default::default()
        };
        let mut buf = Cursor::new(Vec::new());
        let outcome = execute_remove(&storage, "alpha", &mut buf, true, |_, _| Ok(true))?;
        assert_eq!(
            outcome,
            RemoveOutcome::Removed {
                alias: "alpha".into()
            }
        );
        assert!(*storage.deleted.borrow());
        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("Successfully removed"));
        Ok(())
    }
}
