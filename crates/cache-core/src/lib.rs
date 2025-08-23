pub mod config;
pub mod error;
pub mod fetcher;
pub mod index;
pub mod parser;
pub mod storage;
pub mod types;
pub mod profiling;

pub use config::{Config, FollowLinks, ToolConfig};
pub use error::{Error, Result};
pub use fetcher::{Fetcher, FetchResult, FlavorInfo};
pub use index::SearchIndex;
pub use parser::{MarkdownParser, ParseResult};
pub use storage::Storage;
pub use types::*;
pub use profiling::{PerformanceMetrics, OperationTimer, ComponentTimings, ResourceMonitor};