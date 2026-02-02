//! List command implementation

use std::io::{self, Write};

use anyhow::{Context, Result};
use blz_core::{LlmsJson, Source, SourceDescriptor, Storage};

use crate::output::OutputFormat;
use crate::output::render::{SourceListRenderOptions, render_source_list_with_options};
use crate::output::shapes::{SourceListOutput, SourceSummary};
use crate::utils::count_headings;

/// Abstraction over storage interactions required by the list command.
pub trait ListStorage {
    fn list_sources(&self) -> Result<Vec<String>>;
    fn load_metadata(&self, alias: &str) -> Result<Option<Source>>;
    fn load_llms_json(&self, alias: &str) -> Result<LlmsJson>;
    fn load_descriptor(&self, alias: &str) -> Result<Option<SourceDescriptor>>;
}

impl ListStorage for Storage {
    fn list_sources(&self) -> Result<Vec<String>> {
        Ok(self.list_sources())
    }

    fn load_metadata(&self, alias: &str) -> Result<Option<Source>> {
        Self::load_source_metadata(self, alias).map_err(anyhow::Error::from)
    }

    fn load_llms_json(&self, alias: &str) -> Result<LlmsJson> {
        Self::load_llms_json(self, alias).map_err(anyhow::Error::from)
    }

    fn load_descriptor(&self, alias: &str) -> Result<Option<SourceDescriptor>> {
        Self::load_descriptor(self, alias).map_err(anyhow::Error::from)
    }
}

/// Gather source summaries from storage.
///
/// # Errors
///
/// Returns an error if metadata or cached content cannot be loaded.
pub fn collect_source_summaries<S: ListStorage>(storage: &S) -> Result<Vec<SourceSummary>> {
    let aliases = storage.list_sources()?;
    let mut summaries = Vec::new();

    for alias in aliases {
        let metadata = storage
            .load_metadata(&alias)?
            .with_context(|| format!("Failed to load metadata for '{alias}'"))?;
        let llms = storage
            .load_llms_json(&alias)
            .with_context(|| format!("Failed to load JSON for '{alias}'"))?;
        let descriptor = storage.load_descriptor(&alias)?;

        let description = metadata
            .description
            .clone()
            .or_else(|| descriptor.as_ref().and_then(|d| d.description.clone()));
        let category = metadata
            .category
            .clone()
            .or_else(|| descriptor.as_ref().and_then(|d| d.category.clone()));

        let summary = build_source_summary(
            alias,
            &metadata,
            &llms,
            descriptor.as_ref(),
            description,
            category,
        );

        summaries.push(summary);
    }

    Ok(summaries)
}

/// Build a `SourceSummary` from metadata and content.
fn build_source_summary(
    alias: String,
    metadata: &Source,
    llms: &LlmsJson,
    descriptor: Option<&SourceDescriptor>,
    description: Option<String>,
    category: Option<String>,
) -> SourceSummary {
    let mut summary = SourceSummary::new(alias, metadata.url.clone(), llms.line_index.total_lines)
        .with_headings(count_headings(&llms.toc))
        .with_tags(metadata.tags.clone())
        .with_aliases(metadata.aliases.clone())
        .with_fetched_at(metadata.fetched_at.to_rfc3339())
        .with_checksum(llms.metadata.sha256.clone())
        .with_npm_aliases(metadata.npm_aliases.clone())
        .with_github_aliases(metadata.github_aliases.clone());

    if let Some(etag) = &metadata.etag {
        summary = summary.with_etag(etag.clone());
    }

    if let Some(last_modified) = &metadata.last_modified {
        summary = summary.with_last_modified(last_modified.clone());
    }

    if let Some(desc) = description {
        summary = summary.with_description(desc);
    }

    if let Some(cat) = category {
        summary = summary.with_category(cat);
    }

    // Convert origin to JSON value; emit null on serialization failure for backward compatibility
    let origin_value = serde_json::to_value(&metadata.origin).unwrap_or(serde_json::Value::Null);
    summary = summary.with_origin(origin_value);

    // Convert descriptor to JSON value; emit null on serialization failure for backward compatibility
    let descriptor_value = descriptor
        .map(serde_json::to_value)
        .map_or(serde_json::Value::Null, |r| {
            r.unwrap_or(serde_json::Value::Null)
        });
    summary = summary.with_descriptor(descriptor_value);

    summary
}

