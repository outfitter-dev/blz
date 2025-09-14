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
        let mut obj = serde_json::json!({
            "query": query,
            "page": page,
            "limit": limit,
            "totalResults": total_results,
            "totalPages": total_pages,
            "totalLinesSearched": total_lines_searched,
            "searchTimeMs": search_time.as_millis(),
            "sources": sources,
            "results": hits,
        });
        if let Some(s) = suggestions {
            if !s.is_empty() {
                let _ = obj.as_object_mut().expect("json object").insert(
                    "suggestions".to_string(),
                    serde_json::Value::Array(s.to_vec()),
                );
            }
        }
        println!("{}", serde_json::to_string_pretty(&obj)?);
        Ok(())
    }

    /// Format search results as newline-delimited JSON
    pub fn format_search_results_ndjson(hits: &[SearchHit]) -> Result<()> {
        for hit in hits {
            println!("{}", serde_json::to_string(hit)?);
        }
        Ok(())
    }
}
