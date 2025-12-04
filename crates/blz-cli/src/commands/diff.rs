//! Diff command implementation

use anyhow::{Context, Result};
use blz_core::{LlmsJson, Storage, compute_anchor_mappings};
use serde_json::json;
use std::fs;

/// Show diffs for a source between the latest archived snapshot and current
/// state. If no archive exists, a helpful message is printed.
#[allow(clippy::too_many_lines)]
pub async fn show(alias: &str, since: Option<&str>) -> Result<()> {
    let storage = Storage::new()?;
    let canonical = crate::utils::resolver::resolve_source(&storage, alias)?
        .unwrap_or_else(|| alias.to_string());

    if !storage.exists(&canonical) {
        println!("Source '{alias}' not found. Try 'blz list' or 'blz lookup' to add one.");
        return Ok(());
    }

    let current: LlmsJson = storage.load_llms_json(&canonical)?;
    let Some(prev_path) = find_previous_llms_json(&storage, &canonical, since)? else {
        println!(
            "No previous snapshot found for '{canonical}'. Run 'blz update' to create history."
        );
        return Ok(());
    };
    let prev_json_text = fs::read_to_string(&prev_path)?;
    let prev: LlmsJson = serde_json::from_str(&prev_json_text)?;

    // Read contents for content diffs
    let current_text = storage
        .load_llms_txt(&canonical)
        .with_context(|| format!("Failed to load current llms.txt for '{canonical}'"))?;
    let prev_txt_path =
        prev_path.with_file_name(prev_path.file_name().and_then(|n| n.to_str()).map_or_else(
            || "llms.txt".to_string(),
            |n| n.replace("-llms.json", "-llms.txt"),
        ));
    let prev_llms_text = std::fs::read_to_string(&prev_txt_path).with_context(|| {
        format!(
            "Failed to read previous llms.txt at {}",
            prev_txt_path.display()
        )
    })?;

    // Build maps of anchors for added/removed detection
    let (prev_anchors, prev_map) = collect_anchors(&prev);
    let (curr_anchors, curr_map) = collect_anchors(&current);

    // Compute moved sections via anchor mapping
    let moved = compute_anchor_mappings(&prev.toc, &current.toc);
    let moved_enriched: Vec<serde_json::Value> = moved
        .into_iter()
        .map(|m| {
            let oldc = slice_content(&prev_llms_text, &m.old_lines);
            let newc = slice_content(&current_text, &m.new_lines);

            let display_current = curr_map
                .get(&m.anchor)
                .and_then(|v| v.get("headingPath"))
                .cloned();
            let raw_current = curr_map
                .get(&m.anchor)
                .and_then(|v| v.get("rawHeadingPath"))
                .cloned();
            let normalized_current = curr_map
                .get(&m.anchor)
                .and_then(|v| v.get("headingPathNormalized"))
                .cloned();

            let display_previous = prev_map
                .get(&m.anchor)
                .and_then(|v| v.get("headingPath"))
                .cloned();
            let raw_previous = prev_map
                .get(&m.anchor)
                .and_then(|v| v.get("rawHeadingPath"))
                .cloned();
            let normalized_previous = prev_map
                .get(&m.anchor)
                .and_then(|v| v.get("headingPathNormalized"))
                .cloned();

            let heading_path_value = display_current
                .or(display_previous)
                .unwrap_or_else(|| json!(m.heading_path));
            let raw_heading_path_value = raw_current
                .or(raw_previous)
                .unwrap_or_else(|| json!(m.heading_path));
            let normalized_heading_value = normalized_current
                .or(normalized_previous)
                .unwrap_or(serde_json::Value::Null);

            json!({
                "anchor": m.anchor,
                "headingPath": heading_path_value,
                "rawHeadingPath": raw_heading_path_value,
                "headingPathNormalized": normalized_heading_value,
                "oldLines": m.old_lines,
                "newLines": m.new_lines,
                "oldContent": oldc,
                "newContent": newc,
            })
        })
        .collect();

    // Added: in current not in previous
    let added = curr_anchors
        .difference(&prev_anchors)
        .filter_map(|a| curr_map.get(a).cloned())
        .map(|mut v| {
            if let Some(obj) = v.as_object_mut() {
                if let Some(lines) = obj.get("lines").and_then(|x| x.as_str()) {
                    obj.insert("content".into(), json!(slice_content(&current_text, lines)));
                }
            }
            v
        })
        .collect::<Vec<_>>();

    // Removed: in previous not in current
    let removed = prev_anchors
        .difference(&curr_anchors)
        .filter_map(|a| prev_map.get(a).cloned())
        .map(|mut v| {
            if let Some(obj) = v.as_object_mut() {
                if let Some(lines) = obj.get("lines").and_then(|x| x.as_str()) {
                    obj.insert(
                        "content".into(),
                        json!(slice_content(&prev_llms_text, lines)),
                    );
                }
            }
            v
        })
        .collect::<Vec<_>>();

    // Text output by default
    println!(
        "Diff for {}\n  moved: {}\n  added: {}\n  removed: {}",
        canonical,
        moved_enriched.len(),
        added.len(),
        removed.len()
    );

    // Also emit JSON to stdout for tooling (pretty)
    let payload = json!({
        "alias": alias,
        "source": canonical,
        "previous": {
            "sha256": prev.metadata.sha256,
        },
        "current": {
            "sha256": current.metadata.sha256,
        },
        "moved": moved_enriched,
        "added": added,
        "removed": removed,
    });
    println!("\n{}", serde_json::to_string_pretty(&payload)?);

    Ok(())
}