/// Render a list of source summaries to the provided writer.
pub fn render_list<W: Write>(
    writer: &mut W,
    sources: &[SourceSummary],
    format: OutputFormat,
    status: bool,
    details: bool,
    limit: Option<usize>,
) -> Result<()> {
    // Apply limit to sources slice
    let sources = limit.map_or_else(
        || sources.to_vec(),
        |limit_count| sources[..sources.len().min(limit_count)].to_vec(),
    );

    let output = SourceListOutput::new(sources);
    let options = SourceListRenderOptions {
        show_status: status,
        show_details: details,
    };

    render_source_list_with_options(&output, format, &options, writer)
}

/// Dispatch a List command.
pub async fn dispatch(
    format: crate::utils::cli_args::FormatArg,
    status: bool,
    details: bool,
    limit: Option<usize>,
    quiet: bool,
) -> Result<()> {
    execute(format.resolve(quiet), status, details, limit).await
}

/// Execute the list command using production storage and stdout.
///
/// # Errors
///
/// Returns an error if storage access or output rendering fails.
#[allow(clippy::unused_async)]
pub async fn execute(
    format: OutputFormat,
    status: bool,
    details: bool,
    limit: Option<usize>,
) -> Result<()> {
    let storage = Storage::new()?;
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    execute_with_writer(&storage, &mut handle, format, status, details, limit)
}

