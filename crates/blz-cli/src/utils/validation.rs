//! Input validation utilities

use anyhow::Result;

use super::constants::RESERVED_KEYWORDS;

/// Validate an alias to ensure it's not a reserved keyword
pub fn validate_alias(alias: &str) -> Result<()> {
    if RESERVED_KEYWORDS.contains(&alias.to_lowercase().as_str()) {
        return Err(anyhow::anyhow!(
            "Alias '{}' is reserved. Reserved keywords: {}",
            alias,
            RESERVED_KEYWORDS.join(", ")
        ));
    }
    Ok(())
}
