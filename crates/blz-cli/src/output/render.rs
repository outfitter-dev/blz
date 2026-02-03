// Some render functions await adoption by remaining commands (check, get/retrieve)
#![allow(dead_code)]

//! Unified output rendering for CLI shapes.
//!
//! This module provides a single entry point for rendering any [`OutputShape`]
//! to the specified format. Commands produce structured data; this module
//! handles the presentation logic.
//!
//! # Design
//!
//! The dispatcher pattern separates data production from formatting:
//! - Commands return `OutputShape` variants with structured data
//! - The `render` function dispatches to format-specific renderers
//! - Each shape + format combination has a dedicated render function
//!
//! # Examples
//!
//! ```ignore
//! use blz_cli::output::{OutputFormat, shapes::*};
//! use blz_cli::output::render::render;
//!
//! let output = SourceListOutput::new(vec![
//!     SourceSummary::new("react", "https://react.dev/llms.txt", 5000)
//!         .with_headings(120),
//! ]);
//!
//! let mut stdout = std::io::stdout();
//! render(&output.into(), OutputFormat::Json, &mut stdout)?;
//! ```

use std::io::Write;

use anyhow::Result;
use colored::Colorize;

use super::OutputFormat;
use super::shapes::{OutputShape, SourceListOutput, SourceSummary};
use crate::utils::formatting::get_alias_color;

/// Render an [`OutputShape`] to the given writer in the specified format.
///
/// This is the main entry point for output rendering. It dispatches to
/// format-specific renderers based on the shape and format combination.
///
/// # Errors
///
/// Returns an error if writing to the output fails or if serialization fails.
///
/// # Examples
///
/// ```ignore
/// use blz_cli::output::{OutputFormat, shapes::OutputShape};
/// use blz_cli::output::render::render;
///
/// let shape: OutputShape = /* ... */;
/// let mut stdout = std::io::stdout();
/// render(&shape, OutputFormat::Json, &mut stdout)?;
/// ```
pub fn render(shape: &OutputShape, format: OutputFormat, writer: &mut impl Write) -> Result<()> {
    match (shape, format) {
        (OutputShape::SourceList(data), OutputFormat::Text) => {
            render_source_list_text(data, writer)
        },
        (OutputShape::SourceList(data), OutputFormat::Json) => {
            render_source_list_json(data, writer)
        },
        (OutputShape::SourceList(data), OutputFormat::Jsonl) => {
            render_source_list_jsonl(data, writer)
        },
        (OutputShape::SourceList(data), OutputFormat::Raw) => render_source_list_raw(data, writer),

        // Fallback: serialize as JSON for shape/format combinations without custom renderers
        _ => {
            let json = serde_json::to_string_pretty(shape)?;
            writeln!(writer, "{json}")?;
            Ok(())
        },
    }
}

/// Render options for source list output.
///
/// These options control what additional information is displayed
/// in text output mode.
#[derive(Debug, Clone, Default)]
pub struct SourceListRenderOptions {
    /// Show status information (last updated, `ETag`, checksum).
    pub show_status: bool,
    /// Show detailed information (description, origin, aliases).
    pub show_details: bool,
}

/// Render source list with custom options.
///
/// This function provides more control over rendering compared to
/// the basic `render` function.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn render_source_list_with_options(
    data: &SourceListOutput,
    format: OutputFormat,
    options: &SourceListRenderOptions,
    writer: &mut impl Write,
) -> Result<()> {
    match format {
        OutputFormat::Text => render_source_list_text_with_options(data, options, writer),
        OutputFormat::Json => render_source_list_json_with_options(data, options, writer),
        OutputFormat::Jsonl => render_source_list_jsonl_with_options(data, options, writer),
        OutputFormat::Raw => render_source_list_raw(data, writer),
    }
}

// -----------------------------------------------------------------------------
// Text Renderers
// -----------------------------------------------------------------------------

/// Render source list as human-readable text.
fn render_source_list_text(data: &SourceListOutput, writer: &mut impl Write) -> Result<()> {
    let options = SourceListRenderOptions::default();
    render_source_list_text_with_options(data, &options, writer)
}

