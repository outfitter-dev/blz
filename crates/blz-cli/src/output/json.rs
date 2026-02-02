//! JSON output formatting

use std::io::Write;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use blz_core::SearchHit;
use blz_core::numeric::percent_to_u8;

/// Metadata for JSON search output formatting.
///
/// This struct bundles the search context and pagination metadata
/// needed to render JSON output, reducing parameter counts.
#[derive(Debug, Clone)]
pub struct SearchJsonMetadata<'a> {
    /// Original search query.
    pub query: &'a str,
    /// Current page number (1-based).
    pub page: usize,
    /// Results per page.
    pub limit: usize,
    /// Total number of matching results.
    pub total_results: usize,
    /// Total pages available.
    pub total_pages: usize,
    /// Total lines searched across all sources.
    pub total_lines_searched: usize,
    /// Search execution time.
    pub search_time: Duration,
    /// Source aliases included in the search.
    pub sources: &'a [String],
    /// Optional fuzzy suggestions.
    pub suggestions: Option<&'a [serde_json::Value]>,
    /// Whether to include raw scores (reserved for future use).
    #[allow(dead_code)]
    pub show_raw_score: bool,
    /// Decimal precision for scores.
    pub score_precision: u8,
}

/// JSON formatter for search output.
pub struct JsonFormatter;

impl JsonFormatter {
    /// Format search results as JSON with metadata
    pub fn format_search_results_with_meta(
        hits: &[SearchHit],
        metadata: &SearchJsonMetadata<'_>,
    ) -> Result<()> {
        let mut map = build_metadata_map(metadata);

        let results_with_percentage = format_hits_with_scores(hits, metadata.score_precision)?;
        map.insert(
            "results".to_string(),
            serde_json::Value::Array(results_with_percentage),
        );

        if let Some(s) = metadata.suggestions {
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
    /// Streams items one at a time to handle large result sets without
    /// buffering everything in memory.
    ///
    /// Broken pipe errors (e.g., when piping to `head`) are treated as success
    /// since the consumer has received all the data it needs.
    ///
    /// # Errors
    ///
    /// Returns an error if a search hit cannot be serialized to JSON.
    pub fn format_search_results_jsonl(hits: &[SearchHit]) -> Result<()> {
        let stdout = std::io::stdout();
        let mut writer = stdout.lock();

        for hit in hits {
            let mut value =
                serde_json::to_value(hit).context("failed to serialize SearchHit to JSON")?;
            if let serde_json::Value::Object(ref mut map) = value {
                if let Some(source_value) = map.get("source").cloned() {
                    map.entry("alias".to_string()).or_insert(source_value);
                }
            }
            let json = serde_json::to_string(&value).context("failed to serialize item to JSON")?;
            if let Err(e) = writeln!(writer, "{json}") {
                // Treat broken pipe as success - consumer closed the stream early
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    return Ok(());
                }
                return Err(e).context("failed to write JSON line");
            }
        }

        if let Err(e) = writer.flush() {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                return Ok(());
            }
            return Err(e).context("failed to flush stdout");
        }
        Ok(())
    }
}

/// Build the metadata portion of the JSON response.
fn build_metadata_map(meta: &SearchJsonMetadata<'_>) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    map.insert(
        "query".to_string(),
        serde_json::Value::String(meta.query.to_string()),
    );
    map.insert("page".to_string(), serde_json::Value::from(meta.page));
    map.insert("limit".to_string(), serde_json::Value::from(meta.limit));
    map.insert(
        "totalResults".to_string(),
        serde_json::Value::from(meta.total_results),
    );
    map.insert(
        "total_hits".to_string(),
        serde_json::Value::from(meta.total_results),
    );
    map.insert(
        "totalPages".to_string(),
        serde_json::Value::from(meta.total_pages),
    );
    map.insert(
        "total_pages".to_string(),
        serde_json::Value::from(meta.total_pages),
    );
    map.insert(
        "totalLinesSearched".to_string(),
        serde_json::Value::from(meta.total_lines_searched),
    );
    map.insert(
        "total_lines_searched".to_string(),
        serde_json::Value::from(meta.total_lines_searched),
    );
    let ms = meta.search_time.as_millis();
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
            meta.sources
                .iter()
                .cloned()
                .map(serde_json::Value::from)
                .collect(),
        ),
    );
    map
}

/// Format all hits with percentage scores.
fn format_hits_with_scores(
    hits: &[SearchHit],
    score_precision: u8,
) -> Result<Vec<serde_json::Value>> {
    let raw_max_score = hits.iter().fold(0.0_f32, |acc, hit| acc.max(hit.score));
    let max_score = if raw_max_score > 0.0 {
        raw_max_score
    } else {
        1.0_f32
    };
    let precision = score_precision.min(4);

    hits.iter()
        .map(|hit| format_single_hit(hit, raw_max_score, max_score, precision))
        .collect()
}

/// Format a single hit with rounded score and percentage.
fn format_single_hit(
    hit: &SearchHit,
    raw_max_score: f32,
    max_score: f32,
    precision: u8,
) -> Result<serde_json::Value> {
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
}

fn clamp_percentage(percent: f64) -> u8 {
    percent_to_u8(percent)
}
