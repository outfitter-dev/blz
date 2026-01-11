//! JSON output formatting

use anyhow::{Context, Result, anyhow};
use blz_core::SearchHit;

/// JSON formatter for search output.
pub struct JsonFormatter;

impl JsonFormatter {
    /// Format search results as JSON with metadata
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn format_search_results_with_meta(
        hits: &[SearchHit],
        query: &str,
        total_results: usize,
        total_lines_searched: usize,
        search_time: std::time::Duration,
        page: usize,
        limit: usize,
        total_pages: usize,
        sources: &[String],
        suggestions: Option<&[serde_json::Value]>,
        _show_raw_score: bool,
        score_precision: u8,
    ) -> Result<()> {
        // Build JSON object without relying on unwrap/expect to satisfy clippy strictness
        let mut map = serde_json::Map::new();
        map.insert(
            "query".to_string(),
            serde_json::Value::String(query.to_string()),
        );
        map.insert("page".to_string(), serde_json::Value::from(page));
        map.insert("limit".to_string(), serde_json::Value::from(limit));
        map.insert(
            "totalResults".to_string(),
            serde_json::Value::from(total_results),
        );
        map.insert(
            "total_hits".to_string(),
            serde_json::Value::from(total_results),
        );
        map.insert(
            "totalPages".to_string(),
            serde_json::Value::from(total_pages),
        );
        map.insert(
            "total_pages".to_string(),
            serde_json::Value::from(total_pages),
        );
        map.insert(
            "totalLinesSearched".to_string(),
            serde_json::Value::from(total_lines_searched),
        );
        map.insert(
            "total_lines_searched".to_string(),
            serde_json::Value::from(total_lines_searched),
        );
        let ms = search_time.as_millis();
        let search_time_ms: u64 = u64::try_from(ms).unwrap_or(u64::MAX);
        map.insert(
            "searchTimeMs".to_string(),
            serde_json::Value::from(search_time_ms),
        );
        map.insert(
            "execution_time_ms".to_string(),
            serde_json::Value::from(search_time_ms),
        );
        map.insert(
            "sources".to_string(),
            serde_json::Value::Array(
                sources
                    .iter()
                    .cloned()
                    .map(serde_json::Value::from)
                    .collect(),
            ),
        );
        // Add percentage scores to each result
        let raw_max_score = hits.iter().fold(0.0_f32, |acc, hit| acc.max(hit.score));
        let max_score = if raw_max_score > 0.0 {
            raw_max_score
        } else {
            1.0_f32
        };
        let precision = score_precision.min(4);
        let results_with_percentage: Vec<serde_json::Value> = hits
            .iter()
            .map(|hit| {
                let hit_value = serde_json::to_value(hit).context("serialize SearchHit to JSON")?;
                let mut hit_map = hit_value
                    .as_object()
                    .cloned()
                    .ok_or_else(|| anyhow!("expected object for SearchHit"))?;
                if let Some(source_value) = hit_map.get("source").cloned() {
                    hit_map.entry("alias".to_string()).or_insert(source_value);
                }

                let rounded_score = if precision == 0 {
                    f64::from(hit.score.round())
                } else {
                    let factor = 10_f64.powi(i32::from(precision));
                    (f64::from(hit.score) * factor).round() / factor
                };
                hit_map.insert("score".to_string(), serde_json::Value::from(rounded_score));

                let percentage = if raw_max_score > 0.0 {
                    let percent = f64::from(hit.score) / f64::from(max_score) * 100.0_f64;
                    clamp_percentage(percent)
                } else {
                    0
                };
                hit_map.insert("scorePercentage".to_string(), serde_json::json!(percentage));
                Ok(serde_json::Value::Object(hit_map))
            })
            .collect::<anyhow::Result<_>>()?;
        map.insert(
            "results".to_string(),
            serde_json::Value::Array(results_with_percentage),
        );

        if let Some(s) = suggestions {
            if !s.is_empty() {
                map.insert(
                    "suggestions".to_string(),
                    serde_json::Value::Array(s.to_vec()),
                );
            }
        }

        let obj = serde_json::Value::Object(map);
        let json = serde_json::to_string_pretty(&obj)
            .context("serialize search results to pretty JSON")?;
        println!("{json}");
        Ok(())
    }

    /// Format search results as newline-delimited JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if a search hit cannot be serialized to JSON.
    pub fn format_search_results_jsonl(hits: &[SearchHit]) -> Result<()> {
        for hit in hits {
            let mut value = serde_json::to_value(hit).context("serialize SearchHit to JSON")?;
            if let serde_json::Value::Object(ref mut map) = value {
                if let Some(source_value) = map.get("source").cloned() {
                    map.entry("alias".to_string()).or_insert(source_value);
                }
            }
            println!("{}", serde_json::to_string(&value)?);
        }
        Ok(())
    }
}

fn clamp_percentage(percent: f64) -> u8 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        percent.round().clamp(0.0, 100.0) as u8
    }
}
