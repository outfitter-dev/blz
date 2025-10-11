//! Bundled documentation utilities for the `blz-docs` embedded source.
//!
//! This module exposes helpers that keep the shipped llms-full guide in sync
//! with the on-disk cache. The bundled docs live behind the `blz-docs` alias
//! and remain hidden from default search unless explicitly requested.

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use blz_core::{
    MarkdownParser, PerformanceMetrics, SearchIndex, Source, SourceDescriptor, SourceOrigin,
    SourceType, SourceVariant, Storage,
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::sync::LazyLock;

use crate::utils::json_builder::build_llms_json;

/// Canonical alias for the embedded documentation source.
pub const BUNDLED_ALIAS: &str = "blz-docs";

/// Additional aliases that point at the bundled docs.
pub const BUNDLED_ALIASES: &[&str] = &["@blz"];

/// Tags applied to the bundled docs so they stay out of default search.
pub const BUNDLED_TAGS: &[&str] = &["blz", "internal", "guide"];

/// Descriptor category for bundled documentation.
const BUNDLED_CATEGORY: &str = "internal";

/// Human-readable description stored in metadata files.
const BUNDLED_DESCRIPTION: &str = "Bundled BLZ user guide (embedded llms-full).";

/// Synthetic URL persisted into metadata for reference tooling.
const BUNDLED_URL: &str = "blz://docs/embedded/blz-docs";

/// Overview copy surfaced by `blz docs` to orient new users and agents.
const OVERVIEW_TEXT: &str = "blz built-in docs alias: blz-docs (@blz)\n\
- use `blz docs search <query>` to stay scoped to the bundled guide\n\
- run `blz docs sync` after upgrading blz to refresh the guide\n\
- add normal sources with `blz add <alias> <llms-full>`; default search ignores internal docs\n\
- inspect full text with `blz docs cat` or export CLI usage via `blz docs export --format markdown`";

/// Embedded llms-full content compiled into the binary.
static BUNDLED_CONTENT: &str = include_str!("../../../../docs/llms/blz/llms-full.txt");

/// Pre-computed SHA-256 hash (base64) of the bundled content.
static BUNDLED_SHA256: LazyLock<String> = LazyLock::new(|| {
    let mut hasher = Sha256::new();
    hasher.update(BUNDLED_CONTENT.as_bytes());
    B64.encode(hasher.finalize())
});

/// Result of attempting to synchronize the bundled docs onto disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Docs already match the embedded version.
    UpToDate,
    /// Docs were newly installed.
    Installed,
    /// Docs existed but were refreshed to the latest embedded revision.
    Updated,
}

/// Ensure the bundled `blz-docs` source is installed and indexed.
pub fn sync(force: bool, metrics: PerformanceMetrics) -> Result<SyncStatus> {
    let storage = Storage::new()?;
    let needs_install = if force {
        true
    } else if !storage.exists(BUNDLED_ALIAS) {
        // Missing cache files require reinstall even if metadata exists
        true
    } else if let Some(metadata) = storage.load_source_metadata(BUNDLED_ALIAS)? {
        metadata.sha256 != *BUNDLED_SHA256
            || !has_all_tags(&metadata.tags)
            || !has_all_aliases(&metadata.aliases)
    } else {
        true
    };

    if !needs_install {
        return Ok(SyncStatus::UpToDate);
    }

    install(&storage, metrics).map(|was_existing| {
        if was_existing {
            SyncStatus::Updated
        } else {
            SyncStatus::Installed
        }
    })
}

fn install(storage: &Storage, metrics: PerformanceMetrics) -> Result<bool> {
    let alias = BUNDLED_ALIAS;
    let content = BUNDLED_CONTENT;
    let previously_exists = storage.exists(alias);

    let mut parser = MarkdownParser::new()?;
    let parse = parser.parse(content)?;

    storage
        .save_llms_txt(alias, content)
        .context("failed to persist embedded llms.txt")?;

    let llms_path = storage.llms_txt_path(alias)?.to_string_lossy().to_string();

    let mut llms_json = build_llms_json(
        alias,
        BUNDLED_URL,
        "llms.txt",
        BUNDLED_SHA256.clone(),
        None,
        None,
        &parse,
    );

    llms_json.metadata.variant = SourceVariant::LlmsFull;
    llms_json.metadata.aliases = BUNDLED_ALIASES
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    llms_json.metadata.tags = BUNDLED_TAGS
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    llms_json.metadata.description = Some(BUNDLED_DESCRIPTION.to_string());
    llms_json.metadata.category = Some(BUNDLED_CATEGORY.to_string());
    llms_json.metadata.origin = SourceOrigin {
        manifest: None,
        source_type: Some(SourceType::LocalFile {
            path: llms_path.clone(),
        }),
    };

    storage
        .save_llms_json(alias, &llms_json)
        .context("failed to persist embedded llms.json")?;

    let source_metadata = Source {
        url: BUNDLED_URL.to_string(),
        etag: None,
        last_modified: None,
        fetched_at: Utc::now(),
        sha256: BUNDLED_SHA256.clone(),
        variant: SourceVariant::LlmsFull,
        aliases: llms_json.metadata.aliases.clone(),
        tags: llms_json.metadata.tags.clone(),
        description: llms_json.metadata.description.clone(),
        category: llms_json.metadata.category.clone(),
        npm_aliases: Vec::new(),
        github_aliases: Vec::new(),
        origin: llms_json.metadata.origin.clone(),
    };

    storage
        .save_source_metadata(alias, &source_metadata)
        .context("failed to persist embedded metadata.json")?;

    let descriptor = SourceDescriptor {
        alias: alias.to_string(),
        name: Some("BLZ Bundled Docs".to_string()),
        description: Some(BUNDLED_DESCRIPTION.to_string()),
        category: Some(BUNDLED_CATEGORY.to_string()),
        tags: llms_json.metadata.tags.clone(),
        url: Some(BUNDLED_URL.to_string()),
        path: Some(llms_path),
        aliases: llms_json.metadata.aliases.clone(),
        npm_aliases: Vec::new(),
        github_aliases: Vec::new(),
        origin: llms_json.metadata.origin,
    };

    storage
        .save_descriptor(&descriptor)
        .context("failed to persist embedded source descriptor")?;

    let index_path = storage.index_dir(alias)?;
    let index = SearchIndex::create_or_open(&index_path)?.with_metrics(metrics);
    index
        .index_blocks(alias, &parse.heading_blocks)
        .context("failed to index embedded docs")?;

    Ok(previously_exists)
}

fn has_all_tags(existing: &[String]) -> bool {
    BUNDLED_TAGS.iter().all(|tag| {
        existing
            .iter()
            .any(|existing_tag| existing_tag.eq_ignore_ascii_case(tag))
    })
}

fn has_all_aliases(existing: &[String]) -> bool {
    BUNDLED_ALIASES.iter().all(|alias| {
        existing
            .iter()
            .any(|existing_alias| existing_alias.eq_ignore_ascii_case(alias))
    })
}

/// Print the overview banner.
pub fn print_overview() {
    println!("{OVERVIEW_TEXT}");
}

/// Print the bundled llms-full content to stdout.
pub fn print_full_content() {
    println!("{BUNDLED_CONTENT}");
}
