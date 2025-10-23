//! Refresh command implementation

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Result, anyhow};
use blz_core::{
    FetchResult, Fetcher, MarkdownParser, PerformanceMetrics, SearchIndex, Source, Storage,
};
use chrono::Utc;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::utils::count_headings;
use crate::utils::json_builder::build_llms_json;
use crate::utils::resolver;
use crate::utils::url_resolver;

/// Abstraction over storage interactions used by the refresh command.
pub trait RefreshStorage {
    fn load_metadata(&self, alias: &str) -> Result<Source>;
    fn load_llms_aliases(&self, alias: &str) -> Result<Vec<String>>;
    fn save_llms_txt(&self, alias: &str, content: &str) -> Result<()>;
    fn save_llms_json(&self, alias: &str, data: &blz_core::LlmsJson) -> Result<()>;
    fn save_metadata(&self, alias: &str, metadata: &Source) -> Result<()>;
    fn index_path(&self, alias: &str) -> Result<PathBuf>;
}

#[allow(clippy::use_self)]
impl RefreshStorage for Storage {
    fn load_metadata(&self, alias: &str) -> Result<Source> {
        Storage::load_source_metadata(self, alias)
            .map_err(anyhow::Error::from)?
            .ok_or_else(|| anyhow!("Missing metadata for {alias}"))
    }

    fn load_llms_aliases(&self, alias: &str) -> Result<Vec<String>> {
        match Storage::load_llms_json(self, alias) {
            Ok(llms) => Ok(llms.metadata.aliases),
            Err(_) => Ok(Vec::new()),
        }
    }

    fn save_llms_txt(&self, alias: &str, content: &str) -> Result<()> {
        Storage::save_llms_txt(self, alias, content).map_err(anyhow::Error::from)
    }

    fn save_llms_json(&self, alias: &str, data: &blz_core::LlmsJson) -> Result<()> {
        Storage::save_llms_json(self, alias, data).map_err(anyhow::Error::from)
    }

    fn save_metadata(&self, alias: &str, metadata: &Source) -> Result<()> {
        Storage::save_source_metadata(self, alias, metadata).map_err(anyhow::Error::from)
    }

    fn index_path(&self, alias: &str) -> Result<PathBuf> {
        Storage::index_dir(self, alias).map_err(anyhow::Error::from)
    }
}

/// Interface for indexing refreshed content.
pub trait RefreshIndexer {
    fn index(
        &self,
        alias: &str,
        index_path: &std::path::Path,
        metrics: PerformanceMetrics,
        blocks: &[blz_core::HeadingBlock],
    ) -> Result<()>;
}

#[derive(Default)]
struct DefaultIndexer;

impl RefreshIndexer for DefaultIndexer {
    fn index(
        &self,
        alias: &str,
        index_path: &std::path::Path,
        metrics: PerformanceMetrics,
        blocks: &[blz_core::HeadingBlock],
    ) -> Result<()> {
        let index = SearchIndex::create_or_open(index_path)?.with_metrics(metrics);
        index
            .index_blocks(alias, blocks)
            .map_err(anyhow::Error::from)
    }
}

/// Result summary for a refresh operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshOutcome {
    Refreshed {
        alias: String,
        headings: usize,
        lines: usize,
    },
    Unchanged {
        alias: String,
    },
}

