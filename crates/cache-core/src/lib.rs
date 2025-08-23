pub mod config;
pub mod error;
pub mod fetcher;
pub mod index;
pub mod parser;
pub mod registry;
pub mod storage;
pub mod types;
pub mod profiling;

pub use config::{Config, FollowLinks, ToolConfig};
pub use error::{Error, Result};
pub use fetcher::{Fetcher, FetchResult, FlavorInfo};
pub use index::SearchIndex;
pub use parser::{MarkdownParser, ParseResult};
pub use registry::{Registry, RegistryEntry};
pub use storage::Storage;
pub use types::*;
pub use profiling::{PerformanceMetrics, OperationTimer, ComponentTimings, ResourceMonitor};

/// Check if flavor detection is enabled (not disabled by environment variable)
pub fn is_flavor_detection_enabled() -> bool {
    !is_env_var_enabled("CACHE_DISABLE_FLAVOR_CHECK")
}

/// Check if registry lookup is enabled (not disabled by environment variable)
pub fn is_registry_lookup_enabled() -> bool {
    !is_env_var_enabled("CACHE_DISABLE_REGISTRY")
}

/// Check if an environment variable is set to a truthy value
fn is_env_var_enabled(var_name: &str) -> bool {
    match std::env::var(var_name) {
        Ok(value) => {
            let value = value.to_lowercase();
            matches!(value.as_str(), "1" | "true" | "yes")
        }
        Err(_) => false,
    }
}