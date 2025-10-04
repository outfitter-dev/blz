//! List command implementation

use std::io::{self, Write};

use anyhow::{Context, Result};
use blz_core::{LlmsJson, Source, SourceDescriptor, SourceOrigin, SourceType, Storage};
use colored::Colorize;
use serde_json::Value;

use crate::output::OutputFormat;
use crate::utils::count_headings;
use crate::utils::formatting::get_alias_color;

/// Abstraction over storage interactions required by the list command.
pub trait ListStorage {
    fn list_sources(&self) -> Result<Vec<String>>;
    fn load_metadata(&self, alias: &str) -> Result<Option<Source>>;
    fn load_llms_json(&self, alias: &str) -> Result<LlmsJson>;
    fn load_descriptor(&self, alias: &str) -> Result<Option<SourceDescriptor>>;
}

#[allow(clippy::use_self)]
impl ListStorage for Storage {
    fn list_sources(&self) -> Result<Vec<String>> {
        Ok(self.list_sources())
    }

    fn load_metadata(&self, alias: &str) -> Result<Option<Source>> {
        Storage::load_source_metadata(self, alias).map_err(anyhow::Error::from)
    }

    fn load_llms_json(&self, alias: &str) -> Result<LlmsJson> {
        Storage::load_llms_json(self, alias).map_err(anyhow::Error::from)
    }

    fn load_descriptor(&self, alias: &str) -> Result<Option<SourceDescriptor>> {
        Storage::load_descriptor(self, alias).map_err(anyhow::Error::from)
    }
}

/// Summary information for each source returned by `collect_source_summaries`.
#[derive(Debug, Clone)]
pub struct SourceSummary {
    pub alias: String,
    pub url: String,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    pub fetched_at: String,
    pub sha256: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub lines: usize,
    pub headings: usize,
    pub description: Option<String>,
    pub category: Option<String>,
    pub npm_aliases: Vec<String>,
    pub github_aliases: Vec<String>,
    pub origin: SourceOrigin,
    pub descriptor: Option<SourceDescriptor>,
}

/// Gather source summaries from storage.
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

        summaries.push(SourceSummary {
            alias: alias.clone(),
            url: metadata.url,
            tags: metadata.tags,
            aliases: metadata.aliases,
            fetched_at: metadata.fetched_at.to_rfc3339(),
            sha256: metadata.sha256,
            etag: metadata.etag,
            last_modified: metadata.last_modified,
            lines: llms.line_index.total_lines,
            headings: count_headings(&llms.toc),
            description,
            category,
            npm_aliases: metadata.npm_aliases,
            github_aliases: metadata.github_aliases,
            origin: metadata.origin,
            descriptor,
        });
    }

    Ok(summaries)
}

/// Render a list of source summaries to the provided writer.
pub fn render_list<W: Write>(
    writer: &mut W,
    sources: &[SourceSummary],
    format: OutputFormat,
    status: bool,
    details: bool,
) -> Result<()> {
    match format {
        OutputFormat::Text => render_text(writer, sources, status, details),
        OutputFormat::Json => render_json(writer, sources, status),
        OutputFormat::Jsonl => render_jsonl(writer, sources, status),
        OutputFormat::Raw => {
            // Raw format: just print aliases, one per line
            for source in sources {
                writeln!(writer, "{}", source.alias)?;
            }
            Ok(())
        },
    }
}

/// Execute the list command using production storage and stdout.
pub async fn execute(format: OutputFormat, status: bool, details: bool) -> Result<()> {
    let storage = Storage::new()?;
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    execute_with_writer(&storage, &mut handle, format, status, details)
}

/// Testable entry point allowing storage and writer injection.
pub fn execute_with_writer<S, W>(
    storage: &S,
    writer: &mut W,
    format: OutputFormat,
    status: bool,
    details: bool,
) -> Result<()>
where
    S: ListStorage,
    W: Write,
{
    let summaries = collect_source_summaries(storage)?;

    if summaries.is_empty() {
        match format {
            OutputFormat::Text => {
                writeln!(
                    writer,
                    "No sources configured. Use 'blz add' to add sources."
                )?;
                return Ok(());
            },
            OutputFormat::Raw => return Ok(()),
            OutputFormat::Json | OutputFormat::Jsonl => {
                writeln!(writer, "[]")?;
                return Ok(());
            },
        }
    }

    render_list(writer, &summaries, format, status, details)
}

