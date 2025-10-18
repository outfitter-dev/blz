//! Run command tool for executing whitelisted diagnostic commands

use blz_core::Storage;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::path::Path;

use crate::error::{McpError, McpResult};

/// Whitelisted commands that can be executed
const WHITELISTED_COMMANDS: &[&str] =
    &["list", "stats", "history", "validate", "inspect", "schema"];

/// Parameters for run-command tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCommandParams {
    /// Command to execute (must be whitelisted)
    pub command: String,

    /// Optional source argument (for commands that operate on a specific source)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Output from run-command tool
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCommandOutput {
    /// Command that was executed
    pub command: String,
    /// Exit code (0 for success)
    pub exit_code: i32,
    /// Standard output (sanitized)
    pub stdout: String,
    /// Standard error (sanitized)
    #[serde(skip_serializing_if = "String::is_empty")]
    pub stderr: String,
}

/// Sanitize output by replacing absolute paths with relative paths
#[tracing::instrument(skip(output, root_dir))]
fn sanitize_output(output: &str, root_dir: &Path) -> String {
    let start = std::time::Instant::now();

    let root_str = root_dir.to_string_lossy();
    let mut sanitized = output.replace(root_str.as_ref(), "<root>");

    // Also sanitize any home directory references
    if let Some(home) = directories::BaseDirs::new() {
        let home_str = home.home_dir().to_string_lossy();
        sanitized = sanitized.replace(home_str.as_ref(), "~");

        let elapsed = start.elapsed();
        tracing::debug!(
            elapsed_micros = elapsed.as_micros(),
            original_len = output.len(),
            sanitized_len = sanitized.len(),
            "output sanitized"
        );
    } else {
        let elapsed = start.elapsed();
        tracing::debug!(
            elapsed_micros = elapsed.as_micros(),
            original_len = output.len(),
            sanitized_len = sanitized.len(),
            "output sanitized (no home dir)"
        );
    }

    sanitized
}

/// Execute the list command
#[tracing::instrument(skip(storage))]
fn execute_list(
    storage: &Storage,
    #[allow(clippy::used_underscore_binding)] _source: Option<&str>,
) -> McpResult<(String, String)> {
    let sources = storage.list_sources();

    if sources.is_empty() {
        return Ok(("No sources installed.".to_string(), String::new()));
    }

    let mut output = format!("Installed sources ({}):\n", sources.len());
    for source in sources {
        // Try to get metadata for each source
        if let Ok(Some(metadata)) = storage.load_source_metadata(&source) {
            let _ = writeln!(
                output,
                "  {} - {}",
                source,
                metadata.fetched_at.format("%Y-%m-%d %H:%M:%S")
            );
        } else {
            let _ = writeln!(output, "  {source}");
        }
    }

    Ok((output, String::new()))
}

/// Execute the stats command
#[tracing::instrument(skip(storage))]
fn execute_stats(storage: &Storage, source: Option<&str>) -> McpResult<(String, String)> {
    if let Some(source_name) = source {
        // Stats for a specific source
        let metadata = storage
            .load_source_metadata(source_name)?
            .ok_or_else(|| McpError::SourceNotFound(source_name.to_string()))?;

        let output = format!(
            "Source: {}\n\
             URL: {}\n\
             Fetched: {}\n\
             ETag: {}\n\
             Variant: {:?}\n",
            source_name,
            metadata.url,
            metadata.fetched_at.format("%Y-%m-%d %H:%M:%S"),
            metadata.etag.as_ref().map_or("none", |e| e.as_str()),
            metadata.variant
        );

        Ok((output, String::new()))
    } else {
        // Overall stats
        let sources = storage.list_sources();
        let total_sources = sources.len();

        let mut sources_with_metadata = 0;

        for source in &sources {
            if storage
                .load_source_metadata(source)
                .is_ok_and(|m| m.is_some())
            {
                sources_with_metadata += 1;
            }
        }

        let output = format!(
            "Total sources: {total_sources}\n\
             Sources with metadata: {sources_with_metadata}\n"
        );

        Ok((output, String::new()))
    }
}

