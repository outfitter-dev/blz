use std::collections::HashSet;
use std::path::Path;

use anyhow::{Result, anyhow};
use blz_core::{
    Fetcher, FileInfo, Flavor, LineIndex, LlmsJson, ParseMeta, ParseResult, Source, Storage,
};
use chrono::Utc;
use tracing::{debug, warn};
use url::Url;

use crate::utils::preferences;
use crate::utils::settings;
use crate::utils::settings::PreferenceScope;
use crate::utils::store;

pub const BASE_FLAVOR: &str = Flavor::Llms.as_str();
pub const FULL_FLAVOR: &str = Flavor::LlmsFull.as_str();

/// Feature flag: Force preference for llms-full.txt when available.
/// When true, all flavor resolution logic prefers llms-full.txt first,
/// falling back to llms.txt only when full is unavailable.
/// This simplifies UX by eliminating configuration complexity.
pub const FORCE_PREFER_FULL: bool = true;

#[derive(Clone, Debug)]
pub struct FlavorCandidate {
    pub flavor_id: String,
    pub file_name: String,
    pub url: String,
}

pub fn file_name_from_url(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()
        .and_then(|parsed| {
            parsed
                .path_segments()
                .and_then(|mut segments| segments.next_back().map(str::to_string))
        })
        .filter(|name| !name.is_empty())
}

fn fallback_flavor_id(name: &str) -> Option<String> {
    let stem = Path::new(name).file_stem().and_then(|s| s.to_str())?.trim();
    if stem.is_empty() {
        return None;
    }
    Some(stem.to_ascii_lowercase())
}

fn push_candidate(
    candidates: &mut Vec<FlavorCandidate>,
    seen: &mut HashSet<String>,
    name: String,
    url: String,
) {
    let flavor = Storage::flavor_from_url(&url);
    let mut flavor_id = flavor.as_str().to_string();

    // Preserve custom suffixes (e.g., llms-preview) when flavor detection falls back to base flavor.
    if flavor == Flavor::Llms && !name.eq_ignore_ascii_case("llms.txt") {
        if let Some(candidate) = fallback_flavor_id(&name) {
            if candidate != BASE_FLAVOR {
                flavor_id = candidate;
            }
        }
    } else if flavor == Flavor::LlmsFull && !name.eq_ignore_ascii_case("llms-full.txt") {
        if let Some(candidate) = fallback_flavor_id(&name) {
            if candidate != FULL_FLAVOR {
                flavor_id = candidate;
            }
        }
    }

    if seen.insert(flavor_id.clone()) {
        candidates.push(FlavorCandidate {
            flavor_id,
            file_name: name,
            url,
        });
    }
}

pub async fn discover_flavor_candidates(
    fetcher: &Fetcher,
    url: &str,
) -> Result<Vec<FlavorCandidate>> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    match fetcher.check_flavors(url).await {
        Ok(list) if !list.is_empty() => {
            for info in list {
                let flavor = Storage::flavor_from_url(&info.url);
                if matches!(flavor, Flavor::Llms | Flavor::LlmsFull) {
                    push_candidate(
                        &mut candidates,
                        &mut seen,
                        info.name.clone(),
                        info.url.clone(),
                    );
                }
            }
        },
        Ok(_) => {},
        Err(err) => {
            warn!("failed to enumerate llms flavors at {url}: {err}");
        },
    }

    if !seen.contains(BASE_FLAVOR) {
        if let Some(name) = file_name_from_url(url) {
            push_candidate(&mut candidates, &mut seen, name, url.to_string());
        }
    }

    if seen.is_empty() {
        push_candidate(
            &mut candidates,
            &mut seen,
            "llms.txt".to_string(),
            url.to_string(),
        );
    }

    // Feature flag: Sort to prefer llms-full.txt first when forced
    candidates.sort_by_key(|c| match c.flavor_id.as_str() {
        FULL_FLAVOR if FORCE_PREFER_FULL => 0,
        BASE_FLAVOR if FORCE_PREFER_FULL => 1,
        BASE_FLAVOR => 0,
        FULL_FLAVOR => 1,
        _ => 2,
    });

    Ok(candidates)
}

pub fn build_llms_json(
    alias: &str,
    url: &str,
    file_name: &str,
    sha256: String,
    etag: Option<String>,
    last_modified: Option<String>,
    parse_result: &ParseResult,
) -> LlmsJson {
    LlmsJson {
        alias: alias.to_string(),
        source: Source {
            url: url.to_string(),
            etag,
            last_modified,
            fetched_at: Utc::now(),
            sha256: sha256.clone(),
            aliases: Vec::new(),
        },
        toc: parse_result.toc.clone(),
        files: vec![FileInfo {
            path: file_name.to_string(),
            sha256,
        }],
        line_index: LineIndex {
            total_lines: parse_result.line_count,
            byte_offsets: false,
        },
        diagnostics: parse_result.diagnostics.clone(),
        parse_meta: Some(ParseMeta {
            parser_version: 1,
            segmentation: "structured".to_string(),
        }),
    }
}

