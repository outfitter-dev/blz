//! Cache clearing command implementation

use anyhow::Result;
use blz_core::Storage;
use colored::Colorize;
use std::io::{self, Write};

/// Abstraction over the storage operations needed by the clear command.
pub trait ClearStorage {
    fn list_sources(&self) -> Result<Vec<String>>;
    fn clear_cache(&self) -> Result<()>;
}

#[allow(clippy::use_self)]
impl ClearStorage for Storage {
    fn list_sources(&self) -> Result<Vec<String>> {
        Ok(self.list_sources())
    }

    fn clear_cache(&self) -> Result<()> {
        Storage::clear_cache(self).map_err(anyhow::Error::from)
    }
}

/// High-level outcome produced by [`execute_clear`]. Useful for assertions in tests.
#[derive(Debug, PartialEq, Eq)]
pub enum ClearOutcome {
    /// No sources were present to clear.
    AlreadyEmpty,
    /// User cancelled the clear operation.
    Cancelled,
    /// Cache cleared with the number of sources removed.
    Cleared { cleared: usize },
}

/// Core clear implementation with injectable dependencies to enable deterministic tests.
///
/// # Errors
///
/// Returns an error if listing sources, confirmation, or cache deletion fails.
pub fn execute_clear<S, W, C>(
    storage: &S,
    mut writer: W,
    force: bool,
    mut confirm: C,
) -> Result<ClearOutcome>
where
    S: ClearStorage,
    W: Write,
    C: FnMut(&[String]) -> Result<bool>,
{
    let sources = storage.list_sources()?;

    if sources.is_empty() {
        writeln!(writer, "{} Cache is already empty", "ℹ".blue())?;
        return Ok(ClearOutcome::AlreadyEmpty);
    }

    writeln!(
        writer,
        "{} This will permanently delete all cached data for {} source(s):",
        "⚠".yellow(),
        sources.len()
    )?;
    for source in &sources {
        writeln!(writer, "  • {source}")?;
    }
    writeln!(writer)?;

    if !force && !confirm(&sources)? {
        writeln!(writer, "{} Cancelled", "✗".red())?;
        return Ok(ClearOutcome::Cancelled);
    }

    storage.clear_cache()?;

    writeln!(writer, "{} Cache cleared successfully", "✓".green())?;
    writeln!(writer)?;
    writeln!(writer, "To re-add sources, use:")?;
    writeln!(writer, "  blz add <alias> <url>")?;

    Ok(ClearOutcome::Cleared {
        cleared: sources.len(),
    })
}

/// Clears the entire cache directory using the real storage and terminal IO.
///
/// # Errors
///
/// Returns an error if storage access, user confirmation, or deletion fails.
pub fn run(force: bool) -> Result<()> {
    let storage = Storage::new()?;
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();
    let mut input = String::new();

    execute_clear(&storage, &mut stdout_lock, force, |_sources| {
        let prompt_stdout = io::stdout();
        let mut prompt_lock = prompt_stdout.lock();
        write!(prompt_lock, "Are you sure you want to continue? [y/N] ")?;
        prompt_lock.flush()?;

        input.clear();
        io::stdin().read_line(&mut input)?;

        Ok(matches!(
            input.trim().to_ascii_lowercase().as_str(),
            "y" | "yes"
        ))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct MockStorage {
        sources: Vec<String>,
        cleared: RefCell<bool>,
    }

    impl ClearStorage for MockStorage {
        fn list_sources(&self) -> Result<Vec<String>> {
            Ok(self.sources.clone())
        }

        fn clear_cache(&self) -> Result<()> {
            *self.cleared.borrow_mut() = true;
            Ok(())
        }
    }

    #[test]
    fn execute_clear_reports_empty_cache() -> Result<()> {
        let storage = MockStorage::default();
        let mut output = Vec::new();

        let outcome = execute_clear(&storage, &mut output, false, |_| Ok(true))?;

        assert_eq!(outcome, ClearOutcome::AlreadyEmpty);
        let rendered = String::from_utf8(output).expect("valid utf8");
        assert!(rendered.contains("Cache is already empty"));
        Ok(())
    }

    #[test]
    fn execute_clear_skips_confirmation_when_forced() -> Result<()> {
        let storage = MockStorage {
            sources: vec!["alpha".into(), "beta".into()],
            cleared: RefCell::new(false),
        };
        let mut output = Vec::new();

        let outcome = execute_clear(&storage, &mut output, true, |_| {
            anyhow::bail!("confirmation should not be requested when forced");
        })?;

        assert_eq!(outcome, ClearOutcome::Cleared { cleared: 2 });
        assert!(*storage.cleared.borrow());
        let rendered = String::from_utf8(output).expect("valid utf8");
        assert!(rendered.contains("Cache cleared successfully"));
        Ok(())
    }

    #[test]
    fn execute_clear_honours_cancellation() -> Result<()> {
        let storage = MockStorage {
            sources: vec!["only".into()],
            cleared: RefCell::new(false),
        };
        let mut output = Vec::new();

        let outcome = execute_clear(&storage, &mut output, false, |_| Ok(false))?;

        assert_eq!(outcome, ClearOutcome::Cancelled);
        assert!(!*storage.cleared.borrow());
        let rendered = String::from_utf8(output).expect("valid utf8");
        assert!(rendered.contains("Cancelled"));
        Ok(())
    }

    #[test]
    fn execute_clear_runs_when_confirmed() -> Result<()> {
        let storage = MockStorage {
            sources: vec!["pkg".into(), "core".into(), "docs".into()],
            cleared: RefCell::new(false),
        };
        let mut output = Vec::new();

        let outcome = execute_clear(&storage, &mut output, false, |_| Ok(true))?;

        assert_eq!(outcome, ClearOutcome::Cleared { cleared: 3 });
        assert!(*storage.cleared.borrow());
        let rendered = String::from_utf8(output).expect("valid utf8");
        assert!(rendered.contains("pkg"));
        assert!(rendered.contains("Cache cleared successfully"));
        Ok(())
    }
}