/// Execute the history command (placeholder - not fully implemented in core)
#[tracing::instrument(skip(storage))]
fn execute_history(storage: &Storage, source: Option<&str>) -> McpResult<(String, String)> {
    if let Some(source_name) = source {
        // Check if archive directory exists
        let archive_dir = storage.archive_dir(source_name)?;

        if !archive_dir.exists() {
            return Ok((
                format!("No history available for source '{source_name}'"),
                String::new(),
            ));
        }

        // List archive entries
        let entries = std::fs::read_dir(&archive_dir)
            .map_err(|e| McpError::Internal(format!("Failed to read archive dir: {e}")))?;

        let mut archives = Vec::new();
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                archives.push(name.to_string());
            }
        }

        archives.sort();

        if archives.is_empty() {
            return Ok((
                format!("No archived versions for source '{source_name}'"),
                String::new(),
            ));
        }

        let archives_str = archives.join("\n  ");
        let output = format!("Archive history for '{source_name}':\n  {archives_str}\n");

        Ok((output, String::new()))
    } else {
        Ok((
            "History command requires a source argument".to_string(),
            String::new(),
        ))
    }
}

/// Execute the validate command
#[tracing::instrument(skip(storage))]
fn execute_validate(storage: &Storage, source: Option<&str>) -> McpResult<(String, String)> {
    if let Some(source_name) = source {
        // Validate a specific source
        if !storage.exists(source_name) {
            return Ok((
                String::new(),
                format!("Source '{source_name}' does not exist"),
            ));
        }

        // Check for required files
        let llms_txt_path = storage.llms_txt_path(source_name)?;
        let llms_json_path = storage.llms_json_path(source_name)?;
        let index_dir = storage.index_dir(source_name)?;

        let mut issues = Vec::new();

        if !llms_txt_path.exists() {
            issues.push("Missing llms.txt file");
        }

        if !llms_json_path.exists() {
            issues.push("Missing llms.json file");
        }

        if !index_dir.exists() {
            issues.push("Missing index directory");
        }

        if issues.is_empty() {
            Ok((format!("Source '{source_name}' is valid"), String::new()))
        } else {
            let issues_str = issues.join("\n  ");
            Ok((
                String::new(),
                format!("Source '{source_name}' has issues:\n  {issues_str}"),
            ))
        }
    } else {
        // Validate all sources
        let sources = storage.list_sources();
        let mut valid_count = 0;
        let mut invalid_sources = Vec::new();

        for source in sources {
            let llms_txt_path = storage.llms_txt_path(&source)?;
            let llms_json_path = storage.llms_json_path(&source)?;
            let index_dir = storage.index_dir(&source)?;

            if llms_txt_path.exists() && llms_json_path.exists() && index_dir.exists() {
                valid_count += 1;
            } else {
                invalid_sources.push(source);
            }
        }

        let mut output = format!("Valid sources: {valid_count}\n");
        if !invalid_sources.is_empty() {
            let invalid_count = invalid_sources.len();
            let invalid_list = invalid_sources.join("\n  ");
            let _ = write!(output, "Invalid sources: {invalid_count}\n  {invalid_list}");
        }

        Ok((output, String::new()))
    }
}

/// Execute the inspect command
#[tracing::instrument(skip(storage))]
fn execute_inspect(storage: &Storage, source: Option<&str>) -> McpResult<(String, String)> {
    if let Some(source_name) = source {
        if !storage.exists(source_name) {
            return Err(McpError::SourceNotFound(source_name.to_string()));
        }

        let tool_dir = storage.tool_dir(source_name)?;
        let llms_txt_path = storage.llms_txt_path(source_name)?;
        let llms_json_path = storage.llms_json_path(source_name)?;
        let index_dir = storage.index_dir(source_name)?;
        let metadata_path = storage.metadata_path(source_name)?;

        let mut output = format!("Source: {source_name}\n\n");
        output.push_str("File locations:\n");

        let paths = vec![
            ("Tool directory", tool_dir),
            ("llms.txt", llms_txt_path),
            ("llms.json", llms_json_path),
            ("Index directory", index_dir),
            ("Metadata", metadata_path),
        ];

        for (name, path) in paths {
            let exists = if path.exists() { "exists" } else { "missing" };
            let _ = writeln!(output, "  {name}: {} ({exists})", path.display());
        }

        Ok((output, String::new()))
    } else {
        Ok((
            "Inspect command requires a source argument".to_string(),
            String::new(),
        ))
    }
}