/// Data describing remote changes.
#[derive(Debug, Clone)]
pub struct RefreshPayload {
    pub content: String,
    pub sha256: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// Apply a refresh: persist content and re-index the source.
pub fn apply_refresh<S, I>(
    storage: &S,
    alias: &str,
    existing_metadata: Source,
    existing_aliases: Vec<String>,
    payload: RefreshPayload,
    metrics: PerformanceMetrics,
    indexer: &I,
) -> Result<RefreshOutcome>
where
    S: RefreshStorage,
    I: RefreshIndexer,
{
    let mut parser = MarkdownParser::new()?;
    let parse_result = parser.parse(&payload.content)?;

    storage.save_llms_txt(alias, &payload.content)?;

    let mut llms_json = build_llms_json(
        alias,
        &existing_metadata.url,
        "llms.txt",
        payload.sha256.clone(),
        payload.etag.clone(),
        payload.last_modified.clone(),
        &parse_result,
    );

    let mut metadata_aliases = existing_aliases;
    for alias_value in &existing_metadata.aliases {
        if !metadata_aliases.contains(alias_value) {
            metadata_aliases.push(alias_value.clone());
        }
    }
    metadata_aliases.sort();
    metadata_aliases.dedup();
    llms_json.metadata.aliases = metadata_aliases;
    llms_json.metadata.tags.clone_from(&existing_metadata.tags);
    llms_json
        .metadata
        .description
        .clone_from(&existing_metadata.description);
    llms_json
        .metadata
        .category
        .clone_from(&existing_metadata.category);
    llms_json
        .metadata
        .npm_aliases
        .clone_from(&existing_metadata.npm_aliases);
    llms_json
        .metadata
        .github_aliases
        .clone_from(&existing_metadata.github_aliases);
    llms_json.metadata.variant = existing_metadata.variant.clone();
    storage.save_llms_json(alias, &llms_json)?;

    let mut origin = existing_metadata.origin.clone();
    origin.source_type = match (&origin.source_type, &existing_metadata.origin.source_type) {
        (Some(blz_core::SourceType::Remote { .. }), _) | (None, None) => {
            Some(blz_core::SourceType::Remote {
                url: existing_metadata.url.clone(),
            })
        },
        (Some(blz_core::SourceType::LocalFile { path }), _) => {
            Some(blz_core::SourceType::LocalFile { path: path.clone() })
        },
        (None, Some(existing)) => Some(existing.clone()),
    };

    llms_json.metadata.origin = origin.clone();

    let metadata = Source {
        url: existing_metadata.url,
        etag: payload.etag,
        last_modified: payload.last_modified,
        fetched_at: Utc::now(),
        sha256: payload.sha256,
        variant: existing_metadata.variant,
        aliases: existing_metadata.aliases,
        tags: existing_metadata.tags,
        description: existing_metadata.description,
        category: existing_metadata.category,
        npm_aliases: existing_metadata.npm_aliases,
        github_aliases: existing_metadata.github_aliases,
        origin,
    };
    storage.save_metadata(alias, &metadata)?;

    let index_path = storage.index_path(alias)?;
    indexer.index(
        alias,
        index_path.as_path(),
        metrics,
        &parse_result.heading_blocks,
    )?;

    Ok(RefreshOutcome::Refreshed {
        alias: alias.to_string(),
        headings: count_headings(&llms_json.toc),
        lines: llms_json.line_index.total_lines,
    })
}

fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message(message.to_string());
    pb
}

