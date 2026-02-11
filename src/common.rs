use serde::{Deserialize, Serialize};

/// Request sent from client to server.
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    /// Set a key-value pair.
    Set {
        /// The key to set.
        key: String,
        /// The value to associate with the key.
        value: String,
    },
    /// Get the value for a key.
    Get {
        /// The key to look up.
        key: String,
    },
    /// Remove a key.
    Remove {
        /// The key to remove.
        key: String,
    },
}

/// Response sent from server to client.
#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    /// Operation succeeded, optionally with a value.
    Ok(Option<String>),
    /// Operation failed with an error message.
    Err(String),
}
