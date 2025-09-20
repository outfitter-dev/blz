//! JSON output formatting

use anyhow::Result;
use blz_core::SearchHit;

pub struct JsonFormatter;

impl JsonFormatter {
    /// Format search results as JSON with metadata
    #[allow(clippy::too_many_arguments)]
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
            "totalPages".to_string(),
            serde_json::Value::from(total_pages),
        );
        map.insert(
            "totalLinesSearched".to_string(),
            serde_json::Value::from(total_lines_searched),
        );
        let search_time_ms: u64 = u64::try_from(search_time.as_millis()).unwrap_or(u64::MAX);
        map.insert(
            "searchTimeMs".to_string(),
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
        map.insert("results".to_string(), serde_json::to_value(hits)?);

        if let Some(s) = suggestions {
            if !s.is_empty() {
                map.insert(
                    "suggestions".to_string(),
                    serde_json::Value::Array(s.to_vec()),
                );
            }
        }

        let obj = serde_json::Value::Object(map);
        println!("{}", serde_json::to_string_pretty(&obj)?);
        Ok(())
    }

    /// Format search results as newline-delimited JSON
    pub fn format_search_results_jsonl(hits: &[SearchHit]) -> Result<()> {
        for hit in hits {
            println!("{}", serde_json::to_string(hit)?);
        }
        Ok(())
    }
}
