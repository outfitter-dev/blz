use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Index error: {0}")]
    Index(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;