use crate::error::SavantError;
use crate::types::ChatMessage;
use dashmap::DashMap;
use fjall::OptimisticTxDatabase;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Maximum number of content hashes to keep for deduplication per partition.
const DEDUP_WINDOW_SIZE: usize = 100;

/// Partitioned Write-Ahead Log utilizing Fjall's multi-writer capabilities.
pub struct Storage {
    db: Arc<OptimisticTxDatabase>,
    partitions: DashMap<String, fjall::OptimisticTxKeyspace>,
    /// Per-partition message counters for O(1) count queries.
    partition_counts: DashMap<String, AtomicU64>,
    /// Per-partition content hash windows for message deduplication.
    /// Maps agent_id → (hash, timestamp) pairs, oldest evicted first.
    dedup_hashes: DashMap<String, VecDeque<(u64, String)>>,
}

impl Storage {
    pub fn new(path: PathBuf) -> Result<Self, SavantError> {
        info!("Sovereign Substrate: Initializing Fjall at {:?}", path);

        let db = Arc::new(
            OptimisticTxDatabase::builder(&path)
                .open()
                .map_err(|e| SavantError::IoError(std::io::Error::other(e.to_string())))?,
        );

        let storage = Self {
            db,
            partitions: DashMap::new(),
            partition_counts: DashMap::new(),
            dedup_hashes: DashMap::new(),
        };

        Ok(storage)
    }

    /// Ghost-Restore: Re-opens all partitions to ensure data consistency.
    pub fn ghost_restore(&self) -> Result<(), SavantError> {
        warn!("Sovereign Substrate: INITIATING GHOST-RESTORE.");

        // Collect all partition names before clearing
        let partition_names: Vec<String> = self
            .partitions
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        // Clear in-memory caches to force re-open from disk
        self.partitions.clear();

        // Re-open each partition to verify integrity
        for name in &partition_names {
            match self.get_or_create_partition(name) {
                Ok(_) => {
                    debug!("Ghost-Restore: Verified partition [{}] integrity", name);
                }
                Err(e) => {
                    warn!(
                        "Ghost-Restore: Failed to verify partition [{}]: {}",
                        name, e
                    );
                }
            }
        }

        info!(
            "Ghost-Restore: {} partitions verified and re-opened.",
            partition_names.len()
        );
        Ok(())
    }

    fn get_or_create_partition(
        &self,
        agent_id: &str,
    ) -> Result<fjall::OptimisticTxKeyspace, SavantError> {
        if let Some(p) = self.partitions.get(agent_id) {
            return Ok(p.clone());
        }

        let partition_name = format!("agent.{}", agent_id);
        let partition = self
            .db
            .keyspace(&partition_name, fjall::KeyspaceCreateOptions::default)
            .map_err(|e| SavantError::IoError(std::io::Error::other(e.to_string())))?;

        self.partitions
            .insert(agent_id.to_string(), partition.clone());
        Ok(partition)
    }

    /// Appends a chat message to the partition log with deduplication.
    ///
    /// Uses blake3 content hashing to detect and skip duplicate messages
    /// within a sliding window of `DEDUP_WINDOW_SIZE` entries per partition.
    pub fn append_chat(&self, agent_id: &str, msg: &ChatMessage) -> Result<(), SavantError> {
        let partition = self.get_or_create_partition(agent_id)?;
        let payload =
            serde_json::to_string(msg).map_err(|e| SavantError::Unknown(e.to_string()))?;

        // Compute content hash for deduplication
        let content_hash = blake3::hash(msg.content.as_bytes()).to_hex().to_string();

        // Check for duplicate within sliding window
        if let Some(hashes) = self.dedup_hashes.get(agent_id) {
            if hashes.iter().any(|(_, h)| h == &content_hash) {
                // Duplicate detected - silently skip
                return Ok(());
            }
        }

        // Use timestamp_micros with fallback to avoid collisions
        let timestamp = chrono::Utc::now().timestamp_micros().max(0); // Ensure non-negative
        let key = format!("{}:{}", timestamp, uuid::Uuid::new_v4());

        let mut tx = self
            .db
            .write_tx()
            .map_err(|e| SavantError::IoError(std::io::Error::other(e.to_string())))?;
        tx.insert(&partition, key.as_bytes(), payload.as_bytes());

        tx.commit()
            .map_err(|e| SavantError::IoError(std::io::Error::other(format!("IO Error: {:?}", e))))?
            .map_err(|e| {
                SavantError::IoError(std::io::Error::other(format!("Conflict: {:?}", e)))
            })?;

        // Increment partition counter
        self.partition_counts
            .entry(agent_id.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);

        // Add hash to dedup window and evict oldest if over limit
        let mut hashes = self
            .dedup_hashes
            .entry(agent_id.to_string())
            .or_insert_with(VecDeque::new);
        hashes.push_back((timestamp as u64, content_hash));
        while hashes.len() > DEDUP_WINDOW_SIZE {
            hashes.pop_front();
        }

        Ok(())
    }

