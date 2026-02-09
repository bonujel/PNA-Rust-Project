use std::io;
use thiserror::Error;

/// Error type for kvs operations.
#[derive(Error, Debug)]
pub enum KvError {
    /// IO error from file operations.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Serialization/deserialization error.
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Key not found in the store.
    #[error("Key not found")]
    KeyNotFound,

    /// Unexpected command type encountered.
    #[error("Unexpected command type")]
    UnexpectedCommandType,

    /// Log file not found for the given generation.
    #[error("Log file not found for generation {0}")]
    LogFileNotFound(u64),
}

/// Result type alias for kvs operations.
pub type Result<T> = std::result::Result<T, KvError>;
