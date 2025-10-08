use anyhow::Result;
use blz_core::Storage;

/// Resolve a requested source identifier to its canonical alias.
///
/// Resolution order:
/// 1) Exact match to canonical alias
/// 2) Unique match across metadata aliases
/// - Returns Ok(None) if not found
/// - Returns Err if ambiguous across multiple sources
pub fn resolve_source(storage: &Storage, requested: &str) -> Result<Option<String>> {
    let requested_str = requested.to_string();
    let known = storage.list_sources();
    if known.contains(&requested_str) {
        return Ok(Some(requested_str));
    }

    let mut resolved_sources: Vec<String> = Vec::new();
    for src in &known {
        if let Ok(Some(metadata)) = storage.load_source_metadata(src) {
            if metadata.aliases.iter().any(|alias| alias == requested) {
                resolved_sources.push(src.clone());
                continue;
            }
        }

        if let Ok(llms) = storage.load_llms_json(src) {
            if llms.metadata.aliases.iter().any(|alias| alias == requested) {
                resolved_sources.push(src.clone());
            }
        }
    }

    match resolved_sources.len() {
        0 => Ok(None),
        1 => Ok(resolved_sources.into_iter().next()),
        _ => Err(anyhow::anyhow!(
            "Alias '{}' is ambiguous across multiple sources: {} — use --source with a canonical name",
            requested,
            resolved_sources.join(", ")
        )),
    }
}