/// Execute the schema command (returns information about BLZ's data structure)
#[tracing::instrument(skip(storage))]
fn execute_schema(
    storage: &Storage,
    #[allow(clippy::used_underscore_binding)] _source: Option<&str>,
) -> McpResult<(String, String)> {
    let output = format!(
        "BLZ storage schema:\n\n\
         Root directory: {}\n\
         Config directory: {}\n\n\
         Per-source structure:\n\
         <root>/sources/<alias>/\n\
         ├── llms.txt         # Cached documentation content\n\
         ├── llms.json        # Parsed structure (TOC, line map)\n\
         ├── metadata.json    # Source metadata (URL, fetch time, ETag)\n\
         ├── .index/          # Tantivy search index\n\
         └── .archive/        # Historical snapshots\n",
        storage.root_dir().display(),
        storage.config_dir().display()
    );

    Ok((output, String::new()))
}

/// Main handler for run-command tool
#[tracing::instrument(skip(storage))]
pub async fn handle_run_command(
    params: RunCommandParams,
    storage: &Storage,
) -> McpResult<RunCommandOutput> {
    let start = std::time::Instant::now();

    // Validate command is whitelisted
    if !WHITELISTED_COMMANDS.contains(&params.command.as_str()) {
        return Err(McpError::UnsupportedCommand(format!(
            "'{}' is not a supported command. Allowed: {}",
            params.command,
            WHITELISTED_COMMANDS.join(", ")
        )));
    }

    tracing::debug!(command = %params.command, source = ?params.source, "executing command");

    // Execute the command using internal APIs
    let (stdout, stderr) = match params.command.as_str() {
        "list" => execute_list(storage, params.source.as_deref())?,
        "stats" => execute_stats(storage, params.source.as_deref())?,
        "history" => execute_history(storage, params.source.as_deref())?,
        "validate" => execute_validate(storage, params.source.as_deref())?,
        "inspect" => execute_inspect(storage, params.source.as_deref())?,
        "schema" => execute_schema(storage, params.source.as_deref())?,
        _ => {
            return Err(McpError::UnsupportedCommand(format!(
                "Command '{}' is whitelisted but not implemented",
                params.command
            )));
        },
    };

    // Sanitize output
    let root_dir = storage.root_dir();
    let sanitized_stdout = sanitize_output(&stdout, root_dir);
    let sanitized_stderr = sanitize_output(&stderr, root_dir);

    let exit_code = i32::from(!stderr.is_empty());

    let elapsed = start.elapsed();
    tracing::debug!(
        elapsed_micros = elapsed.as_micros(),
        exit_code,
        "command execution completed"
    );

    Ok(RunCommandOutput {
        command: params.command,
        exit_code,
        stdout: sanitized_stdout,
        stderr: sanitized_stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_output() {
        let root = std::path::Path::new("/Users/test/.blz");
        let output = "File at /Users/test/.blz/sources/bun/llms.txt";
        let sanitized = sanitize_output(output, root);
        assert!(sanitized.contains("<root>"));
        assert!(!sanitized.contains("/Users/test/.blz"));
    }

    #[test]
    fn test_whitelist_validation() {
        assert!(WHITELISTED_COMMANDS.contains(&"list"));
        assert!(WHITELISTED_COMMANDS.contains(&"stats"));
        assert!(WHITELISTED_COMMANDS.contains(&"history"));
        assert!(WHITELISTED_COMMANDS.contains(&"validate"));
        assert!(WHITELISTED_COMMANDS.contains(&"inspect"));
        assert!(WHITELISTED_COMMANDS.contains(&"schema"));
        assert!(!WHITELISTED_COMMANDS.contains(&"delete"));
        assert!(!WHITELISTED_COMMANDS.contains(&"add"));
    }

    #[tokio::test]
    async fn test_reject_non_whitelisted() {
        let storage = Storage::new().expect("Failed to create storage");
        let params = RunCommandParams {
            command: "delete".to_string(),
            source: None,
        };

        let result = handle_run_command(params, &storage).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, McpError::UnsupportedCommand(_)));
        }
    }
}