/// Execute refresh for a specific source.
#[allow(clippy::too_many_lines)]
pub async fn execute(alias: &str, metrics: PerformanceMetrics, quiet: bool) -> Result<()> {
    let storage = Storage::new()?;
    let canonical_alias =
        resolver::resolve_source(&storage, alias)?.unwrap_or_else(|| alias.to_string());

    if !storage.exists(&canonical_alias) {
        return Err(anyhow!("Source '{alias}' not found"));
    }

    let spinner = if quiet {
        ProgressBar::hidden()
    } else {
        create_spinner(format!("Checking {canonical_alias}...").as_str())
    };

    let start = Instant::now();
    let existing_metadata = storage.load_metadata(&canonical_alias)?;
    let existing_aliases = storage.load_llms_aliases(&canonical_alias)?;
    let fetcher = Fetcher::new()?;

    // Check for URL upgrades (llms.txt -> llms-full.txt)
    let (final_url, updated_variant) = if existing_metadata.variant == blz_core::SourceVariant::Llms
    {
        // Try to resolve a better variant
        match url_resolver::resolve_best_url(&fetcher, &existing_metadata.url).await {
            Ok(resolved) if resolved.variant == blz_core::SourceVariant::LlmsFull => {
                if !quiet {
                    spinner.finish_and_clear();
                    println!(
                        "{} llms-full.txt is now available for {}",
                        "✨".green(),
                        canonical_alias.green()
                    );
                    println!(
                        "  Upgrading from {} to {}",
                        "llms.txt".yellow(),
                        "llms-full.txt".green()
                    );
                }
                (resolved.final_url, resolved.variant)
            },
            Ok(_resolved) => {
                // No upgrade available, use existing URL
                (
                    existing_metadata.url.clone(),
                    existing_metadata.variant.clone(),
                )
            },
            Err(_) => {
                // Resolution failed, use existing URL
                (
                    existing_metadata.url.clone(),
                    existing_metadata.variant.clone(),
                )
            },
        }
    } else {
        // Already using llms-full or custom URL, no upgrade needed
        (
            existing_metadata.url.clone(),
            existing_metadata.variant.clone(),
        )
    };

    let fetch_result = fetcher
        .fetch_with_cache(
            &final_url,
            existing_metadata.etag.as_deref(),
            existing_metadata.last_modified.as_deref(),
        )
        .await?;

    let outcome = match fetch_result {
        FetchResult::NotModified { .. } => {
            spinner.finish_and_clear();
            if !quiet {
                println!("{} {} (unchanged)", "✓".green(), canonical_alias.green());
            }
            RefreshOutcome::Unchanged {
                alias: canonical_alias.clone(),
            }
        },
        FetchResult::Modified {
            content,
            sha256,
            etag,
            last_modified,
        } => {
            spinner.set_message(format!("Parsing {canonical_alias}..."));
            let payload = RefreshPayload {
                content,
                sha256,
                etag,
                last_modified,
            };
            let indexer = DefaultIndexer;

            // Refresh metadata with new URL and variant if upgraded
            let mut updated_metadata = existing_metadata.clone();
            updated_metadata.url = final_url;
            updated_metadata.variant = updated_variant;

            let outcome = apply_refresh(
                &storage,
                &canonical_alias,
                updated_metadata,
                existing_aliases,
                payload,
                metrics,
                &indexer,
            )?;
            spinner.finish_and_clear();
            outcome
        },
    };

    if !quiet {
        let elapsed = start.elapsed();
        match outcome {
            RefreshOutcome::Refreshed {
                alias,
                headings,
                lines,
            } => println!(
                "{} {} ({} headings, {} lines) in {:?}",
                "✓ Refreshed".green(),
                alias.green(),
                headings,
                lines,
                elapsed
            ),
            RefreshOutcome::Unchanged { alias } => println!(
                "{} {} (unchanged in {:?})",
                "✓".green(),
                alias.green(),
                elapsed
            ),
        }
    }

    Ok(())
}