fn render_text<W: Write>(
    writer: &mut W,
    sources: &[SourceSummary],
    status: bool,
    details: bool,
) -> Result<()> {
    for (idx, source) in sources.iter().enumerate() {
        let colored_alias = get_alias_color(&source.alias, idx);
        writeln!(writer, "{} - {}", colored_alias, source.url.bright_black())?;
        writeln!(
            writer,
            "  {} lines, {} headings",
            source.lines, source.headings
        )?;

        if !source.tags.is_empty() {
            writeln!(writer, "  Tags: {}", source.tags.join(", "))?;
        }

        if status {
            writeln!(writer, "  Last updated: {}", source.fetched_at)?;
            if let Some(etag) = &source.etag {
                writeln!(writer, "  ETag: {etag}")?;
            }
            if let Some(last_modified) = &source.last_modified {
                writeln!(writer, "  Last-Modified: {last_modified}")?;
            }
            writeln!(writer, "  SHA256: {}", source.sha256)?;
        }

        if details {
            if let Some(description) = &source.description {
                writeln!(writer, "  Description: {description}")?;
            }
            if let Some(category) = &source.category {
                writeln!(writer, "  Category: {category}")?;
            }
            if !source.npm_aliases.is_empty() {
                writeln!(writer, "  npm: {}", source.npm_aliases.join(", "))?;
            }
            if !source.github_aliases.is_empty() {
                writeln!(writer, "  github: {}", source.github_aliases.join(", "))?;
            }
            if let Some(descriptor) = &source.descriptor {
                if let Some(url) = &descriptor.url {
                    writeln!(writer, "  Descriptor URL: {url}")?;
                }
                if let Some(path) = &descriptor.path {
                    writeln!(writer, "  Local path: {path}")?;
                }
                if let Some(manifest) = &descriptor.origin.manifest {
                    writeln!(
                        writer,
                        "  Manifest: {} ({})",
                        manifest.path, manifest.entry_alias
                    )?;
                }
            }

            writeln!(
                writer,
                "  Origin: {}",
                match &source.origin.source_type {
                    Some(SourceType::Remote { url }) => format!("remote ({url})"),
                    Some(SourceType::LocalFile { path }) => format!("local ({path})"),
                    None => "unknown".to_string(),
                }
            )?;
        }

        writeln!(writer)?;
    }
    Ok(())
}

fn render_json<W: Write>(writer: &mut W, sources: &[SourceSummary], status: bool) -> Result<()> {
    let json_sources: Vec<Value> = sources
        .iter()
        .map(|source| summary_to_json(source, status))
        .collect();
    serde_json::to_writer_pretty(&mut *writer, &json_sources)?;
    writeln!(writer)?;
    Ok(())
}

fn render_jsonl<W: Write>(writer: &mut W, sources: &[SourceSummary], status: bool) -> Result<()> {
    for source in sources {
        serde_json::to_writer(&mut *writer, &summary_to_json(source, status))?;
        writeln!(writer)?;
    }
    Ok(())
}

fn summary_to_json(source: &SourceSummary, status: bool) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert("alias".to_string(), Value::String(source.alias.clone()));
    obj.insert("url".to_string(), Value::String(source.url.clone()));
    obj.insert("lines".to_string(), serde_json::json!(source.lines));
    obj.insert("headings".to_string(), serde_json::json!(source.headings));
    obj.insert("tags".to_string(), serde_json::json!(source.tags.clone()));
    obj.insert(
        "aliases".to_string(),
        serde_json::json!(source.aliases.clone()),
    );
    obj.insert(
        "fetchedAt".to_string(),
        Value::String(source.fetched_at.clone()),
    );
    obj.insert("sha256".to_string(), Value::String(source.sha256.clone()));

    if let Some(description) = &source.description {
        obj.insert(
            "description".to_string(),
            Value::String(description.clone()),
        );
    }
    if let Some(category) = &source.category {
        obj.insert("category".to_string(), Value::String(category.clone()));
    }
    obj.insert(
        "npmAliases".to_string(),
        serde_json::json!(source.npm_aliases.clone()),
    );
    obj.insert(
        "githubAliases".to_string(),
        serde_json::json!(source.github_aliases.clone()),
    );
    obj.insert(
        "origin".to_string(),
        serde_json::to_value(&source.origin).unwrap_or(Value::Null),
    );
    if let Some(descriptor) = &source.descriptor {
        obj.insert(
            "descriptor".to_string(),
            serde_json::to_value(descriptor).unwrap_or(Value::Null),
        );
    }

    if status {
        if let Some(etag) = &source.etag {
            obj.insert("etag".to_string(), Value::String(etag.clone()));
        }
        if let Some(last_modified) = &source.last_modified {
            obj.insert(
                "lastModified".to_string(),
                Value::String(last_modified.clone()),
            );
        }
    }

    Value::Object(obj)
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
        }
    }

    fn heading_entries(count: usize) -> Vec<blz_core::TocEntry> {
        (0..count)
            .map(|i| blz_core::TocEntry {
                heading_path: vec![format!("Heading {i}")],
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
        execute_with_writer(&storage, &mut buf, OutputFormat::Text, false, false)?;
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
        execute_with_writer(&storage, &mut buf, OutputFormat::Json, true, false)?;
        let output = String::from_utf8(buf.into_inner())?;
        let value: serde_json::Value = serde_json::from_str(&output)?;
        assert_eq!(value[0]["alias"], "alpha");
        assert!(value[0].get("etag").is_some());
        Ok(())
    }

    #[test]
    fn render_text_omits_status_details_when_disabled() -> Result<()> {
        let summary = SourceSummary {
            alias: "alpha".into(),
            url: "https://example.com".into(),
            tags: vec!["stable".into()],
            aliases: vec![],
            fetched_at: "2025-10-01T12:00:00Z".into(),
            sha256: "abc".into(),
            etag: Some("tag".into()),
            last_modified: Some("Wed".into()),
            lines: 42,
            headings: 5,
            description: Some("Example".into()),
            category: Some("library".into()),
            npm_aliases: vec![],
            github_aliases: vec![],
            origin: blz_core::SourceOrigin {
                manifest: None,
                source_type: Some(blz_core::SourceType::Remote {
                    url: "https://example.com".into(),
                }),
            },
            descriptor: None,
        };
        let mut buf = Cursor::new(Vec::new());
        render_text(&mut buf, &[summary], false, false)?;
        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("42 lines"));
        assert!(!output.contains("ETag"));
        assert!(!output.contains("Origin:"));
        Ok(())
    }
}