fn find_previous_llms_json(
    storage: &Storage,
    alias: &str,
    since: Option<&str>,
) -> Result<Option<std::path::PathBuf>> {
    let dir = storage.archive_dir(alias)?;
    if !dir.exists() {
        return Ok(None);
    }

    let mut candidates = fs::read_dir(&dir)
        .with_context(|| format!("Failed to read archive directory: {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("-llms.json"))
        })
        .collect::<Vec<_>>();

    // Sort descending by filename
    candidates.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    if let Some(since_ts) = since {
        // Find the first archive whose timestamp >= since
        if let Some(p) = candidates.iter().find(|p| {
            let needle = format!("{since_ts}-llms.json");
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n >= needle.as_str())
        }) {
            return Ok(Some(p.clone()));
        }
    }

    Ok(candidates.into_iter().next())
}

fn collect_anchors(
    doc: &LlmsJson,
) -> (
    std::collections::BTreeSet<String>,
    std::collections::HashMap<String, serde_json::Value>,
) {
    fn walk(
        set: &mut std::collections::BTreeSet<String>,
        map: &mut std::collections::HashMap<String, serde_json::Value>,
        list: &[blz_core::TocEntry],
    ) {
        for e in list {
            if let Some(a) = e.anchor.as_ref() {
                set.insert(a.clone());
                let display_path = sanitized_path(e);
                map.insert(
                    a.clone(),
                    json!({
                        "anchor": a,
                        "headingPath": display_path,
                        "rawHeadingPath": &e.heading_path,
                        "headingPathNormalized": &e.heading_path_normalized,
                        "lines": &e.lines,
                    }),
                );
            }
            if !e.children.is_empty() {
                walk(set, map, &e.children);
            }
        }
    }
    let mut set = std::collections::BTreeSet::new();
    let mut map: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
    walk(&mut set, &mut map, &doc.toc);
    (set, map)
}

fn slice_content(all: &str, lines_spec: &str) -> String {
    let mut parts = lines_spec.split(['-', ':']);
    let start = parts
        .next()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(1);
    let end = parts
        .next()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(start);
    let mut out = String::new();
    for (idx, line) in all.lines().enumerate() {
        let n = idx + 1;
        if n < start {
            continue;
        }
        if n > end {
            break;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
    }
    out
}

fn sanitized_path(entry: &blz_core::TocEntry) -> Vec<String> {
    entry
        .heading_path_display
        .clone()
        .unwrap_or_else(|| entry.heading_path.clone())
}