/// Execute refresh for all sources.
pub async fn execute_all(metrics: PerformanceMetrics, quiet: bool) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources();

    if sources.is_empty() {
        anyhow::bail!("No sources configured. Use 'blz add' to add sources.");
    }

    let fetcher = Fetcher::new()?;
    let mut refreshed_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;
    let indexer = DefaultIndexer;

    for alias in sources {
        let spinner = if quiet {
            ProgressBar::hidden()
        } else {
            create_spinner(format!("Checking {alias}...").as_str())
        };

        let metadata = storage.load_metadata(&alias)?;
        let aliases = storage.load_llms_aliases(&alias)?;
        let fetch_result = fetcher
            .fetch_with_cache(
                &metadata.url,
                metadata.etag.as_deref(),
                metadata.last_modified.as_deref(),
            )
            .await?;

        match fetch_result {
            FetchResult::NotModified { .. } => {
                spinner.finish_and_clear();
                skipped_count += 1;
                if !quiet {
                    println!("{} {} (unchanged)", "✓".green(), alias.green());
                }
            },
            FetchResult::Modified {
                content,
                sha256,
                etag,
                last_modified,
            } => {
                spinner.set_message(format!("Parsing {alias}..."));
                match apply_refresh(
                    &storage,
                    &alias,
                    metadata,
                    aliases,
                    RefreshPayload {
                        content,
                        sha256,
                        etag,
                        last_modified,
                    },
                    metrics.clone(),
                    &indexer,
                ) {
                    Ok(RefreshOutcome::Refreshed { .. }) => {
                        refreshed_count += 1;
                        spinner.finish_and_clear();
                        if !quiet {
                            println!("{} {}", "✓ Refreshed".green(), alias.green());
                        }
                    },
                    Ok(RefreshOutcome::Unchanged { .. }) => {
                        skipped_count += 1;
                        spinner.finish_and_clear();
                    },
                    Err(e) => {
                        spinner.finish_and_clear();
                        if !quiet {
                            eprintln!("{}: {}", alias.red(), e);
                        }
                        error_count += 1;
                    },
                }
            },
        }
    }

    if !quiet {
        println!(
            "\nSummary: {} refreshed, {} unchanged, {} errors",
            refreshed_count.to_string().green(),
            skipped_count,
            if error_count > 0 {
                error_count.to_string().red()
            } else {
                error_count.to_string().normal()
            }
        );
        metrics.print_summary();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    use anyhow::Result;

    #[derive(Default)]
    struct MockStorage {
        metadata: HashMap<String, Source>,
        saved_txt: RefCell<Vec<String>>, // aliases that called save_llms_txt
        saved_json: RefCell<Vec<String>>, // aliases that called save_llms_json
        saved_metadata: RefCell<Vec<Source>>,
        index_paths: HashMap<String, PathBuf>,
    }

    impl RefreshStorage for MockStorage {
        fn load_metadata(&self, alias: &str) -> Result<Source> {
            self.metadata
                .get(alias)
                .cloned()
                .ok_or_else(|| anyhow!("missing metadata"))
        }

        fn load_llms_aliases(&self, _alias: &str) -> Result<Vec<String>> {
            Ok(Vec::new())
        }

        fn save_llms_txt(&self, alias: &str, _content: &str) -> Result<()> {
            self.saved_txt.borrow_mut().push(alias.to_string());
            Ok(())
        }

        fn save_llms_json(&self, alias: &str, _data: &blz_core::LlmsJson) -> Result<()> {
            self.saved_json.borrow_mut().push(alias.to_string());
            Ok(())
        }

        fn save_metadata(&self, _alias: &str, metadata: &Source) -> Result<()> {
            self.saved_metadata.borrow_mut().push(metadata.clone());
            Ok(())
        }

        fn index_path(&self, alias: &str) -> Result<PathBuf> {
            self.index_paths
                .get(alias)
                .cloned()
                .ok_or_else(|| anyhow!("missing index path"))
        }
    }

    #[derive(Default)]
    struct MockIndexer {
        indexed: RefCell<Vec<String>>,
    }

    impl RefreshIndexer for MockIndexer {
        fn index(
            &self,
            alias: &str,
            _index_path: &std::path::Path,
            _metrics: PerformanceMetrics,
            _blocks: &[blz_core::HeadingBlock],
        ) -> Result<()> {
            self.indexed.borrow_mut().push(alias.to_string());
            Ok(())
        }
    }

    fn sample_source() -> Source {
        Source {
            url: "https://example.com".into(),
            etag: Some("etag".into()),
            last_modified: Some("Wed, 01 Oct 2025 12:00:00 GMT".into()),
            fetched_at: Utc::now(),
            sha256: "sha".into(),
            variant: blz_core::SourceVariant::Llms,
            aliases: vec!["alpha".into()],
            tags: vec!["docs".into()],
            description: Some("Example source".into()),
            category: Some("library".into()),
            npm_aliases: vec!["alpha".into()],
            github_aliases: vec!["org/alpha".into()],
            origin: blz_core::SourceOrigin {
                manifest: None,
                source_type: Some(blz_core::SourceType::Remote {
                    url: "https://example.com".into(),
                }),
            },
        }
    }

    fn sample_payload() -> RefreshPayload {
        RefreshPayload {
            content: "# Title\n\nContent".into(),
            sha256: "new-sha".into(),
            etag: Some("new-etag".into()),
            last_modified: Some("Thu, 02 Oct 2025 12:00:00 GMT".into()),
        }
    }

    #[test]
    fn apply_refresh_persists_changes() -> Result<()> {
        let mut storage = MockStorage::default();
        storage.metadata.insert("alpha".into(), sample_source());
        storage
            .index_paths
            .insert("alpha".into(), PathBuf::from("index"));

        let indexer = MockIndexer::default();
        let outcome = apply_refresh(
            &storage,
            "alpha",
            sample_source(),
            vec!["canonical".into()],
            sample_payload(),
            PerformanceMetrics::default(),
            &indexer,
        )?;

        assert!(matches!(outcome, RefreshOutcome::Refreshed { .. }));
        assert_eq!(storage.saved_txt.borrow().as_slice(), ["alpha"]);
        assert_eq!(storage.saved_json.borrow().as_slice(), ["alpha"]);
        assert_eq!(indexer.indexed.borrow().as_slice(), ["alpha"]);
        assert_eq!(storage.saved_metadata.borrow().len(), 1);
        Ok(())
    }
}