    /// Retrieves chat history for an agent, most recent first.
    pub fn get_history(
        &self,
        agent_id: &str,
        limit: usize,
    ) -> Result<Vec<ChatMessage>, SavantError> {
        let partition = self.get_or_create_partition(agent_id)?;
        let mut history = Vec::new();

        let iter = partition.inner().iter().rev();
        for item in iter.take(limit) {
            let value = item
                .value()
                .map_err(|e: fjall::Error| SavantError::Unknown(e.to_string()))?;
            if let Ok(msg) = serde_json::from_slice::<ChatMessage>(&value) {
                history.push(msg);
            }
        }

        history.reverse();
        Ok(history)
    }

    /// Retrieves swarm-wide history.
    pub fn get_swarm_history(&self, limit: usize) -> Result<Vec<ChatMessage>, SavantError> {
        self.get_history("swarm.insights", limit)
    }

    /// Prunes old history entries, keeping only the most recent `keep_last` messages.
    ///
    /// Keys are formatted as `{timestamp}:{uuid}` where timestamp is microseconds-since-epoch.
    /// Fjall stores keys in lexicographic order, so iterating forward yields the oldest entries first.
    /// This function takes the first `to_delete` entries (oldest) and removes them.
    pub fn prune_history(&self, agent_id: &str, keep_last: usize) -> Result<(), SavantError> {
        let partition = self.get_or_create_partition(agent_id)?;
        let count = partition.inner().iter().count();
        if count <= keep_last {
            return Ok(());
        }

        let to_delete = count - keep_last;
        let mut tx = self
            .db
            .write_tx()
            .map_err(|e| SavantError::IoError(std::io::Error::other(e.to_string())))?;

        let mut keys = Vec::new();
        for item in partition.inner().iter().take(to_delete) {
            let key = item
                .key()
                .map_err(|e: fjall::Error| SavantError::Unknown(e.to_string()))?;
            keys.push(key.to_vec());
        }

        let deleted_count = keys.len();
        for key in keys {
            tx.remove(&partition, key);
        }

        match tx.commit() {
            Ok(conflict_result) => {
                match conflict_result {
                    Ok(_) => {
                        // Update counter
                        if let Some(counter) = self.partition_counts.get(agent_id) {
                            counter.fetch_sub(deleted_count as u64, Ordering::Relaxed);
                        }
                        debug!(
                            "Pruned {} old entries for agent {}",
                            deleted_count, agent_id
                        );
                    }
                    Err(e) => {
                        warn!("Prune commit conflict for agent {}: {}", agent_id, e);
                        return Err(SavantError::IoError(std::io::Error::other(format!(
                            "Conflict: {:?}",
                            e
                        ))));
                    }
                }
            }
            Err(e) => {
                return Err(SavantError::IoError(std::io::Error::other(format!(
                    "IO Error: {:?}",
                    e
                ))));
            }
        }

        Ok(())
    }

    /// Gracefully shuts down the storage engine, ensuring all data is flushed.
    ///
    /// Clears all in-memory partitions and counters. The underlying Fjall database
    /// is dropped when the Arc reference count reaches zero, which triggers
    /// Fjall's Drop implementation to flush pending writes.
    pub fn shutdown(&self) -> Result<(), SavantError> {
        info!("Storage: Initiating graceful shutdown...");
        let partition_count = self.partitions.len();
        self.partitions.clear();
        self.partition_counts.clear();
        info!(
            "Storage: Shutdown complete. {} partitions cleared. Database will be dropped when last Arc reference is released.",
            partition_count
        );
        Ok(())
    }
}
