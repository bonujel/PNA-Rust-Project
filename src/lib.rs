#![deny(missing_docs)]

//! A log-structured key-value store library.
//!
//! This library provides a persistent key-value store using
//! log-structured file I/O based on the bitcask storage model.

mod error;
mod kv;

pub use error::{KvError, Result};
pub use kv::KvStore;
