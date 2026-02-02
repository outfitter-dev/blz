//! Command to display detailed information about a cached source

use anyhow::{Context, Result};
use blz_core::Storage;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::output::OutputFormat;
use crate::output::render::render;
use crate::output::shapes::{FilterStatsOutput, OutputShape, SourceInfoOutput};
use crate::utils::count_headings;

/// Execute the info command.
///
/// # Errors
///
/// Returns an error if storage access, metadata loading, or serialization fails.
pub async fn execute_info(alias: &str, format: OutputFormat) -> Result<()> {
    let storage = Storage::new()?;

    // Resolve metadata alias to canonical if needed
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .unwrap_or_else(|| alias.to_string());

    if !storage.exists(&canonical) {
        anyhow::bail!("Source '{alias}' not found. Run `blz list` to see available sources.");
    }

    let llms = storage
        .load_llms_json(&canonical)
        .with_context(|| format!("Failed to load metadata for '{canonical}'"))?;
    let metadata = llms.metadata.clone();

    let llms_file = storage.llms_txt_path(&canonical)?;

    // Read file stats
    let file_metadata = fs::metadata(&llms_file)
        .with_context(|| format!("Failed to read source file for '{canonical}'"))?;

    let size_bytes = file_metadata.len();
    let lines = llms.line_index.total_lines;
    let headings = count_headings(&llms.toc);

    let cache_path = llms_file
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default()
        .display()
        .to_string();

    // Build the output shape
    let mut info = SourceInfoOutput::new(
        canonical,
        metadata.url.clone(),
        format!("{:?}", metadata.variant),
        lines,
        headings,
        size_bytes,
        cache_path,
    )
    .with_aliases(metadata.aliases.clone())
    .with_last_updated(metadata.fetched_at.to_rfc3339())
    .with_checksum(metadata.sha256);

    if let Some(etag) = metadata.etag {
        info = info.with_etag(etag);
    }

    if let Some(stats) = llms.filter_stats {
        info = info.with_filter_stats(FilterStatsOutput {
            enabled: stats.enabled,
            headings_total: stats.headings_total,
            headings_accepted: stats.headings_accepted,
            headings_rejected: stats.headings_rejected,
            reason: stats.reason,
        });
    }

    // Render the output using unified renderer
    let shape: OutputShape = info.into();
    let mut stdout = io::stdout();
    render(&shape, format, &mut stdout)?;

    Ok(())
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::fs;

    use tempfile::TempDir;

    use crate::output::OutputFormat;
    use crate::utils::test_support;

    struct EnvGuard {
        key: &'static str,
        original: Option<OsString>,
    }

    impl EnvGuard {
        fn new(key: &'static str) -> Self {
            Self {
                key,
                original: std::env::var_os(key),
            }
        }

        fn set<S: AsRef<std::ffi::OsStr>>(&self, value: S) {
            // SAFETY: Environment mutations are synchronised via env_mutex() to avoid
            // concurrent access across tests, and `self.key` is a static str while
            // `value` implements AsRef<OsStr>, satisfying set_var requirements.
            unsafe {
                std::env::set_var(self.key, value);
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: Protected by env_mutex(); see set().
            unsafe {
                if let Some(value) = self.original.clone() {
                    std::env::set_var(self.key, value);
                } else {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    fn test_execute_info_returns_context_for_invalid_metadata() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create tokio runtime");

        let temp = TempDir::new().expect("create temp dir");
        let data_dir = temp.path().join("data");
        let config_dir = temp.path().join("config");
        fs::create_dir_all(&data_dir).expect("create data dir");
        fs::create_dir_all(&config_dir).expect("create config dir");

        let data_dir_str = data_dir.to_string_lossy().to_string();
        let config_dir_str = config_dir.to_string_lossy().to_string();
        let error = {
            let env_lock = test_support::env_mutex()
                .lock()
                .expect("env mutex poisoned");
            let data_guard = EnvGuard::new("BLZ_DATA_DIR");
            data_guard.set(&data_dir_str);
            let config_guard = EnvGuard::new("BLZ_GLOBAL_CONFIG_DIR");
            config_guard.set(&config_dir_str);

            let storage = Storage::new().expect("initialize storage");
            storage
                .ensure_tool_dir("demo")
                .expect("create alias directory");
            let llms_path = storage
                .llms_json_path("demo")
                .expect("resolve llms.json path");
            fs::write(&llms_path, "{ invalid json").expect("write malformed llms.json");

            let error = runtime
                .block_on(execute_info("demo", OutputFormat::Json))
                .expect_err("expected invalid metadata to error");

            drop(config_guard);
            drop(data_guard);
            drop(env_lock);

            error
        };
        let message = error.to_string();

        assert!(
            message.contains("Failed to load metadata for 'demo'"),
            "missing context in error: {message}"
        );
        let chain_messages: Vec<String> = error
            .chain()
            .map(std::string::ToString::to_string)
            .collect();
        assert!(
            chain_messages.len() >= 2,
            "Expected error chain with context and source, got: {chain_messages:?}"
        );
        assert!(
            chain_messages
                .iter()
                .any(|m| m.contains("Failed to parse llms.json")),
            "missing parse failure detail: {chain_messages:?}"
        );
    }
}
