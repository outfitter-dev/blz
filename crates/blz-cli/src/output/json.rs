//! JSON output formatting

use crate::output::formatter::FormatParams;
use anyhow::Result;
use blz_core::SearchHit;

pub struct JsonFormatter;

impl JsonFormatter {
    /// Format search results as JSON
    pub fn format_search_results(hits: &[SearchHit]) -> Result<()> {
        let json = serde_json::to_string_pretty(hits)?;
        println!("{json}");
        Ok(())
    }

    /// Format search results as newline-delimited JSON
    pub fn format_search_results_ndjson(hits: &[SearchHit]) -> Result<()> {
        for hit in hits {
            println!("{}", serde_json::to_string(hit)?);
        }
        Ok(())
    }

    /// Format search results as a full JSON envelope with metadata
    pub fn format_search_results_full(params: &FormatParams) -> Result<()> {
        use serde_json::json;
        let obj = json!({
            "query": params.query,
            "total_results": params.total_results,
            "search_time_ms": params.search_time.as_millis(),
            "sources": params.sources,
            "hits": params.hits,
        });
        println!("{}", serde_json::to_string_pretty(&obj)?);
        Ok(())
    }
}
