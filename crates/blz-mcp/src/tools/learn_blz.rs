//! Learn tool for returning curated BLZ guidance

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{McpError, McpResult};

/// The JSON payload embedded at compile time
const LEARN_PAYLOAD: &str = include_str!("../../data/learn_blz.json");

/// Parameters for learn-blz tool (empty - no parameters needed)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearnBlzParams {}

/// Output from learn-blz tool
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearnBlzOutput {
    /// The guidance content as structured JSON
    pub content: Value,
}

/// Load and parse the learn payload
#[tracing::instrument]
fn load_learn_payload() -> McpResult<Value> {
    let start = std::time::Instant::now();

    let value: Value = serde_json::from_str(LEARN_PAYLOAD)
        .map_err(|e| McpError::LearnPayloadError(format!("Failed to parse learn_blz.json: {e}")))?;

    let elapsed = start.elapsed();
    tracing::debug!(
        elapsed_micros = elapsed.as_micros(),
        "learn payload loaded and parsed"
    );

    Ok(value)
}

/// Handle learn-blz tool
#[tracing::instrument(skip_all)]
pub async fn handle_learn_blz(
    #[allow(clippy::used_underscore_binding)] _params: LearnBlzParams,
) -> McpResult<LearnBlzOutput> {
    tracing::debug!("loading learn-blz guidance");

    let content = load_learn_payload()?;

    Ok(LearnBlzOutput { content })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_learn_payload_parses() {
        let result = load_learn_payload();
        assert!(result.is_ok());

        if let Ok(value) = result {
            assert!(value.is_object());

            // Verify expected top-level keys exist
            let obj = value.as_object().expect("Payload should be a JSON object");
            assert!(obj.contains_key("description"));
            assert!(obj.contains_key("core_concepts"));
            assert!(obj.contains_key("common_workflows"));
            assert!(obj.contains_key("best_practices"));
            assert!(obj.contains_key("examples"));
            assert!(obj.contains_key("cli_equivalents"));
        }
    }

    #[tokio::test]
    async fn test_handle_learn_blz() {
        let result = handle_learn_blz(LearnBlzParams {}).await;
        assert!(result.is_ok());

        if let Ok(output) = result {
            assert!(output.content.is_object());
        }
    }
}
