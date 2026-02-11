use sled::Db;

use super::KvsEngine;
use crate::{KvError, Result};

/// A key-value store backed by the `sled` embedded database.
pub struct SledKvsEngine {
    db: Db,
}

impl SledKvsEngine {
    /// Creates a new `SledKvsEngine` from an already-opened sled `Db`.
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

impl KvsEngine for SledKvsEngine {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        self.db.insert(key.as_bytes(), value.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    fn get(&mut self, key: String) -> Result<Option<String>> {
        Ok(self
            .db
            .get(key.as_bytes())?
            .map(|ivec| String::from_utf8(ivec.to_vec()))
            .transpose()?)
    }

    fn remove(&mut self, key: String) -> Result<()> {
        self.db
            .remove(key.as_bytes())?
            .ok_or(KvError::KeyNotFound)?;
        self.db.flush()?;
        Ok(())
    }
}
