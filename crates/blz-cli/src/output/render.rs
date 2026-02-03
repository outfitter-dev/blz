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

use blz_core::numeric::{format_bytes, safe_percentage};

use super::OutputFormat;
use super::shapes::{
    OutputShape, SourceInfoOutput, SourceListOutput, SourceSummary, TocEntry, TocMultiOutput,
    TocOutput, TocPaginatedEntry, TocPaginatedOutput, TocRenderOptions,
};
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

        (OutputShape::SourceInfo(data), OutputFormat::Text) => {
            render_source_info_text(data, writer)
        },
        (OutputShape::SourceInfo(data), OutputFormat::Json) => {
            render_source_info_json(data, writer)
        },
        (OutputShape::SourceInfo(data), OutputFormat::Jsonl) => {
            render_source_info_jsonl(data, writer)
        },
        (OutputShape::SourceInfo(data), OutputFormat::Raw) => render_source_info_raw(data, writer),

        // TOC tree output (single source)
        (OutputShape::Toc(data), OutputFormat::Text) => {
            render_toc_text(data, &TocRenderOptions::default(), writer)
        },
        (OutputShape::Toc(data), OutputFormat::Json) => render_toc_json(data, writer),
        (OutputShape::Toc(data), OutputFormat::Jsonl) => render_toc_jsonl(data, writer),

        // TOC paginated output (flat list)
        (OutputShape::TocPaginated(data), OutputFormat::Text) => {
            render_toc_paginated_text(data, &TocRenderOptions::default(), writer)
        },
        (OutputShape::TocPaginated(data), OutputFormat::Json) => {
            render_toc_paginated_json(data, writer)
        },
        (OutputShape::TocPaginated(data), OutputFormat::Jsonl) => {
            render_toc_paginated_jsonl(data, writer)
        },

        // TOC multi-source output
        (OutputShape::TocMulti(data), OutputFormat::Text) => {
            render_toc_multi_text(data, &TocRenderOptions::default(), writer)
        },
        (OutputShape::TocMulti(data), OutputFormat::Json) => render_toc_multi_json(data, writer),
        (OutputShape::TocMulti(data), OutputFormat::Jsonl) => render_toc_multi_jsonl(data, writer),

        // TOC raw output is not supported
        (
            OutputShape::Toc(_) | OutputShape::TocPaginated(_) | OutputShape::TocMulti(_),
            OutputFormat::Raw,
        ) => render_toc_raw_error(writer),

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

// -----------------------------------------------------------------------------
// Source Info Renderers
// -----------------------------------------------------------------------------

/// Render source info as human-readable text.
fn render_source_info_text(data: &SourceInfoOutput, writer: &mut impl Write) -> Result<()> {
    writeln!(writer, "Source: {}", data.alias)?;
    writeln!(writer, "URL: {}", data.url)?;
    writeln!(writer, "Variant: {}", data.variant)?;

    if !data.aliases.is_empty() {
        writeln!(writer, "Aliases: {}", data.aliases.join(", "))?;
    }

    writeln!(writer, "Lines: {}", format_number(data.lines))?;
    writeln!(writer, "Headings: {}", format_number(data.headings))?;
    writeln!(writer, "Size: {}", format_bytes(data.size_bytes))?;

    if let Some(updated) = &data.last_updated {
        writeln!(writer, "Last Updated: {updated}")?;
    }

    if let Some(etag) = &data.etag {
        writeln!(writer, "ETag: {etag}")?;
    }

    if let Some(checksum) = &data.checksum {
        writeln!(writer, "Checksum: {checksum}")?;
    }

    writeln!(writer, "Cache Location: {}", data.cache_path)?;

    // Display language filtering information
    writeln!(writer)?;
    if let Some(stats) = &data.filter_stats {
        writeln!(writer, "Language Filtering:")?;
        let status_text = if stats.enabled {
            "enabled".green()
        } else {
            "disabled".yellow()
        };
        writeln!(writer, "  Status: {status_text}")?;

        if stats.enabled && stats.headings_rejected > 0 {
            let percentage = safe_percentage(stats.headings_rejected, stats.headings_total);
            writeln!(
                writer,
                "  Filtered: {} headings ({percentage:.1}%)",
                format_number(stats.headings_rejected)
            )?;
            writeln!(writer, "  Reason: {}", stats.reason)?;
        }
    } else {
        writeln!(
            writer,
            "Language Filtering: {} (added before filtering feature)",
            "unknown".yellow()
        )?;
    }

    Ok(())
}

