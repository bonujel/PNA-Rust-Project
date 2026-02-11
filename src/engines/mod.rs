use crate::Result;

/// Trait for a key-value storage engine.
///
/// Implementors provide persistent key-value storage with
/// set, get, and remove operations.
pub trait KvsEngine {
    /// Sets the value of a string key to a string.
    ///
    /// If the key already exists, the previous value will be overwritten.
    fn set(&mut self, key: String, value: String) -> Result<()>;

    /// Gets the string value of a given string key.
    ///
    /// Returns `None` if the key does not exist.
    fn get(&mut self, key: String) -> Result<Option<String>>;

    /// Removes a given key.
    ///
    /// Returns an error if the key does not exist.
    fn remove(&mut self, key: String) -> Result<()>;
}

mod kvs;
mod sled_engine;

pub use self::kvs::KvStore;
pub use self::sled_engine::SledKvsEngine;
