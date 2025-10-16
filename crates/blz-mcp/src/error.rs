//! Error types for BLZ MCP server with MCP error code mapping

use thiserror::Error;

/// Errors that can occur in the MCP server
#[derive(Debug, Error)]
pub enum McpError {
    /// Storage operation failed
    #[error("storage error: {0}")]
    Storage(#[from] blz_core::Error),

    /// Index operation failed
    #[error("index error: {0}")]
    Index(String),

    /// JSON serialization/deserialization error
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Protocol error
    #[error("protocol error: {0}")]
    Protocol(String),

    /// Internal server error
    #[error("internal error: {0}")]
    Internal(String),

    /// Invalid citation format
    #[error("invalid citation: {0}")]
    InvalidCitation(String),

    /// Invalid line padding value
    #[error("invalid line padding: {0} (max: 50)")]
    InvalidPadding(u32),

    /// Source not found
    #[error("source not found: {0}")]
    SourceNotFound(String),
}

impl McpError {
    /// Map error to MCP error code
    pub const fn error_code(&self) -> i32 {
        match self {
            Self::Storage(_) | Self::Index(_) | Self::Internal(_) => -32603, // Internal error
            Self::Json(_) => -32700,                                         // Parse error
            Self::Protocol(_) => -32600,                                     // Invalid request
            Self::InvalidCitation(_) | Self::InvalidPadding(_) | Self::SourceNotFound(_) => {
                -32602 // Invalid params
            },
        }
    }
}

impl From<anyhow::Error> for McpError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

/// Result type alias for MCP operations
pub type McpResult<T> = Result<T, McpError>;