/// Render source list as human-readable text with options.
fn render_source_list_text_with_options(
    data: &SourceListOutput,
    options: &SourceListRenderOptions,
    writer: &mut impl Write,
) -> Result<()> {
    if data.sources.is_empty() {
        writeln!(
            writer,
            "No sources configured. Use 'blz add' to add sources."
        )?;
        return Ok(());
    }

    for (idx, source) in data.sources.iter().enumerate() {
        render_source_text(writer, source, idx, options)?;
        writeln!(writer)?;
    }

    Ok(())
}

/// Render a single source summary as text.
fn render_source_text(
    writer: &mut impl Write,
    source: &SourceSummary,
    index: usize,
    options: &SourceListRenderOptions,
) -> Result<()> {
    let colored_alias = get_alias_color(&source.alias, index);
    writeln!(writer, "{} - {}", colored_alias, source.url.bright_black())?;
    writeln!(
        writer,
        "  {} lines, {} headings",
        source.lines, source.headings
    )?;

    if !source.tags.is_empty() {
        writeln!(writer, "  Tags: {}", source.tags.join(", "))?;
    }

    if options.show_status {
        if let Some(fetched_at) = &source.fetched_at {
            writeln!(writer, "  Last updated: {fetched_at}")?;
        }
        if let Some(etag) = &source.etag {
            writeln!(writer, "  ETag: {etag}")?;
        }
        if let Some(last_modified) = &source.last_modified {
            writeln!(writer, "  Last-Modified: {last_modified}")?;
        }
        if let Some(checksum) = &source.checksum {
            writeln!(writer, "  SHA256: {checksum}")?;
        }
    }

    if options.show_details {
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
        render_origin_text(writer, source)?;
    }

    Ok(())
}

