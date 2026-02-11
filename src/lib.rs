#![deny(missing_docs)]

//! A log-structured key-value store library with client-server networking.
//!
//! This library provides a persistent key-value store using
//! log-structured file I/O based on the bitcask storage model,
//! with support for pluggable storage engines and TCP networking.

mod client;
mod common;
mod engines;
mod error;
mod server;

pub use client::KvsClient;
pub use common::{Request, Response};
pub use engines::{KvStore, KvsEngine, SledKvsEngine};
pub use error::{KvError, Result};
pub use server::KvsServer;
