use fjall::{OptimisticTxDatabase, OptimisticTxKeyspace};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info};

use crate::error::SavantError;

/// Unified Fjall storage engine.
///
/// This engine wraps an `OptimisticTxDatabase` and provides high-level
/// methods for keyspace management, transactions, and data operations.
/// Both `core::db::Storage` and `memory::LsmStorageEngine` can use this
/// to eliminate duplication and centralize Fjall configuration.
#[derive(Clone)]
pub struct FjallEngine {
    db: Arc<OptimisticTxDatabase>,
    #[allow(dead_code)]
    path: PathBuf,
}

impl FjallEngine {
    /// Initializes a new Fjall engine at the given storage path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Arc<Self>, SavantError> {
        let path = path.as_ref().to_path_buf();
        info!("[FjallEngine] Initializing database at {:?}", path);

        let db = Arc::new(
            OptimisticTxDatabase::builder(&path)
                .open()
                .map_err(|e| SavantError::IoError(std::io::Error::other(e.to_string())))?,
        );

        Ok(Arc::new(Self { db, path }))
    }

    /// Returns a reference to the underlying database.
    pub fn db(&self) -> &OptimisticTxDatabase {
        &self.db
    }

    /// Returns the underlying database path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Gets or creates a keyspace (namespace) within the database.
    pub fn get_or_create_keyspace(&self, name: &str) -> Result<OptimisticTxKeyspace, SavantError> {
        self.db
            .keyspace(name, fjall::KeyspaceCreateOptions::default)
            .map_err(|e| SavantError::IoError(std::io::Error::other(e.to_string())))
    }

    /// Inserts a key-value pair into the specified keyspace.
    /// Transaction is committed immediately.
    pub fn insert(&self, keyspace: &str, key: &[u8], value: &[u8]) -> Result<(), SavantError> {
        let ks = self.get_or_create_keyspace(keyspace)?;
        let mut tx = self
            .db
            .write_tx()
            .map_err(|e| SavantError::IoError(std::io::Error::other(e.to_string())))?;
        tx.insert(&ks, key, value);
        tx.commit()
            .map_err(|e| SavantError::IoError(std::io::Error::other(e.to_string())))?
            .map_err(|e| {
                SavantError::IoError(std::io::Error::other(format!("Conflict: {:?}", e)))
            })?;
        Ok(())
    }

    /// Removes a key from the specified keyspace.
    pub fn remove(&self, keyspace: &str, key: &[u8]) -> Result<(), SavantError> {
        let ks = self.get_or_create_keyspace(keyspace)?;
        let mut tx = self
            .db
            .write_tx()
            .map_err(|e| SavantError::IoError(std::io::Error::other(e.to_string())))?;
        tx.remove(&ks, key);
        tx.commit()
            .map_err(|e| SavantError::IoError(std::io::Error::other(e.to_string())))?
            .map_err(|e| {
                SavantError::IoError(std::io::Error::other(format!("Conflict: {:?}", e)))
            })?;
        Ok(())
    }

    /// Retrieves a value by key from the specified keyspace.
    /// Returns `Ok(None)` if key not found.
    pub fn get(&self, keyspace: &str, key: &[u8]) -> Result<Option<Vec<u8>>, SavantError> {
        let ks = self.get_or_create_keyspace(keyspace)?;
        ks.get(key)
            .map_err(|e| SavantError::IoError(std::io::Error::other(e.to_string())))
            .map(|opt| opt.map(|v| v.to_vec()))
    }

    /// Flushes pending writes.
    ///
    /// Note: Fjall auto-persists data on drop and uses background
    /// fsync. This method is a no-op for compatibility but data
    /// durability is ensured by Fjall's WAL.
    pub fn flush(&self) -> Result<(), SavantError> {
        debug!("[FjallEngine] Flush requested (no-op, Fjall auto-persists)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_basic_ops() {
        let temp_dir = std::env::temp_dir().join("savant_fjall_test");
        let _ = std::fs::create_dir_all(&temp_dir);
        let engine = FjallEngine::new(&temp_dir).unwrap();

        // Insert and retrieve
        engine.insert("test", b"key1", b"value1").unwrap();
        let val = engine.get("test", b"key1").unwrap().unwrap();
        assert_eq!(&val, b"value1");

        // Remove
        engine.remove("test", b"key1").unwrap();
        let val = engine.get("test", b"key1").unwrap();
        assert!(val.is_none());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