/// Render origin information as text.
fn render_origin_text(writer: &mut impl Write, source: &SourceSummary) -> Result<()> {
    // Handle descriptor if present
    if let Some(descriptor) = &source.descriptor {
        if let Some(url) = descriptor.get("url").and_then(|v| v.as_str()) {
            writeln!(writer, "  Descriptor URL: {url}")?;
        }
        if let Some(path) = descriptor.get("path").and_then(|v| v.as_str()) {
            writeln!(writer, "  Local path: {path}")?;
        }
        if let Some(origin) = descriptor.get("origin") {
            if let Some(manifest) = origin.get("manifest") {
                if let (Some(path), Some(entry_alias)) = (
                    manifest.get("path").and_then(|v| v.as_str()),
                    manifest.get("entryAlias").and_then(|v| v.as_str()),
                ) {
                    writeln!(writer, "  Manifest: {path} ({entry_alias})")?;
                }
            }
        }
    }

    // Handle origin if present
    if let Some(origin) = &source.origin {
        if let Some(source_type) = origin.get("sourceType") {
            let origin_str = match source_type.get("kind").and_then(|v| v.as_str()) {
                Some("remote") => source_type
                    .get("url")
                    .and_then(|v| v.as_str())
                    .map_or_else(|| "remote".to_string(), |url| format!("remote ({url})")),
                Some("localFile") => source_type
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map_or_else(|| "local".to_string(), |path| format!("local ({path})")),
                _ => "unknown".to_string(),
            };
            writeln!(writer, "  Origin: {origin_str}")?;
        }
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// JSON Renderers
// -----------------------------------------------------------------------------

/// Render source list as JSON.
fn render_source_list_json(data: &SourceListOutput, writer: &mut impl Write) -> Result<()> {
    let options = SourceListRenderOptions::default();
    render_source_list_json_with_options(data, &options, writer)
}

/// Render source list as JSON with options.
fn render_source_list_json_with_options(
    data: &SourceListOutput,
    options: &SourceListRenderOptions,
    writer: &mut impl Write,
) -> Result<()> {
    if data.sources.is_empty() {
        writeln!(writer, "[]")?;
        return Ok(());
    }

    let json_sources: Vec<serde_json::Value> = data
        .sources
        .iter()
        .map(|source| source_to_json(source, options))
        .collect();

    serde_json::to_writer_pretty(&mut *writer, &json_sources)?;
    writeln!(writer)?;
    Ok(())
}

/// Convert a source summary to a JSON value.
fn source_to_json(source: &SourceSummary, options: &SourceListRenderOptions) -> serde_json::Value {
    let mut obj = serde_json::Map::new();

    // Always include core fields
    obj.insert(
        "alias".to_string(),
        serde_json::Value::String(source.alias.clone()),
    );
    obj.insert(
        "url".to_string(),
        serde_json::Value::String(source.url.clone()),
    );
    obj.insert("lines".to_string(), serde_json::json!(source.lines));
    obj.insert("headings".to_string(), serde_json::json!(source.headings));
    obj.insert("tags".to_string(), serde_json::json!(source.tags.clone()));
    obj.insert(
        "aliases".to_string(),
        serde_json::json!(source.aliases.clone()),
    );
    obj.insert("status".to_string(), serde_json::json!(source.status));

    // Include fetched_at and checksum (always present in original list.rs output)
    if let Some(fetched_at) = &source.fetched_at {
        obj.insert(
            "fetchedAt".to_string(),
            serde_json::Value::String(fetched_at.clone()),
        );
    }
    if let Some(checksum) = &source.checksum {
        obj.insert(
            "sha256".to_string(),
            serde_json::Value::String(checksum.clone()),
        );
    }

    // Optional metadata fields
    if let Some(description) = &source.description {
        obj.insert(
            "description".to_string(),
            serde_json::Value::String(description.clone()),
        );
    }
    if let Some(category) = &source.category {
        obj.insert(
            "category".to_string(),
            serde_json::Value::String(category.clone()),
        );
    }

    // Package aliases
    obj.insert(
        "npmAliases".to_string(),
        serde_json::json!(source.npm_aliases.clone()),
    );
    obj.insert(
        "githubAliases".to_string(),
        serde_json::json!(source.github_aliases.clone()),
    );

    // Origin and descriptor
    if let Some(origin) = &source.origin {
        obj.insert("origin".to_string(), origin.clone());
    }
    if let Some(descriptor) = &source.descriptor {
        obj.insert("descriptor".to_string(), descriptor.clone());
    }

    // Status-related fields (conditionally included)
    if options.show_status {
        if let Some(etag) = &source.etag {
            obj.insert("etag".to_string(), serde_json::Value::String(etag.clone()));
        }
        if let Some(last_modified) = &source.last_modified {
            obj.insert(
                "lastModified".to_string(),
                serde_json::Value::String(last_modified.clone()),
            );
        }
    }

    serde_json::Value::Object(obj)
}

// -----------------------------------------------------------------------------
// JSONL Renderer
// -----------------------------------------------------------------------------

/// Render source list as newline-delimited JSON.
fn render_source_list_jsonl(data: &SourceListOutput, writer: &mut impl Write) -> Result<()> {
    let options = SourceListRenderOptions::default();
    render_source_list_jsonl_with_options(data, &options, writer)
}

/// Render source list as newline-delimited JSON with options.
fn render_source_list_jsonl_with_options(
    data: &SourceListOutput,
    options: &SourceListRenderOptions,
    writer: &mut impl Write,
) -> Result<()> {
    for source in &data.sources {
        serde_json::to_writer(&mut *writer, &source_to_json(source, options))?;
        writeln!(writer)?;
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Raw Renderer
// -----------------------------------------------------------------------------

/// Render source list as raw output (just aliases, one per line).
fn render_source_list_raw(data: &SourceListOutput, writer: &mut impl Write) -> Result<()> {
    for source in &data.sources {
        writeln!(writer, "{}", source.alias)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::shapes::SourceStatus;
    use std::io::Cursor;

    fn sample_source() -> SourceSummary {
        SourceSummary::new("react", "https://react.dev/llms.txt", 5000)
            .with_headings(120)
            .with_tags(vec!["javascript".to_string(), "frontend".to_string()])
            .with_fetched_at("2025-01-15T12:00:00Z")
            .with_checksum("abc123def456")
    }

    #[test]
    fn test_render_source_list_text_empty() -> Result<()> {
        let data = SourceListOutput::new(vec![]);
        let mut buf = Cursor::new(Vec::new());
        render_source_list_text(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("No sources configured"));
        Ok(())
    }

    #[test]
    fn test_render_source_list_text_with_source() -> Result<()> {
        let data = SourceListOutput::new(vec![sample_source()]);
        let mut buf = Cursor::new(Vec::new());
        render_source_list_text(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("react"));
        assert!(output.contains("5000 lines"));
        assert!(output.contains("120 headings"));
        assert!(output.contains("Tags: javascript, frontend"));
        Ok(())
    }

    #[test]
    fn test_render_source_list_text_with_status() -> Result<()> {
        let source = sample_source()
            .with_etag("etag-value")
            .with_last_modified("Wed, 15 Jan 2025 12:00:00 GMT");

        let data = SourceListOutput::new(vec![source]);
        let options = SourceListRenderOptions {
            show_status: true,
            show_details: false,
        };
        let mut buf = Cursor::new(Vec::new());
        render_source_list_text_with_options(&data, &options, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("Last updated: 2025-01-15T12:00:00Z"));
        assert!(output.contains("ETag: etag-value"));
        assert!(output.contains("SHA256: abc123def456"));
        Ok(())
    }

    #[test]
    fn test_render_source_list_json_empty() -> Result<()> {
        let data = SourceListOutput::new(vec![]);
        let mut buf = Cursor::new(Vec::new());
        render_source_list_json(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert_eq!(output.trim(), "[]");
        Ok(())
    }

    #[test]
    fn test_render_source_list_json_with_source() -> Result<()> {
        let data = SourceListOutput::new(vec![sample_source()]);
        let mut buf = Cursor::new(Vec::new());
        render_source_list_json(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        let parsed: serde_json::Value = serde_json::from_str(&output)?;

        assert!(parsed.is_array());
        let arr = parsed.as_array().expect("should be array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["alias"], "react");
        assert_eq!(arr[0]["lines"], 5000);
        assert_eq!(arr[0]["headings"], 120);
        assert_eq!(arr[0]["fetchedAt"], "2025-01-15T12:00:00Z");
        Ok(())
    }

    #[test]
    fn test_render_source_list_jsonl() -> Result<()> {
        let data = SourceListOutput::new(vec![
            SourceSummary::new("react", "https://react.dev/llms.txt", 5000),
            SourceSummary::new("bun", "https://bun.sh/llms.txt", 3000),
        ]);
        let mut buf = Cursor::new(Vec::new());
        render_source_list_jsonl(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);

        let first: serde_json::Value = serde_json::from_str(lines[0])?;
        let second: serde_json::Value = serde_json::from_str(lines[1])?;
        assert_eq!(first["alias"], "react");
        assert_eq!(second["alias"], "bun");
        Ok(())
    }

    #[test]
    fn test_render_source_list_raw() -> Result<()> {
        let data = SourceListOutput::new(vec![
            SourceSummary::new("react", "https://react.dev/llms.txt", 5000),
            SourceSummary::new("bun", "https://bun.sh/llms.txt", 3000),
        ]);
        let mut buf = Cursor::new(Vec::new());
        render_source_list_raw(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert_eq!(output, "react\nbun\n");
        Ok(())
    }

    #[test]
    fn test_render_unified_dispatcher() -> Result<()> {
        let data = SourceListOutput::new(vec![sample_source()]);
        let shape: OutputShape = data.into();

        // Test JSON format through dispatcher
        let mut buf = Cursor::new(Vec::new());
        render(&shape, OutputFormat::Json, &mut buf)?;
        let output = String::from_utf8(buf.into_inner())?;
        let parsed: serde_json::Value = serde_json::from_str(&output)?;
        assert_eq!(parsed[0]["alias"], "react");

        // Test text format through dispatcher
        let data = SourceListOutput::new(vec![sample_source()]);
        let shape: OutputShape = data.into();
        let mut buf = Cursor::new(Vec::new());
        render(&shape, OutputFormat::Text, &mut buf)?;
        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("react"));

        Ok(())
    }

    #[test]
    fn test_source_summary_builder() {
        let source = SourceSummary::new("test", "https://test.com", 100)
            .with_status(SourceStatus::Fresh)
            .with_headings(10)
            .with_tags(vec!["tag1".to_string()])
            .with_description("A test source")
            .with_category("testing");

        assert_eq!(source.alias, "test");
        assert_eq!(source.url, "https://test.com");
        assert_eq!(source.lines, 100);
        assert_eq!(source.status, SourceStatus::Fresh);
        assert_eq!(source.headings, 10);
        assert_eq!(source.tags, vec!["tag1"]);
        assert_eq!(source.description, Some("A test source".to_string()));
        assert_eq!(source.category, Some("testing".to_string()));
    }
}