/// Resolve the effective flavor for a given alias by merging per-source
/// overrides with scope-level defaults and on-disk availability.
pub fn resolve_flavor(storage: &Storage, alias: &str) -> Result<String> {
    let available_raw = storage.available_flavors(alias)?;

    // Normalize available flavors to simplify comparisons.
    let mut available: Vec<String> = available_raw
        .into_iter()
        .filter_map(|flavor| normalize_flavor(&flavor))
        .collect();

    if available.is_empty() {
        // No persisted flavors yet; default to base flavor for downstream logic.
        return Ok(BASE_FLAVOR.to_string());
    }

    let available_set: HashSet<&str> = available.iter().map(String::as_str).collect();

    // Feature flag: Always prefer llms-full.txt when available
    if FORCE_PREFER_FULL && available_set.contains(FULL_FLAVOR) {
        return Ok(FULL_FLAVOR.to_string());
    }

    if let Some(preferred) = per_source_override(alias) {
        if available_set.contains(preferred.as_str()) {
            return Ok(preferred);
        }
        debug!(
            alias = alias,
            preferred = preferred.as_str(),
            "preferred flavor missing on disk; falling back"
        );
    }

    let prefer_full = settings::effective_prefer_llms_full();
    if prefer_full && available_set.contains(FULL_FLAVOR) {
        return Ok(FULL_FLAVOR.to_string());
    }

    if available_set.contains(BASE_FLAVOR) {
        return Ok(BASE_FLAVOR.to_string());
    }

    // Fallback: return the first available flavor (already normalized)
    available.sort();
    Ok(available.remove(0))
}

/// Set or clear the preferred flavor override for a given alias in the
/// requested scope. Passing `None` clears the override.
pub fn set_preferred_flavor(
    scope: PreferenceScope,
    alias: &str,
    flavor: Option<&str>,
) -> Result<()> {
    let scope_key = scope_key(scope)?;
    let mut store = store::load_store();

    let normalized = match flavor {
        Some(raw) => {
            normalize_flavor(raw).ok_or_else(|| anyhow!("flavor value must not be empty"))?
        },
        None => String::new(),
    };

    {
        let record = store.scopes.entry(scope_key.clone()).or_default();
        if flavor.is_some() {
            record
                .sources
                .entry(alias.to_string())
                .or_default()
                .preferred_flavor = Some(normalized);
        } else {
            if let Some(entry) = record.sources.get_mut(alias) {
                entry.preferred_flavor = None;
            }
            record
                .sources
                .retain(|_, overrides| overrides.preferred_flavor.is_some());
        }
    }

    // Remove the scope if completely empty to keep the store tidy.
    if flavor.is_none() {
        if let Some(record) = store.scopes.get(&scope_key) {
            let empty_sources = record.sources.is_empty();
            let empty_cli = record.cli_preferences.is_null();
            let empty_settings = record.user_settings.is_null();
            if empty_sources && empty_cli && empty_settings {
                store.scopes.remove(&scope_key);
            }
        }
    }

    store::save_store(&store)?;
    Ok(())
}

fn per_source_override(alias: &str) -> Option<String> {
    let store = store::load_store();
    let mut resolved: Option<String> = None;
    for scope_key in preferences::scope_chain() {
        if let Some(record) = store.scopes.get(&scope_key) {
            if let Some(overrides) = record.sources.get(alias) {
                if let Some(raw) = overrides.preferred_flavor.as_deref() {
                    if let Some(normalized) = normalize_flavor(raw) {
                        resolved = Some(normalized);
                    }
                }
            }
        }
    }
    resolved
}

pub fn normalize_flavor(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut normalized = trimmed.to_ascii_lowercase();
    if let Some(stripped) = normalized.strip_suffix(".txt") {
        normalized = stripped.to_string();
    }
    if let Some(stripped) = normalized.strip_suffix(".json") {
        normalized = stripped.to_string();
    }
    normalized = normalized.replace('_', "-").replace(' ', "");

    if normalized.is_empty() {
        return None;
    }

    let mapped = match normalized.as_str() {
        "llms" => BASE_FLAVOR.to_string(),
        "llms-full" | "llmsfull" => FULL_FLAVOR.to_string(),
        other => other.to_string(),
    };
    Some(mapped)
}

fn scope_key(scope: PreferenceScope) -> Result<String> {
    match scope {
        PreferenceScope::Global => Ok("global".to_string()),
        PreferenceScope::Project => preferences::project_scope_key()
            .ok_or_else(|| anyhow!("unable to determine project scope key")),
        PreferenceScope::Local => preferences::local_scope_key()
            .ok_or_else(|| anyhow!("unable to determine local scope key")),
    }
}