/// Render source info as JSON.
fn render_source_info_json(data: &SourceInfoOutput, writer: &mut impl Write) -> Result<()> {
    serde_json::to_writer_pretty(&mut *writer, data)?;
    writeln!(writer)?;
    Ok(())
}

/// Render source info as JSONL.
fn render_source_info_jsonl(data: &SourceInfoOutput, writer: &mut impl Write) -> Result<()> {
    serde_json::to_writer(&mut *writer, data)?;
    writeln!(writer)?;
    Ok(())
}

/// Render source info as raw output (just the URL).
fn render_source_info_raw(data: &SourceInfoOutput, writer: &mut impl Write) -> Result<()> {
    writeln!(writer, "{}", data.url)?;
    Ok(())
}

// -----------------------------------------------------------------------------
// TOC Renderers (Single Source Tree)
// -----------------------------------------------------------------------------

/// Render TOC with custom options.
///
/// This function provides more control over rendering compared to
/// the basic `render` function.
///
/// # Errors
///
/// Returns an error if writing to the output fails.
pub fn render_toc_with_options(
    data: &TocOutput,
    format: OutputFormat,
    options: &TocRenderOptions,
    writer: &mut impl Write,
) -> Result<()> {
    match format {
        OutputFormat::Text => render_toc_text(data, options, writer),
        OutputFormat::Json => render_toc_json(data, writer),
        OutputFormat::Jsonl => render_toc_jsonl(data, writer),
        OutputFormat::Raw => render_toc_raw_error(writer),
    }
}

/// Render TOC as human-readable text with tree structure.
fn render_toc_text(
    data: &TocOutput,
    options: &TocRenderOptions,
    writer: &mut impl Write,
) -> Result<()> {
    writeln!(writer, "Table of contents for {}\n", data.alias.green())?;

    if options.tree_mode {
        // Tree view with box-drawing characters
        let mut state = TreeState::default();
        for (i, entry) in data.entries.iter().enumerate() {
            let is_last = i == data.entries.len() - 1;
            render_tree_entry(writer, entry, 0, is_last, "", options, &mut state)?;
        }
    } else {
        // Hierarchical indented list
        for entry in &data.entries {
            render_hierarchical_entry(writer, entry, 0, options)?;
        }
    }

    Ok(())
}

/// Render TOC as JSON.
fn render_toc_json(data: &TocOutput, writer: &mut impl Write) -> Result<()> {
    serde_json::to_writer_pretty(&mut *writer, data)?;
    writeln!(writer)?;
    Ok(())
}

/// Render TOC as JSONL.
fn render_toc_jsonl(data: &TocOutput, writer: &mut impl Write) -> Result<()> {
    // For tree structure, we emit each top-level entry as a line
    for entry in &data.entries {
        serde_json::to_writer(&mut *writer, entry)?;
        writeln!(writer)?;
    }
    Ok(())
}

/// Return error for unsupported raw TOC output.
fn render_toc_raw_error(writer: &mut impl Write) -> Result<()> {
    writeln!(
        writer,
        "Raw output is not supported for toc listings. Use --format json, jsonl, or text instead."
    )?;
    anyhow::bail!("Raw output is not supported for toc listings");
}

// -----------------------------------------------------------------------------
// TOC Renderers (Paginated Flat List)
// -----------------------------------------------------------------------------

/// Render paginated TOC with custom options.
pub fn render_toc_paginated_with_options(
    data: &TocPaginatedOutput,
    format: OutputFormat,
    options: &TocRenderOptions,
    writer: &mut impl Write,
) -> Result<()> {
    match format {
        OutputFormat::Text => render_toc_paginated_text(data, options, writer),
        OutputFormat::Json => render_toc_paginated_json(data, writer),
        OutputFormat::Jsonl => render_toc_paginated_jsonl(data, writer),
        OutputFormat::Raw => render_toc_raw_error(writer),
    }
}