/// Testable entry point allowing storage and writer injection.
///
/// # Errors
///
/// Returns an error if storage access or output rendering fails.
pub fn execute_with_writer<S, W>(
    storage: &S,
    writer: &mut W,
    format: OutputFormat,
    status: bool,
    details: bool,
    limit: Option<usize>,
) -> Result<()>
where
    S: ListStorage,
    W: Write,
{
    let summaries = collect_source_summaries(storage)?;

    // Handle empty case for JSONL to maintain backward compatibility
    // (render module outputs nothing for empty JSONL, but original printed "[]")
    if summaries.is_empty() && format == OutputFormat::Jsonl {
        writeln!(writer, "[]")?;
        return Ok(());
    }

    render_list(writer, &summaries, format, status, details, limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Cursor;

    use chrono::{TimeZone, Utc};

    #[derive(Default)]
    struct MockStorage {
        aliases: Vec<String>,
        metadata: HashMap<String, Source>,
        llms: HashMap<String, LlmsJson>,
        descriptors: HashMap<String, SourceDescriptor>,
        fail_on_metadata: bool,
    }

    impl ListStorage for MockStorage {
        fn list_sources(&self) -> Result<Vec<String>> {
            Ok(self.aliases.clone())
        }

        fn load_metadata(&self, alias: &str) -> Result<Option<Source>> {
            if self.fail_on_metadata {
                anyhow::bail!("boom")
            }
            Ok(self.metadata.get(alias).cloned())
        }

        fn load_llms_json(&self, alias: &str) -> Result<LlmsJson> {
            self.llms.get(alias).cloned().context("missing llms json")
        }

        fn load_descriptor(&self, alias: &str) -> Result<Option<SourceDescriptor>> {
            Ok(self.descriptors.get(alias).cloned())
        }
    }

    fn sample_source(url: &str) -> Source {
        Source {
            url: url.to_string(),
            etag: Some("etag123".into()),
            last_modified: Some("Wed, 01 Oct 2025 12:00:00 GMT".into()),
            fetched_at: Utc.with_ymd_and_hms(2025, 10, 1, 12, 0, 0).unwrap(),
            sha256: "test-sha".into(),
            variant: blz_core::SourceVariant::Llms,
            aliases: vec!["pkg".into()],
            tags: vec!["docs".into(), "stable".into()],
            description: Some("Sample description".into()),
            category: Some("library".into()),
            npm_aliases: vec!["pkg".into()],
            github_aliases: vec!["org/pkg".into()],
            origin: blz_core::SourceOrigin {
                manifest: None,
                source_type: Some(blz_core::SourceType::Remote {
                    url: url.to_string(),
                }),
            },
            filter_non_english: Some(true),
        }
    }

    fn sample_llms(alias: &str, metadata: Source, lines: usize, headings: usize) -> LlmsJson {
        LlmsJson {
            source: alias.to_string(),
            metadata,
            toc: heading_entries(headings),
            files: vec![],
            line_index: blz_core::LineIndex {
                total_lines: lines,
                byte_offsets: false,
            },
            diagnostics: vec![],
            parse_meta: None,
            filter_stats: None,
        }
    }

    fn heading_entries(count: usize) -> Vec<blz_core::TocEntry> {
        (0..count)
            .map(|i| blz_core::TocEntry {
                heading_path: vec![format!("Heading {i}")],
                heading_path_display: Some(vec![format!("Heading {i}")]),
                heading_path_normalized: Some(vec![format!("heading {i}")]),
                lines: "1-2".into(),
                anchor: None,
                children: vec![],
            })
            .collect()
    }

    #[test]
    fn collect_source_summaries_empty() -> Result<()> {
        let storage = MockStorage::default();
        let summaries = collect_source_summaries(&storage)?;
        assert!(summaries.is_empty());
        Ok(())
    }

    #[test]
    fn execute_with_writer_renders_empty_text() -> Result<()> {
        let storage = MockStorage::default();
        let mut buf = Cursor::new(Vec::new());
        execute_with_writer(&storage, &mut buf, OutputFormat::Text, false, false, None)?;
        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("No sources configured"));
        Ok(())
    }

    #[test]
    fn execute_with_writer_produces_json() -> Result<()> {
        let metadata = sample_source("https://example.com");
        let storage = MockStorage {
            aliases: vec!["alpha".into()],
            metadata: HashMap::from([(String::from("alpha"), metadata.clone())]),
            llms: HashMap::from([(
                String::from("alpha"),
                sample_llms("alpha", metadata, 120, 12),
            )]),
            descriptors: HashMap::new(),
            fail_on_metadata: false,
        };
        let mut buf = Cursor::new(Vec::new());
        execute_with_writer(&storage, &mut buf, OutputFormat::Json, true, false, None)?;
        let output = String::from_utf8(buf.into_inner())?;
        let value: serde_json::Value = serde_json::from_str(&output)?;
        assert_eq!(value[0]["alias"], "alpha");
        assert!(value[0].get("etag").is_some());
        Ok(())
    }

    #[test]
    fn render_text_omits_status_details_when_disabled() -> Result<()> {
        let origin = blz_core::SourceOrigin {
            manifest: None,
            source_type: Some(blz_core::SourceType::Remote {
                url: "https://example.com".into(),
            }),
        };
        let origin_value = serde_json::to_value(&origin).ok();

        let summary = SourceSummary::new("alpha", "https://example.com", 42)
            .with_headings(5)
            .with_tags(vec!["stable".into()])
            .with_fetched_at("2025-10-01T12:00:00Z")
            .with_checksum("abc")
            .with_etag("tag")
            .with_last_modified("Wed")
            .with_description("Example")
            .with_category("library");

        let summary = if let Some(origin_val) = origin_value {
            summary.with_origin(origin_val)
        } else {
            summary
        };

        let mut buf = Cursor::new(Vec::new());
        render_list(&mut buf, &[summary], OutputFormat::Text, false, false, None)?;
        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("42 lines"));
        assert!(!output.contains("ETag"));
        assert!(!output.contains("Origin:"));
        Ok(())
    }

    #[test]
    fn list_with_limit_returns_exact_count() -> Result<()> {
        let metadata = sample_source("https://example.com");
        let storage = MockStorage {
            aliases: vec!["alpha".into(), "beta".into(), "gamma".into()],
            metadata: HashMap::from([
                (String::from("alpha"), metadata.clone()),
                (String::from("beta"), metadata.clone()),
                (String::from("gamma"), metadata.clone()),
            ]),
            llms: HashMap::from([
                (
                    String::from("alpha"),
                    sample_llms("alpha", metadata.clone(), 100, 10),
                ),
                (
                    String::from("beta"),
                    sample_llms("beta", metadata.clone(), 200, 20),
                ),
                (
                    String::from("gamma"),
                    sample_llms("gamma", metadata, 300, 30),
                ),
            ]),
            descriptors: HashMap::new(),
            fail_on_metadata: false,
        };

        // Test limit of 2 sources
        let mut buf = Cursor::new(Vec::new());
        execute_with_writer(
            &storage,
            &mut buf,
            OutputFormat::Json,
            false,
            false,
            Some(2),
        )?;
        let output = String::from_utf8(buf.into_inner())?;
        let value: serde_json::Value = serde_json::from_str(&output)?;

        assert!(value.is_array());
        assert_eq!(
            value.as_array().expect("should be array").len(),
            2,
            "Should return exactly 2 sources when limit=2"
        );

        Ok(())
    }

    #[test]
    fn list_with_limit_greater_than_sources_returns_all() -> Result<()> {
        let metadata = sample_source("https://example.com");
        let storage = MockStorage {
            aliases: vec!["alpha".into(), "beta".into()],
            metadata: HashMap::from([
                (String::from("alpha"), metadata.clone()),
                (String::from("beta"), metadata.clone()),
            ]),
            llms: HashMap::from([
                (
                    String::from("alpha"),
                    sample_llms("alpha", metadata.clone(), 100, 10),
                ),
                (String::from("beta"), sample_llms("beta", metadata, 200, 20)),
            ]),
            descriptors: HashMap::new(),
            fail_on_metadata: false,
        };

        // Test limit greater than source count
        let mut buf = Cursor::new(Vec::new());
        execute_with_writer(
            &storage,
            &mut buf,
            OutputFormat::Json,
            false,
            false,
            Some(10),
        )?;
        let output = String::from_utf8(buf.into_inner())?;
        let value: serde_json::Value = serde_json::from_str(&output)?;

        assert!(value.is_array());
        assert_eq!(
            value.as_array().expect("should be array").len(),
            2,
            "Should return all sources when limit exceeds source count"
        );

        Ok(())
    }

    #[test]
    fn list_with_limit_zero_returns_zero() -> Result<()> {
        let metadata = sample_source("https://example.com");
        let storage = MockStorage {
            aliases: vec!["alpha".into()],
            metadata: HashMap::from([(String::from("alpha"), metadata.clone())]),
            llms: HashMap::from([(
                String::from("alpha"),
                sample_llms("alpha", metadata, 100, 10),
            )]),
            descriptors: HashMap::new(),
            fail_on_metadata: false,
        };

        // Test limit of 0
        let mut buf = Cursor::new(Vec::new());
        execute_with_writer(
            &storage,
            &mut buf,
            OutputFormat::Json,
            false,
            false,
            Some(0),
        )?;
        let output = String::from_utf8(buf.into_inner())?;
        let value: serde_json::Value = serde_json::from_str(&output)?;

        assert!(value.is_array());
        assert_eq!(
            value.as_array().expect("should be array").len(),
            0,
            "Should return 0 sources when limit=0"
        );

        Ok(())
    }

    #[test]
    fn list_with_limit_applies_to_text_output() -> Result<()> {
        let metadata = sample_source("https://example.com");
        let storage = MockStorage {
            aliases: vec!["alpha".into(), "beta".into(), "gamma".into()],
            metadata: HashMap::from([
                (String::from("alpha"), metadata.clone()),
                (String::from("beta"), metadata.clone()),
                (String::from("gamma"), metadata.clone()),
            ]),
            llms: HashMap::from([
                (
                    String::from("alpha"),
                    sample_llms("alpha", metadata.clone(), 100, 10),
                ),
                (
                    String::from("beta"),
                    sample_llms("beta", metadata.clone(), 200, 20),
                ),
                (
                    String::from("gamma"),
                    sample_llms("gamma", metadata, 300, 30),
                ),
            ]),
            descriptors: HashMap::new(),
            fail_on_metadata: false,
        };

        // Test limit with text output
        let mut buf = Cursor::new(Vec::new());
        execute_with_writer(
            &storage,
            &mut buf,
            OutputFormat::Text,
            false,
            false,
            Some(1),
        )?;
        let output = String::from_utf8(buf.into_inner())?;

        // Count how many source blocks appear (each has a URL line)
        let url_count = output.matches("https://example.com").count();
        assert_eq!(url_count, 1, "Text output should respect limit");

        Ok(())
    }

    #[test]
    fn list_with_limit_applies_to_jsonl_output() -> Result<()> {
        let metadata = sample_source("https://example.com");
        let storage = MockStorage {
            aliases: vec!["alpha".into(), "beta".into(), "gamma".into()],
            metadata: HashMap::from([
                (String::from("alpha"), metadata.clone()),
                (String::from("beta"), metadata.clone()),
                (String::from("gamma"), metadata.clone()),
            ]),
            llms: HashMap::from([
                (
                    String::from("alpha"),
                    sample_llms("alpha", metadata.clone(), 100, 10),
                ),
                (
                    String::from("beta"),
                    sample_llms("beta", metadata.clone(), 200, 20),
                ),
                (
                    String::from("gamma"),
                    sample_llms("gamma", metadata, 300, 30),
                ),
            ]),
            descriptors: HashMap::new(),
            fail_on_metadata: false,
        };

        // Test limit with JSONL output
        let mut buf = Cursor::new(Vec::new());
        execute_with_writer(
            &storage,
            &mut buf,
            OutputFormat::Jsonl,
            false,
            false,
            Some(2),
        )?;
        let output = String::from_utf8(buf.into_inner())?;

        // Count JSON lines
        let line_count = output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count();
        assert_eq!(line_count, 2, "JSONL output should respect limit");

        Ok(())
    }
}