#[cfg(test)]
#[allow(unsafe_code, clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use anyhow::Result;
    use blz_core::{LlmsJson, Source};
    use chrono::Utc;
    use tempfile::tempdir;

    fn sample_json(alias: &str, url: &str) -> LlmsJson {
        LlmsJson {
            alias: alias.to_string(),
            source: Source {
                url: url.to_string(),
                etag: None,
                last_modified: None,
                fetched_at: Utc::now(),
                sha256: "abc".into(),
                aliases: Vec::new(),
            },
            toc: Vec::new(),
            files: Vec::new(),
            line_index: blz_core::LineIndex {
                total_lines: 0,
                byte_offsets: false,
            },
            diagnostics: Vec::new(),
            parse_meta: None,
        }
    }

    fn with_temp_env<F: FnOnce(Storage, &std::path::Path) -> Result<()>>(f: F) -> Result<()> {
        use std::ffi::OsString;

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
        }

        impl Drop for EnvGuard {
            fn drop(&mut self) {
                unsafe {
                    match &self.original {
                        Some(value) => std::env::set_var(self.key, value),
                        None => std::env::remove_var(self.key),
                    }
                }
            }
        }

        let _env_guard = crate::utils::test_support::env_mutex()
            .lock()
            .expect("env mutex poisoned");

        let storage_dir = tempdir()?;
        let storage = Storage::with_root(storage_dir.path().to_path_buf())?;

        let config_dir = tempdir()?;
        let config_guard = EnvGuard::new("BLZ_CONFIG_DIR");
        let global_guard = EnvGuard::new("BLZ_GLOBAL_CONFIG_DIR");
        let prefer_guard = EnvGuard::new("BLZ_PREFER_LLMS_FULL");
        // keep guards alive until end of scope so drops restore env state
        let _restore_guards = (config_guard, global_guard, prefer_guard);

        unsafe {
            std::env::set_var("BLZ_CONFIG_DIR", config_dir.path());
            std::env::set_var("BLZ_GLOBAL_CONFIG_DIR", config_dir.path());
            std::env::remove_var("BLZ_PREFER_LLMS_FULL");
        }

        f(storage, config_dir.path())
    }

    #[test]
    fn resolve_defaults_to_base_when_only_llms_present() -> Result<()> {
        with_temp_env(|storage, _config_dir| {
            let json = sample_json("react", "https://example.com/llms.txt");
            storage.save_flavor_json("react", BASE_FLAVOR, &json)?;
            let flavor = resolve_flavor(&storage, "react")?;
            assert_eq!(flavor, BASE_FLAVOR);
            Ok(())
        })
    }

    #[test]
    fn resolve_prefers_full_when_available_and_global_pref_true() -> Result<()> {
        with_temp_env(|storage, _config_dir| {
            let json = sample_json("react", "https://example.com/llms.txt");
            storage.save_flavor_json("react", BASE_FLAVOR, &json)?;
            storage.save_flavor_json("react", FULL_FLAVOR, &json)?;

            unsafe {
                std::env::set_var("BLZ_PREFER_LLMS_FULL", "1");
            }

            let result = resolve_flavor(&storage, "react");

            unsafe {
                std::env::remove_var("BLZ_PREFER_LLMS_FULL");
            }

            let flavor = result?;
            assert_eq!(flavor, FULL_FLAVOR);
            Ok(())
        })
    }

    #[test]
    fn resolve_respects_per_source_override() -> Result<()> {
        with_temp_env(|storage, _config_dir| {
            let json = sample_json("react", "https://example.com/llms.txt");
            storage.save_flavor_json("react", BASE_FLAVOR, &json)?;
            storage.save_flavor_json("react", FULL_FLAVOR, &json)?;

            set_preferred_flavor(PreferenceScope::Local, "react", Some("llms"))?;

            let flavor = resolve_flavor(&storage, "react")?;
            assert_eq!(flavor, BASE_FLAVOR);
            Ok(())
        })
    }

    #[test]
    fn clearing_override_removes_entry() -> Result<()> {
        with_temp_env(|storage, _config_dir| {
            let json = sample_json("vue", "https://example.com/llms.txt");
            storage.save_flavor_json("vue", BASE_FLAVOR, &json)?;
            storage.save_flavor_json("vue", FULL_FLAVOR, &json)?;

            set_preferred_flavor(PreferenceScope::Local, "vue", Some("llms-full"))?;
            set_preferred_flavor(PreferenceScope::Local, "vue", None)?;

            let store = store::load_store();
            let key = preferences::local_scope_key().expect("local scope key");
            assert!(
                !store
                    .scopes
                    .get(&key)
                    .is_some_and(|record| record.sources.contains_key("vue"))
            );

            let flavor = resolve_flavor(&storage, "vue")?;
            assert_eq!(flavor, BASE_FLAVOR);
            Ok(())
        })
    }

    #[test]
    fn override_falls_back_when_flavor_missing() -> Result<()> {
        with_temp_env(|storage, _config_dir| {
            let json = sample_json("bun", "https://example.com/llms.txt");
            storage.save_flavor_json("bun", BASE_FLAVOR, &json)?;

            set_preferred_flavor(PreferenceScope::Global, "bun", Some("llms-full"))?;
            let flavor = resolve_flavor(&storage, "bun")?;
            assert_eq!(flavor, BASE_FLAVOR);
            Ok(())
        })
    }
}