/// Render paginated TOC as text.
fn render_toc_paginated_text(
    data: &TocPaginatedOutput,
    options: &TocRenderOptions,
    writer: &mut impl Write,
) -> Result<()> {
    // Group entries by source for display
    let sources: std::collections::HashSet<&str> =
        data.entries.iter().map(|e| e.source.as_str()).collect();

    if sources.len() > 1 {
        writeln!(
            writer,
            "Table of contents (showing {} sources)",
            sources.len()
        )?;
    } else if let Some(source) = sources.iter().next() {
        writeln!(writer, "Table of contents for {}\n", source.green())?;
    }

    for entry in &data.entries {
        render_paginated_entry_text(writer, entry, options)?;
    }

    writeln!(
        writer,
        "\nPage {} of {} ({} total results)",
        data.page,
        data.total_pages.max(1),
        data.total_results
    )?;

    Ok(())
}

/// Render paginated TOC as JSON.
fn render_toc_paginated_json(data: &TocPaginatedOutput, writer: &mut impl Write) -> Result<()> {
    serde_json::to_writer_pretty(&mut *writer, data)?;
    writeln!(writer)?;
    Ok(())
}

/// Render paginated TOC as JSONL.
fn render_toc_paginated_jsonl(data: &TocPaginatedOutput, writer: &mut impl Write) -> Result<()> {
    // First line: metadata
    let metadata = serde_json::json!({
        "page": data.page,
        "total_pages": data.total_pages.max(1),
        "total_results": data.total_results,
        "page_size": data.page_size,
    });
    serde_json::to_writer(&mut *writer, &metadata)?;
    writeln!(writer)?;

    // Subsequent lines: entries
    for entry in &data.entries {
        serde_json::to_writer(&mut *writer, entry)?;
        writeln!(writer)?;
    }
    Ok(())
}

