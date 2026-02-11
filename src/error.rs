use std::io;
use std::string::FromUtf8Error;
use thiserror::Error;

/// Error type for kvs operations.
#[derive(Error, Debug)]
pub enum KvError {
    /// IO error from file or network operations.
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

    /// Sled database error.
    #[error("sled error: {0}")]
    Sled(#[from] sled::Error),

    /// UTF-8 conversion error.
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] FromUtf8Error),

    /// Error message from the server.
    #[error("{0}")]
    StringError(String),
}

/// Result type alias for kvs operations.
pub type Result<T> = std::result::Result<T, KvError>;
