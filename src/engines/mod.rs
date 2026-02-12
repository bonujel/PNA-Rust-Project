use crate::Result;

/// Trait for a key-value storage engine.
///
/// Implementors provide persistent key-value storage with
/// set, get, and remove operations.
///
/// Engines must be cloneable (cheaply, via `Arc`) and safe to
/// send across threads, enabling concurrent access from a thread pool.
pub trait KvsEngine: Clone + Send + 'static {
    /// Sets the value of a string key to a string.
    ///
    /// If the key already exists, the previous value will be overwritten.
    fn set(&self, key: String, value: String) -> Result<()>;

    /// Gets the string value of a given string key.
    ///
    /// Returns `None` if the key does not exist.
    fn get(&self, key: String) -> Result<Option<String>>;

    /// Removes a given key.
    ///
    /// Returns an error if the key does not exist.
    fn remove(&self, key: String) -> Result<()>;
}

mod kvs;
mod sled_engine;

pub use self::kvs::KvStore;
pub use self::sled_engine::SledKvsEngine;
