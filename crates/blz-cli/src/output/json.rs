//! JSON output formatting

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
}