/// Render a single paginated entry as text.
fn render_paginated_entry_text(
    writer: &mut impl Write,
    entry: &TocPaginatedEntry,
    options: &TocRenderOptions,
) -> Result<()> {
    let name = entry.heading_path.last().map_or("", String::as_str);
    let indent = "  ".repeat(entry.heading_level.saturating_sub(1) as usize);
    let lines_display = format!("[{}]", entry.lines).dimmed();

    if options.show_anchors {
        let anchor = entry.anchor.as_deref().unwrap_or("");
        writeln!(
            writer,
            "{indent}- {name} {lines_display} {}",
            anchor.bright_black()
        )?;
    } else {
        writeln!(writer, "{indent}- {name} {lines_display}")?;
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// TOC Renderers (Multi-Source)
// -----------------------------------------------------------------------------

/// Render multi-source TOC with custom options.
pub fn render_toc_multi_with_options(
    data: &TocMultiOutput,
    format: OutputFormat,
    options: &TocRenderOptions,
    writer: &mut impl Write,
) -> Result<()> {
    match format {
        OutputFormat::Text => render_toc_multi_text(data, options, writer),
        OutputFormat::Json => render_toc_multi_json(data, writer),
        OutputFormat::Jsonl => render_toc_multi_jsonl(data, writer),
        OutputFormat::Raw => render_toc_raw_error(writer),
    }
}

/// Render multi-source TOC as text.
fn render_toc_multi_text(
    data: &TocMultiOutput,
    options: &TocRenderOptions,
    writer: &mut impl Write,
) -> Result<()> {
    for (idx, source) in data.sources.iter().enumerate() {
        if idx > 0 {
            writeln!(writer)?;
        }

        if data.sources.len() > 1 {
            writeln!(writer, "\n{}:", source.alias.green())?;
        } else {
            writeln!(writer, "Table of contents for {}\n", source.alias.green())?;
        }

        if options.tree_mode {
            let mut state = TreeState::default();
            for (i, entry) in source.entries.iter().enumerate() {
                let is_last = i == source.entries.len() - 1;
                render_tree_entry(writer, entry, 0, is_last, "", options, &mut state)?;
            }
        } else {
            for entry in &source.entries {
                render_hierarchical_entry(writer, entry, 0, options)?;
            }
        }
    }

    Ok(())
}

/// Render multi-source TOC as JSON.
fn render_toc_multi_json(data: &TocMultiOutput, writer: &mut impl Write) -> Result<()> {
    serde_json::to_writer_pretty(&mut *writer, data)?;
    writeln!(writer)?;
    Ok(())
}

/// Render multi-source TOC as JSONL.
fn render_toc_multi_jsonl(data: &TocMultiOutput, writer: &mut impl Write) -> Result<()> {
    for source in &data.sources {
        serde_json::to_writer(&mut *writer, source)?;
        writeln!(writer)?;
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Tree Rendering Helpers
// -----------------------------------------------------------------------------

/// State for tree rendering, tracking previous depth and H1 children.
#[derive(Default)]
struct TreeState {
    count: usize,
    prev_depth: Option<usize>,
    prev_h1_had_children: bool,
}

/// Render a tree entry with box-drawing characters.
fn render_tree_entry(
    writer: &mut impl Write,
    entry: &TocEntry,
    depth: usize,
    is_last: bool,
    prefix: &str,
    options: &TocRenderOptions,
    state: &mut TreeState,
) -> Result<bool> {
    let name = &entry.title;
    let lines_display = format!("[{}]", entry.lines).dimmed();

    // Add blank line when jumping up levels (but not to H1 - H1 handles its own spacing)
    if let Some(prev) = state.prev_depth {
        if depth < prev && depth > 0 {
            // Jumping up levels within H2+
            if depth > 1 {
                // H3+ has continuation pipes
                let pipe_prefix = prefix.trim_end();
                writeln!(writer, "{pipe_prefix}")?;
            } else if depth == 1 {
                // H2 level: show pipe if not last sibling
                if is_last {
                    writeln!(writer)?;
                } else {
                    writeln!(writer, "\u{2502}")?; // │
                }
            }
        }
    }

    // H1s (depth 0) are left-aligned with no branch characters
    if depth == 0 {
        // Add blank line before H1 if previous H1 had visible children
        if state.prev_h1_had_children {
            writeln!(writer)?;
        }
        if options.show_anchors {
            let anchor = entry.anchor.as_deref().unwrap_or("");
            writeln!(writer, "{name} {lines_display} {}", anchor.bright_black())?;
        } else {
            writeln!(writer, "{name} {lines_display}")?;
        }
    } else {
        // H2+ use tree structure
        let branch = if is_last {
            "\u{2514}\u{2500} "
        } else {
            "\u{251c}\u{2500} "
        }; // └─ or ├─
        if options.show_anchors {
            let anchor = entry.anchor.as_deref().unwrap_or("");
            writeln!(
                writer,
                "{prefix}{branch}{name} {lines_display} {}",
                anchor.bright_black()
            )?;
        } else {
            writeln!(writer, "{prefix}{branch}{name} {lines_display}")?;
        }
    }

    state.count += 1;
    state.prev_depth = Some(depth);

    let mut had_visible_children = false;

    // Render children
    let new_prefix = if depth == 0 {
        // For H1s, children don't get additional prefix since H1 is left-aligned
        String::new()
    } else {
        format!(
            "{}{}  ",
            prefix,
            if is_last { " " } else { "\u{2502}" } // │
        )
    };

    for (i, child) in entry.children.iter().enumerate() {
        let child_is_last = i == entry.children.len() - 1;
        let child_printed = render_tree_entry(
            writer,
            child,
            depth + 1,
            child_is_last,
            &new_prefix,
            options,
            state,
        )?;
        if child_printed {
            had_visible_children = true;
        }
    }

    // If this is an H1, update the flag for next H1
    if depth == 0 {
        state.prev_h1_had_children = had_visible_children;
    }

    Ok(true)
}

/// Render a hierarchical entry with indentation (non-tree mode).
fn render_hierarchical_entry(
    writer: &mut impl Write,
    entry: &TocEntry,
    depth: usize,
    options: &TocRenderOptions,
) -> Result<()> {
    let name = &entry.title;
    let indent = "  ".repeat(depth);
    let lines_display = format!("[{}]", entry.lines).dimmed();

    if options.show_anchors {
        let anchor = entry.anchor.as_deref().unwrap_or("");
        writeln!(
            writer,
            "{indent}- {name} {lines_display} {}",
            anchor.bright_black()
        )?;
    } else {
        writeln!(writer, "{indent}- {name} {lines_display}")?;
    }

    for child in &entry.children {
        render_hierarchical_entry(writer, child, depth + 1, options)?;
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// Formatting Helpers
// -----------------------------------------------------------------------------

/// Format a number with thousand separators.
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::shapes::{
        FilterStatsOutput, SourceStatus, TocEntry, TocMultiOutput, TocOutput, TocPaginatedEntry,
        TocPaginatedOutput,
    };
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

    // -------------------------------------------------------------------------
    // Formatting helper tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(12345), "12,345");
        assert_eq!(format_number(123_456), "123,456");
        assert_eq!(format_number(1_234_567), "1,234,567");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
        assert_eq!(format_bytes(1_258_291), "1.2 MB");
    }

    #[test]
    fn test_safe_percentage() {
        assert!((safe_percentage(0, 100) - 0.0).abs() < f64::EPSILON);
        assert!((safe_percentage(50, 100) - 50.0).abs() < f64::EPSILON);
        assert!((safe_percentage(100, 100) - 100.0).abs() < f64::EPSILON);
        assert!((safe_percentage(33, 100) - 33.0).abs() < f64::EPSILON);
        assert!((safe_percentage(0, 0) - 0.0).abs() < f64::EPSILON); // Edge case: no total
    }

    #[test]
    fn test_safe_percentage_precision() {
        let result = safe_percentage(1, 3);
        assert!((result - 33.333_333).abs() < 0.001);
    }

    // -------------------------------------------------------------------------
    // Source info render tests
    // -------------------------------------------------------------------------

    fn sample_source_info() -> SourceInfoOutput {
        SourceInfoOutput::new(
            "react",
            "https://react.dev/llms.txt",
            "LlmsFull",
            5000,
            120,
            512_000,
            "/home/user/.blz/sources/react",
        )
        .with_aliases(vec!["reactjs".to_string()])
        .with_last_updated("2025-01-15T12:00:00Z")
        .with_etag("abc123")
        .with_checksum("sha256hash")
    }

    #[test]
    fn test_render_source_info_text() -> Result<()> {
        let data = sample_source_info();
        let mut buf = Cursor::new(Vec::new());
        render_source_info_text(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("Source: react"));
        assert!(output.contains("URL: https://react.dev/llms.txt"));
        assert!(output.contains("Variant: LlmsFull"));
        assert!(output.contains("Aliases: reactjs"));
        assert!(output.contains("Lines: 5,000"));
        assert!(output.contains("Headings: 120"));
        assert!(output.contains("Size: 500.0 KB"));
        assert!(output.contains("Last Updated: 2025-01-15T12:00:00Z"));
        assert!(output.contains("ETag: abc123"));
        assert!(output.contains("Checksum: sha256hash"));
        assert!(output.contains("Cache Location: /home/user/.blz/sources/react"));
        Ok(())
    }

    #[test]
    fn test_render_source_info_text_with_filter_stats() -> Result<()> {
        let data = sample_source_info().with_filter_stats(FilterStatsOutput {
            enabled: true,
            headings_total: 100,
            headings_accepted: 80,
            headings_rejected: 20,
            reason: "non-English content removed".to_string(),
        });
        let mut buf = Cursor::new(Vec::new());
        render_source_info_text(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("Language Filtering:"));
        assert!(output.contains("Filtered: 20 headings (20.0%)"));
        assert!(output.contains("Reason: non-English content removed"));
        Ok(())
    }

    #[test]
    fn test_render_source_info_json() -> Result<()> {
        let data = sample_source_info();
        let mut buf = Cursor::new(Vec::new());
        render_source_info_json(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        let parsed: serde_json::Value = serde_json::from_str(&output)?;

        assert_eq!(parsed["alias"], "react");
        assert_eq!(parsed["url"], "https://react.dev/llms.txt");
        assert_eq!(parsed["variant"], "LlmsFull");
        assert_eq!(parsed["lines"], 5000);
        assert_eq!(parsed["headings"], 120);
        assert_eq!(parsed["sizeBytes"], 512_000);
        assert_eq!(parsed["lastUpdated"], "2025-01-15T12:00:00Z");
        assert_eq!(parsed["etag"], "abc123");
        assert_eq!(parsed["checksum"], "sha256hash");
        Ok(())
    }

    #[test]
    fn test_render_source_info_jsonl() -> Result<()> {
        let data = sample_source_info();
        let mut buf = Cursor::new(Vec::new());
        render_source_info_jsonl(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        // JSONL should be a single line
        assert_eq!(output.lines().count(), 1);

        let parsed: serde_json::Value = serde_json::from_str(&output)?;
        assert_eq!(parsed["alias"], "react");
        Ok(())
    }

    #[test]
    fn test_render_source_info_raw() -> Result<()> {
        let data = sample_source_info();
        let mut buf = Cursor::new(Vec::new());
        render_source_info_raw(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert_eq!(output.trim(), "https://react.dev/llms.txt");
        Ok(())
    }

    #[test]
    fn test_render_source_info_dispatcher() -> Result<()> {
        let data = sample_source_info();
        let shape: OutputShape = data.into();

        // Test JSON format through dispatcher
        let mut buf = Cursor::new(Vec::new());
        render(&shape, OutputFormat::Json, &mut buf)?;
        let output = String::from_utf8(buf.into_inner())?;
        let parsed: serde_json::Value = serde_json::from_str(&output)?;
        assert_eq!(parsed["alias"], "react");

        // Test text format through dispatcher
        let data = sample_source_info();
        let shape: OutputShape = data.into();
        let mut buf = Cursor::new(Vec::new());
        render(&shape, OutputFormat::Text, &mut buf)?;
        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("Source: react"));

        Ok(())
    }

    // -------------------------------------------------------------------------
    // TOC render tests
    // -------------------------------------------------------------------------

    fn sample_toc_entry() -> TocEntry {
        TocEntry {
            level: 1,
            title: "Getting Started".to_string(),
            lines: "1-50".to_string(),
            anchor: Some("getting-started".to_string()),
            heading_path: vec!["Getting Started".to_string()],
            children: vec![
                TocEntry {
                    level: 2,
                    title: "Installation".to_string(),
                    lines: "10-30".to_string(),
                    anchor: Some("installation".to_string()),
                    heading_path: vec!["Getting Started".to_string(), "Installation".to_string()],
                    children: vec![],
                },
                TocEntry {
                    level: 2,
                    title: "Quick Start".to_string(),
                    lines: "31-50".to_string(),
                    anchor: None,
                    heading_path: vec!["Getting Started".to_string(), "Quick Start".to_string()],
                    children: vec![],
                },
            ],
        }
    }

    #[test]
    fn test_render_toc_text_hierarchical() -> Result<()> {
        let data = TocOutput::new("react", vec![sample_toc_entry()]);
        let options = TocRenderOptions::default();
        let mut buf = Cursor::new(Vec::new());
        render_toc_text(&data, &options, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("Table of contents for"));
        assert!(output.contains("Getting Started"));
        assert!(output.contains("Installation"));
        assert!(output.contains("Quick Start"));
        assert!(output.contains("[1-50]"));
        Ok(())
    }

    #[test]
    fn test_render_toc_text_tree_mode() -> Result<()> {
        let data = TocOutput::new("react", vec![sample_toc_entry()]);
        let options = TocRenderOptions {
            tree_mode: true,
            show_anchors: false,
        };
        let mut buf = Cursor::new(Vec::new());
        render_toc_text(&data, &options, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("Getting Started"));
        // Tree should contain box-drawing characters
        assert!(output.contains("\u{251c}") || output.contains("\u{2514}")); // ├ or └
        Ok(())
    }

    #[test]
    fn test_render_toc_text_with_anchors() -> Result<()> {
        let data = TocOutput::new("react", vec![sample_toc_entry()]);
        let options = TocRenderOptions {
            tree_mode: false,
            show_anchors: true,
        };
        let mut buf = Cursor::new(Vec::new());
        render_toc_text(&data, &options, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("getting-started"));
        assert!(output.contains("installation"));
        Ok(())
    }

    #[test]
    fn test_render_toc_json() -> Result<()> {
        let data = TocOutput::new("react", vec![sample_toc_entry()]);
        let mut buf = Cursor::new(Vec::new());
        render_toc_json(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        let parsed: serde_json::Value = serde_json::from_str(&output)?;

        assert_eq!(parsed["alias"], "react");
        assert_eq!(parsed["totalEntries"], 3); // 1 parent + 2 children
        assert!(parsed["entries"].is_array());
        Ok(())
    }

    #[test]
    fn test_render_toc_jsonl() -> Result<()> {
        let data = TocOutput::new("react", vec![sample_toc_entry()]);
        let mut buf = Cursor::new(Vec::new());
        render_toc_jsonl(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 1); // One top-level entry

        let first: serde_json::Value = serde_json::from_str(lines[0])?;
        assert_eq!(first["title"], "Getting Started");
        Ok(())
    }

    #[test]
    fn test_render_toc_paginated_text() -> Result<()> {
        let entries = vec![TocPaginatedEntry {
            alias: "react".to_string(),
            source: "react".to_string(),
            heading_path: vec!["Hooks".to_string(), "useEffect".to_string()],
            raw_heading_path: vec![],
            heading_path_normalized: vec![],
            heading_level: 2,
            lines: "100-150".to_string(),
            anchor: None,
        }];
        let data = TocPaginatedOutput::new(entries, 1, 5, 100, Some(20));
        let options = TocRenderOptions::default();
        let mut buf = Cursor::new(Vec::new());
        render_toc_paginated_text(&data, &options, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("useEffect"));
        assert!(output.contains("Page 1 of 5"));
        assert!(output.contains("100 total results"));
        Ok(())
    }

    #[test]
    fn test_render_toc_paginated_json() -> Result<()> {
        let data = TocPaginatedOutput::new(vec![], 2, 10, 200, Some(20));
        let mut buf = Cursor::new(Vec::new());
        render_toc_paginated_json(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        let parsed: serde_json::Value = serde_json::from_str(&output)?;

        assert_eq!(parsed["page"], 2);
        assert_eq!(parsed["total_pages"], 10);
        assert_eq!(parsed["total_results"], 200);
        assert_eq!(parsed["page_size"], 20);
        Ok(())
    }

    #[test]
    fn test_render_toc_paginated_jsonl() -> Result<()> {
        let entries = vec![TocPaginatedEntry {
            alias: "react".to_string(),
            source: "react".to_string(),
            heading_path: vec!["Hooks".to_string()],
            raw_heading_path: vec![],
            heading_path_normalized: vec![],
            heading_level: 1,
            lines: "1-100".to_string(),
            anchor: None,
        }];
        let data = TocPaginatedOutput::new(entries, 1, 1, 1, None);
        let mut buf = Cursor::new(Vec::new());
        render_toc_paginated_jsonl(&data, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2); // metadata + 1 entry

        let metadata: serde_json::Value = serde_json::from_str(lines[0])?;
        assert_eq!(metadata["page"], 1);

        let entry: serde_json::Value = serde_json::from_str(lines[1])?;
        assert_eq!(entry["alias"], "react");
        Ok(())
    }

    #[test]
    fn test_render_toc_multi_text() -> Result<()> {
        let react = TocOutput::new("react", vec![sample_toc_entry()]);
        let bun = TocOutput::new(
            "bun",
            vec![TocEntry {
                level: 1,
                title: "Installation".to_string(),
                lines: "1-30".to_string(),
                anchor: None,
                heading_path: vec!["Installation".to_string()],
                children: vec![],
            }],
        );

        let data = TocMultiOutput::new(vec![react, bun]);
        let options = TocRenderOptions::default();
        let mut buf = Cursor::new(Vec::new());
        render_toc_multi_text(&data, &options, &mut buf)?;

        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("react"));
        assert!(output.contains("bun"));
        assert!(output.contains("Getting Started"));
        assert!(output.contains("Installation"));
        Ok(())
    }

    #[test]
    fn test_render_toc_dispatcher() -> Result<()> {
        let data = TocOutput::new("react", vec![sample_toc_entry()]);
        let shape: OutputShape = data.into();

        // Test JSON through dispatcher
        let mut buf = Cursor::new(Vec::new());
        render(&shape, OutputFormat::Json, &mut buf)?;
        let output = String::from_utf8(buf.into_inner())?;
        let parsed: serde_json::Value = serde_json::from_str(&output)?;
        assert_eq!(parsed["alias"], "react");

        // Test text through dispatcher
        let data = TocOutput::new("react", vec![sample_toc_entry()]);
        let shape: OutputShape = data.into();
        let mut buf = Cursor::new(Vec::new());
        render(&shape, OutputFormat::Text, &mut buf)?;
        let output = String::from_utf8(buf.into_inner())?;
        assert!(output.contains("Table of contents for"));

        Ok(())
    }

    #[test]
    fn test_render_toc_raw_returns_error() {
        let data = TocOutput::new("react", vec![]);
        let shape: OutputShape = data.into();
        let mut buf = Cursor::new(Vec::new());

        let result = render(&shape, OutputFormat::Raw, &mut buf);
        assert!(result.is_err());
    }
}
