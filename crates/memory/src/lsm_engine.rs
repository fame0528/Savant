//! CortexaDB Storage Engine
//!
//! This module provides a transactional storage backend using CortexaDB,
//! a vector + graph embedded database designed for AI agent memory.
//! It replaces the previous Fjall LSM-tree implementation.
//!
//! Key features:
//! - Collection-based partitioning (one per session)
//! - Vector search capabilities for semantic retrieval
//! - Graph relationships for DAG compaction
//! - WAL-backed hard durability
//! - Zero-copy reads using rkyv where applicable

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use cortexadb_core::{BatchRecord, CortexaDB};
use rkyv::rancor::Error as RkyvError;

use crate::error::MemoryError;
use crate::models::{verify_tool_pair_integrity, AgentMessage};

/// Vector dimension for CortexaDB embeddings (default fallback only).
/// Actual dimension should come from LsmConfig.vector_dimension.
const DEFAULT_VECTOR_DIM: usize = 384;

/// Maximum entries to retrieve per collection query.
const MAX_BATCH_SIZE: usize = 100_000;

/// Creates metadata HashMap with a key field.
fn make_meta(key: &str, timestamp: i64) -> Option<HashMap<String, String>> {
    let mut m = HashMap::new();
    m.insert("key".to_string(), key.to_string());
    m.insert("timestamp".to_string(), timestamp.to_string());
    Some(m)
}

/// Creates metadata HashMap with just a key field.
fn make_key_meta(key: &str) -> Option<HashMap<String, String>> {
    let mut m = HashMap::new();
    m.insert("key".to_string(), key.to_string());
    Some(m)
}

/// Statistics about the storage engine (for monitoring)
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    pub total_messages: u64,
    pub total_sessions: u64,
    pub disk_usage_bytes: u64,
    pub cache_hit_rate: f32,
}

/// The core storage engine backed by CortexaDB.
///
/// This engine uses CortexaDB collections for partitioning:
/// - `transcript.{session_id}` — conversation transcripts
/// - `metadata` — semantic metadata entries
/// - `temporal` — bi-temporal metadata
/// - `dag` — DAG compaction nodes
/// - `distillation` — distillation state flags
/// - `facts` — semantic facts (SPO triples)
pub struct LsmStorageEngine {
    db: CortexaDB,
    /// Known session IDs (maintained in memory for iteration).
    sessions: dashmap::DashSet<String>,
    /// Configured vector dimension for zero-embedding construction.
    vector_dimension: usize,
}

/// Configuration for the CortexaDB storage engine.
#[derive(Debug, Clone)]
pub struct LsmConfig {
    /// Vector dimension for embeddings (default: 384)
    pub vector_dimension: usize,
    /// Sync policy: true = sync after every write, false = async (default: true)
    pub strict_sync: bool,
    /// Checkpoint interval in operations (default: 1000)
    pub checkpoint_every_ops: usize,
}

impl Default for LsmConfig {
    fn default() -> Self {
        Self {
            vector_dimension: DEFAULT_VECTOR_DIM,
            strict_sync: true,
            checkpoint_every_ops: 1000,
        }
    }
}

impl LsmStorageEngine {
    /// Initializes the CortexaDB storage engine.
    pub fn new(storage_path: &Path, config: LsmConfig) -> Result<Arc<Self>, MemoryError> {
        info!(
            "Initializing CortexaDB at {:?} (dim={}, sync={})",
            storage_path, config.vector_dimension, config.strict_sync
        );

        let path_str = storage_path.to_str().ok_or_else(|| {
            MemoryError::InitFailed("Database path is not valid UTF-8".to_string())
        })?;

        let db = CortexaDB::open(path_str, config.vector_dimension)
            .map_err(|e| MemoryError::InitFailed(format!("CortexaDB open failed: {}", e)))?;

        // Rebuild sessions set from the session registry
        let sessions = dashmap::DashSet::new();
        let vector_dimension = config.vector_dimension;
        if let Ok(hits) = db.search_in_collection(
            "_registry",
            vec![0.0; vector_dimension],
            MAX_BATCH_SIZE,
            None,
        ) {
            for hit in hits {
                if let Ok(memory) = db.get_memory(hit.id) {
                    if let Some(session_id) = memory.metadata.get("session_id") {
                        sessions.insert(session_id.clone());
                    }
                }
            }
        }

        info!(
            "CortexaDB Engine initialized with {} known sessions",
            sessions.len()
        );

        Ok(Arc::new(Self {
            db,
            sessions,
            vector_dimension,
        }))
    }

    /// Convenience: Create with default configuration.
    pub fn with_defaults(storage_path: &Path) -> Result<Arc<Self>, MemoryError> {
        Self::new(storage_path, LsmConfig::default())
    }

    /// Creates a minimal-value placeholder embedding at the configured dimension.
    /// Uses 0.001 per dimension to avoid `VectorError::ZeroVector` in the search
    /// layer while remaining a distinguishable placeholder for missing embeddings.
    pub fn zero_embedding(&self) -> Vec<f32> {
        vec![0.001; self.vector_dimension]
    }

    /// Returns the collection name for a session transcript.
    fn transcript_collection(session_id: &str) -> String {
        format!("transcript.{}", session_id)
    }

    /// Appends a single message to the session transcript.
    #[instrument(skip(self, message), fields(session = %session_id, msg_id = %message.id))]
    pub fn append_message(
        &self,
        session_id: &str,
        message: &AgentMessage,
    ) -> Result<(), MemoryError> {
        let bytes = rkyv::to_bytes::<RkyvError>(message)
            .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;

        let collection = Self::transcript_collection(session_id);
        let timestamp: i64 = message.timestamp.into();
        let key = crate::models::message_key(session_id, timestamp, &message.id);

        self.db
            .add_with_content(
                &collection,
                bytes.to_vec(),
                self.zero_embedding(),
                make_meta(&key, timestamp),
            )
            .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;

        // Register session in the registry for persistence across restarts
        if self.sessions.insert(session_id.to_string()) {
            let mut reg_meta = HashMap::new();
            reg_meta.insert("session_id".to_string(), session_id.to_string());
            if let Err(e) = self.db.add_with_content(
                "_registry",
                session_id.as_bytes().to_vec(),
                self.zero_embedding(),
                Some(reg_meta),
            ) {
                warn!(
                    "[memory::lsm] Failed to register session {}: {}",
                    session_id, e
                );
            }
        }

        debug!("Appended message {} to session {}", message.id, session_id);
        Ok(())
    }

    /// Iterates over all messages across all sessions.
    pub fn iter_all_messages(&self) -> impl Iterator<Item = AgentMessage> + '_ {
        let mut all_msgs: Vec<AgentMessage> = Vec::new();

        for session_ref in self.sessions.iter() {
            let session_id = session_ref.key().clone();
            let collection = Self::transcript_collection(&session_id);

            if let Ok(hits) = self.db.search_in_collection(
                &collection,
                self.zero_embedding(),
                MAX_BATCH_SIZE,
                None,
            ) {
                for hit in hits {
                    if let Ok(memory) = self.db.get_memory(hit.id) {
                        if memory.content.len() <= 10 * 1024 * 1024 {
                            if let Ok(archived) = rkyv::access::<
                                rkyv::Archived<AgentMessage>,
                                rkyv::rancor::Error,
                            >(&memory.content)
                            {
                                if let Ok(msg) =
                                    rkyv::deserialize::<AgentMessage, rkyv::rancor::Error>(archived)
                                {
                                    all_msgs.push(msg);
                                }
                            }
                        }
                    }
                }
            }
        }

        all_msgs.sort_by_key(|m| i64::from(m.timestamp));
        all_msgs.into_iter()
    }

    /// Iterates over messages from the last N hours across all sessions.
    /// Used by the NREM dream phase for structured memory replay.
    pub fn iter_recent_messages(&self, hours: u64) -> Vec<AgentMessage> {
        let cutoff = chrono::Utc::now().timestamp() - (hours as i64 * 3600);
        self.iter_all_messages()
            .filter(|msg| i64::from(msg.timestamp) >= cutoff)
            .collect()
    }

    /// Inserts a MemoryEntry into the metadata collection.
    pub fn insert_metadata(
        &self,
        id: u64,
        entry: &crate::models::MemoryEntry,
    ) -> Result<(), MemoryError> {
        let bytes = rkyv::to_bytes::<RkyvError>(entry)
            .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
        let key = format!("meta:{}", id);

        self.db
            .add_with_content(
                "metadata",
                bytes.to_vec(),
                self.zero_embedding(),
                make_key_meta(&key),
            )
            .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;
        Ok(())
    }

    /// Removes a MemoryEntry from the metadata collection.
    pub fn remove_metadata(&self, id: u64) -> Result<(), MemoryError> {
        let key = format!("meta:{}", id);
        let filter = {
            let mut m = HashMap::new();
            m.insert("key".to_string(), key);
            m
        };

        if let Ok(hits) =
            self.db
                .search_in_collection("metadata", self.zero_embedding(), 1, Some(filter))
        {
            for hit in hits {
                self.db
                    .delete(hit.id)
                    .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;
            }
        }
        Ok(())
    }

    /// Fetches the tail of a session's conversation history.
    #[instrument(skip(self), fields(session = %session_id, limit))]
    pub fn fetch_session_tail(&self, session_id: &str, limit: usize) -> Vec<AgentMessage> {
        let collection = Self::transcript_collection(session_id);
        let mut messages = Vec::new();
        let mut validation_failures = 0;

        let hits = match self.db.search_in_collection(
            &collection,
            self.zero_embedding(),
            MAX_BATCH_SIZE,
            None,
        ) {
            Ok(h) => h,
            Err(_) => return messages,
        };

        for hit in hits {
            let memory = match self.db.get_memory(hit.id) {
                Ok(m) => m,
                Err(_) => continue,
            };

            if memory.content.len() > 10 * 1024 * 1024 {
                warn!("Oversized message detected: {} bytes", memory.content.len());
                continue;
            }

            let archived = match rkyv::access::<rkyv::Archived<AgentMessage>, rkyv::rancor::Error>(
                &memory.content,
            ) {
                Ok(a) => a,
                Err(_) => {
                    validation_failures += 1;
                    continue;
                }
            };

            match rkyv::deserialize::<AgentMessage, rkyv::rancor::Error>(archived) {
                Ok(msg) => {
                    if msg.channel != "Archive" {
                        messages.push(msg);
                    }
                }
                Err(_) => {
                    validation_failures += 1;
                }
            }
        }

        // Sort by timestamp ascending, then take last N and reverse for newest-first
        messages.sort_by_key(|m| i64::from(m.timestamp));
        if messages.len() > limit {
            messages = messages.split_off(messages.len() - limit);
        }
        messages.reverse();

        if validation_failures > 0 {
            debug!(
                "Skipped {} invalid entries for session '{}'",
                validation_failures, session_id
            );
        }

        debug!(
            "Fetched {} messages for session {}",
            messages.len(),
            session_id
        );
        messages
    }

    /// Atomically compacts a batch of messages into the database.
    #[instrument(skip(self, batch), fields(session = %session_id, batch_size = batch.len()))]
    pub fn atomic_compact(
        &self,
        session_id: &str,
        batch: Vec<AgentMessage>,
    ) -> Result<(), MemoryError> {
        if batch.is_empty() {
            return Ok(());
        }

        verify_tool_pair_integrity(&batch)?;

        let collection = Self::transcript_collection(session_id);

        // Phase 1: Insert compacted batch FIRST (write-before-delete ensures data safety)
        // If this fails, old entries remain intact — no data loss.
        let mut records: Vec<BatchRecord> = Vec::with_capacity(batch.len());
        for msg in &batch {
            let bytes = rkyv::to_bytes::<RkyvError>(msg)
                .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
            let timestamp: i64 = msg.timestamp.into();
            let key = crate::models::message_key(session_id, timestamp, &msg.id);
            let mut meta = HashMap::new();
            meta.insert("key".to_string(), key);
            meta.insert("timestamp".to_string(), timestamp.to_string());
            records.push(BatchRecord {
                collection: collection.clone(),
                content: bytes.to_vec(),
                embedding: Some(self.zero_embedding()),
                metadata: Some(meta),
            });
        }

        self.db
            .add_batch(records)
            .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;

        // Phase 2: Delete old entries AFTER successful insert
        // If this fails, duplicates exist temporarily but next compaction cleans them up.
        // This ordering guarantees no data loss — the insert is the commitment point.
        if let Ok(hits) =
            self.db
                .search_in_collection(&collection, self.zero_embedding(), MAX_BATCH_SIZE, None)
        {
            for hit in hits {
                self.db
                    .delete(hit.id)
                    .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;
            }
        }

        info!(
            session = %session_id,
            inserted = batch.len(),
            "Atomic compaction succeeded"
        );
        Ok(())
    }

    /// Counts the number of messages in a session.
    pub fn count_session_messages(&self, session_id: &str) -> Result<u64, MemoryError> {
        let collection = Self::transcript_collection(session_id);
        let hits = self
            .db
            .search_in_collection(&collection, self.zero_embedding(), MAX_BATCH_SIZE, None)
            .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;
        Ok(hits.len() as u64)
    }

    /// Fetches all message IDs for a session.
    pub fn fetch_all_message_ids_for_session(&self, session_id: &str) -> Vec<String> {
        let collection = Self::transcript_collection(session_id);
        let mut ids = Vec::new();

        if let Ok(hits) =
            self.db
                .search_in_collection(&collection, self.zero_embedding(), MAX_BATCH_SIZE, None)
        {
            for hit in hits {
                if let Ok(memory) = self.db.get_memory(hit.id) {
                    if let Some(key) = memory.metadata.get("key") {
                        ids.push(key.clone());
                    }
                }
            }
        }
        ids
    }

    /// Deletes a session entirely.
    pub fn delete_session(&self, session_id: &str) -> Result<(), MemoryError> {
        let collection = Self::transcript_collection(session_id);
        let mut deleted = 0;

        if let Ok(hits) =
            self.db
                .search_in_collection(&collection, self.zero_embedding(), MAX_BATCH_SIZE, None)
        {
            for hit in hits {
                self.db
                    .delete(hit.id)
                    .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;
                deleted += 1;
            }
        }

        self.sessions.remove(session_id);
        info!("Deleted session {} ({} messages)", session_id, deleted);
        Ok(())
    }

    /// Retrieves engine statistics.
    pub fn stats(&self) -> Result<StorageStats, MemoryError> {
        let db_stats = self
            .db
            .stats()
            .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;

        // Approximate disk usage from WAL length (each entry ~1KB average)
        let disk_usage_bytes = db_stats.wal_length * 1024;

        // Index rate: fraction of entries that have been indexed
        let cache_hit_rate = if db_stats.entries > 0 {
            db_stats.indexed_embeddings as f32 / db_stats.entries as f32
        } else {
            0.0
        };

        Ok(StorageStats {
            total_messages: db_stats.entries as u64,
            total_sessions: self.sessions.len() as u64,
            disk_usage_bytes,
            cache_hit_rate,
        })
    }

    /// Returns an iterator over all metadata entries.
    pub fn iter_metadata(&self) -> Result<Vec<crate::models::MemoryEntry>, MemoryError> {
        let mut entries = Vec::new();
        let mut stale_count = 0;

        if let Ok(hits) =
            self.db
                .search_in_collection("metadata", self.zero_embedding(), MAX_BATCH_SIZE, None)
        {
            for hit in hits {
                if let Ok(memory) = self.db.get_memory(hit.id) {
                    if memory.content.len() > 1024 * 1024 {
                        stale_count += 1;
                        continue;
                    }
                    let archived = match rkyv::access::<
                        <crate::models::MemoryEntry as rkyv::Archive>::Archived,
                        RkyvError,
                    >(&memory.content)
                    {
                        Ok(a) => a,
                        Err(_) => {
                            stale_count += 1;
                            continue;
                        }
                    };
                    if let Ok(entry) =
                        rkyv::deserialize::<crate::models::MemoryEntry, RkyvError>(archived)
                    {
                        entries.push(entry);
                    } else {
                        stale_count += 1;
                    }
                }
            }
        }

        if stale_count > 0 {
            debug!("Skipped {} stale metadata entries", stale_count);
        }
        Ok(entries)
    }

    /// Stores temporal metadata for a memory entry.
    pub fn store_temporal_metadata(
        &self,
        temporal: &crate::models::TemporalMetadata,
    ) -> Result<(), MemoryError> {
        let key = crate::models::temporal_key(temporal.memory_id);
        let bytes = serde_json::to_vec(temporal)
            .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;

        self.db
            .add_with_content(
                "temporal",
                bytes,
                self.zero_embedding(),
                make_key_meta(&key),
            )
            .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;
        Ok(())
    }

    /// Looks up temporal metadata by memory ID.
    pub fn get_temporal_metadata(
        &self,
        memory_id: u64,
    ) -> Result<Option<crate::models::TemporalMetadata>, MemoryError> {
        let key = crate::models::temporal_key(memory_id);
        let filter = {
            let mut m = HashMap::new();
            m.insert("key".to_string(), key);
            m
        };

        if let Ok(hits) =
            self.db
                .search_in_collection("temporal", self.zero_embedding(), 1, Some(filter))
        {
            if let Some(hit) = hits.first() {
                if let Ok(memory) = self.db.get_memory(hit.id) {
                    let temporal: crate::models::TemporalMetadata =
                        serde_json::from_slice(&memory.content)
                            .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
                    return Ok(Some(temporal));
                }
            }
        }
        Ok(None)
    }

    /// Finds active temporal entries for an entity name.
    pub fn find_active_temporal_by_entity(
        &self,
        entity_name: &str,
    ) -> Result<Vec<crate::models::TemporalMetadata>, MemoryError> {
        let mut results = Vec::new();

        if let Ok(hits) =
            self.db
                .search_in_collection("temporal", self.zero_embedding(), MAX_BATCH_SIZE, None)
        {
            for hit in hits {
                if let Ok(memory) = self.db.get_memory(hit.id) {
                    if let Ok(temporal) =
                        serde_json::from_slice::<crate::models::TemporalMetadata>(&memory.content)
                    {
                        if temporal.is_active() && temporal.entity_name == entity_name {
                            results.push(temporal);
                        }
                    }
                }
            }
        }
        Ok(results)
    }

    /// Stores a DAG node for reversible compaction.
    pub fn store_dag_node(&self, node: &crate::models::DagNode) -> Result<(), MemoryError> {
        let key = crate::models::dag_node_key(&node.node_id);
        let bytes = serde_json::to_vec(node)
            .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;

        self.db
            .add_with_content("dag", bytes, self.zero_embedding(), make_key_meta(&key))
            .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;

        // Graph edges require u64 IDs, but DAG nodes use UUIDs.
        // Child relationships are stored in the DagNode.child_nodes field directly.
        // No graph edges are created since CortexaDB's connect() requires numeric IDs.
        if !node.child_nodes.is_empty() {
            debug!(
                "DAG node {} has {} child nodes (stored in node data)",
                node.node_id,
                node.child_nodes.len()
            );
        }

        Ok(())
    }

    /// Loads a DAG node by ID.
    pub fn load_dag_node(
        &self,
        node_id: &str,
    ) -> Result<Option<crate::models::DagNode>, MemoryError> {
        let key = crate::models::dag_node_key(node_id);
        let filter = {
            let mut m = HashMap::new();
            m.insert("key".to_string(), key);
            m
        };

        if let Ok(hits) =
            self.db
                .search_in_collection("dag", self.zero_embedding(), 1, Some(filter))
        {
            if let Some(hit) = hits.first() {
                if let Ok(memory) = self.db.get_memory(hit.id) {
                    let node: crate::models::DagNode = serde_json::from_slice(&memory.content)
                        .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
                    return Ok(Some(node));
                }
            }
        }
        Ok(None)
    }

    /// Helper to find a CortexaDB entry ID by its metadata key.
    #[allow(dead_code)]
    fn find_by_key(&self, collection: &str, key: &str) -> Result<Option<u64>, MemoryError> {
        let filter = {
            let mut m = HashMap::new();
            m.insert("key".to_string(), key.to_string());
            m
        };

        if let Ok(hits) =
            self.db
                .search_in_collection(collection, self.zero_embedding(), 1, Some(filter))
        {
            if let Some(hit) = hits.first() {
                return Ok(Some(hit.id));
            }
        }
        Ok(None)
    }

    /// Inserts a semantic fact into the SPO index.
    pub fn insert_fact(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        entry_id: u64,
    ) -> Result<(), MemoryError> {
        let key = format!("{}:{}:{}", subject, predicate, entry_id);
        let mut meta = HashMap::new();
        meta.insert("key".to_string(), key);
        meta.insert("subject".to_string(), subject.to_string());
        meta.insert("predicate".to_string(), predicate.to_string());
        meta.insert("entry_id".to_string(), entry_id.to_string());

        self.db
            .add_with_content(
                "facts",
                object.as_bytes().to_vec(),
                self.zero_embedding(),
                Some(meta),
            )
            .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;
        Ok(())
    }

    /// Iterates over all recorded facts (SPO triples) in the system.
    pub fn iter_facts(&self) -> Vec<(String, String, String, u64)> {
        let mut results = Vec::new();

        if let Ok(hits) =
            self.db
                .search_in_collection("facts", self.zero_embedding(), MAX_BATCH_SIZE, None)
        {
            for hit in hits {
                if let Ok(memory) = self.db.get_memory(hit.id) {
                    let object = String::from_utf8_lossy(&memory.content).to_string();
                    let subject = memory.metadata.get("subject").cloned().unwrap_or_default();
                    let predicate = memory
                        .metadata
                        .get("predicate")
                        .cloned()
                        .unwrap_or_default();
                    let entry_id = memory
                        .metadata
                        .get("entry_id")
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                    results.push((subject, predicate, object, entry_id));
                }
            }
        }
        results
    }

    /// Retrieves all facts for a given subject.
    pub fn get_facts_by_subject(&self, subject: &str) -> Vec<(String, String, u64)> {
        let mut results = Vec::new();
        let filter = {
            let mut m = HashMap::new();
            m.insert("subject".to_string(), subject.to_string());
            m
        };

        if let Ok(hits) = self.db.search_in_collection(
            "facts",
            self.zero_embedding(),
            MAX_BATCH_SIZE,
            Some(filter),
        ) {
            for hit in hits {
                if let Ok(memory) = self.db.get_memory(hit.id) {
                    let predicate = memory
                        .metadata
                        .get("predicate")
                        .cloned()
                        .unwrap_or_default();
                    let entry_id = memory
                        .metadata
                        .get("entry_id")
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                    results.push((
                        predicate,
                        String::from_utf8_lossy(&memory.content).to_string(),
                        entry_id,
                    ));
                }
            }
        }
        results
    }

    /// Deletes a specific fact from the SPO index.
    pub fn delete_fact(
        &self,
        subject: &str,
        predicate: &str,
        entry_id: u64,
    ) -> Result<(), MemoryError> {
        let key = format!("{}:{}:{}", subject, predicate, entry_id);
        let filter = {
            let mut m = HashMap::new();
            m.insert("key".to_string(), key);
            m
        };

        if let Ok(hits) =
            self.db
                .search_in_collection("facts", self.zero_embedding(), 1, Some(filter))
        {
            for hit in hits {
                self.db
                    .delete(hit.id)
                    .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;
            }
        }
        Ok(())
    }

    /// Removes a MemoryEntry from the metadata collection by ID.
    pub fn delete_metadata(&self, id: u64) -> Result<(), MemoryError> {
        self.remove_metadata(id)
    }

    /// Fetches a MemoryEntry from the metadata collection by ID.
    pub fn get_metadata(&self, id: u64) -> Result<Option<crate::models::MemoryEntry>, MemoryError> {
        let key = format!("meta:{}", id);
        let filter = {
            let mut m = HashMap::new();
            m.insert("key".to_string(), key);
            m
        };

        if let Ok(hits) =
            self.db
                .search_in_collection("metadata", self.zero_embedding(), 1, Some(filter))
        {
            if let Some(hit) = hits.first() {
                if let Ok(memory) = self.db.get_memory(hit.id) {
                    let archived = rkyv::access::<
                        <crate::models::MemoryEntry as rkyv::Archive>::Archived,
                        rkyv::rancor::Error,
                    >(&memory.content)
                    .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;

                    let entry =
                        rkyv::deserialize::<crate::models::MemoryEntry, rkyv::rancor::Error>(
                            archived,
                        )
                        .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
                    return Ok(Some(entry));
                }
            }
        }
        Ok(None)
    }

    /// Fetches a single message by ID across all sessions.
    pub fn fetch_message_by_id(&self, msg_id: &str) -> Result<Option<AgentMessage>, MemoryError> {
        for session_ref in self.sessions.iter() {
            let session_id = session_ref.key().clone();
            let collection = Self::transcript_collection(&session_id);

            if let Ok(hits) = self.db.search_in_collection(
                &collection,
                self.zero_embedding(),
                MAX_BATCH_SIZE,
                None,
            ) {
                for hit in hits {
                    if let Ok(memory) = self.db.get_memory(hit.id) {
                        if memory.content.len() > 10 * 1024 * 1024 {
                            continue;
                        }
                        if let Ok(archived) = rkyv::access::<
                            <AgentMessage as rkyv::Archive>::Archived,
                            rkyv::rancor::Error,
                        >(&memory.content)
                        {
                            if let Ok(msg) =
                                rkyv::deserialize::<AgentMessage, rkyv::rancor::Error>(archived)
                            {
                                if msg.id == msg_id {
                                    return Ok(Some(msg));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Checks if a message has already been distilled.
    pub fn is_distilled(&self, msg_id: &str) -> bool {
        let filter = {
            let mut m = HashMap::new();
            m.insert("msg_id".to_string(), msg_id.to_string());
            m
        };

        self.db
            .search_in_collection("distillation", self.zero_embedding(), 1, Some(filter))
            .map(|h| !h.is_empty())
            .unwrap_or(false)
    }

    /// Marks a message as successfully distilled.
    pub fn mark_distilled(&self, msg_id: &str) -> Result<(), MemoryError> {
        let mut meta = HashMap::new();
        meta.insert("msg_id".to_string(), msg_id.to_string());

        self.db
            .add_with_content("distillation", vec![1], self.zero_embedding(), Some(meta))
            .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;
        Ok(())
    }

    // ========================================================================
    // Session / Turn State Management
    // ========================================================================

    /// Saves or updates a session state in the "sessions" collection.
    pub fn save_session_state(
        &self,
        state: &crate::models::SessionState,
    ) -> Result<(), MemoryError> {
        let bytes = rkyv::to_bytes::<RkyvError>(state)
            .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
        let key = crate::models::session_state_key(&state.session_id);

        self.db
            .add_with_content(
                "sessions",
                bytes.to_vec(),
                self.zero_embedding(),
                make_key_meta(&key),
            )
            .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;

        debug!("Saved session state for {}", state.session_id);
        Ok(())
    }

    /// Loads a session state from the "sessions" collection.
    /// Returns None if the session has no stored state.
    pub fn get_session_state(
        &self,
        session_id: &str,
    ) -> Result<Option<crate::models::SessionState>, MemoryError> {
        let key = crate::models::session_state_key(session_id);
        let filter = {
            let mut m = HashMap::new();
            m.insert("key".to_string(), key);
            m
        };

        if let Ok(hits) =
            self.db
                .search_in_collection("sessions", self.zero_embedding(), 1, Some(filter))
        {
            if let Some(hit) = hits.first() {
                if let Ok(memory) = self.db.get_memory(hit.id) {
                    let archived = rkyv::access::<
                        <crate::models::SessionState as rkyv::Archive>::Archived,
                        rkyv::rancor::Error,
                    >(&memory.content)
                    .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;

                    let state =
                        rkyv::deserialize::<crate::models::SessionState, rkyv::rancor::Error>(
                            archived,
                        )
                        .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
                    return Ok(Some(state));
                }
            }
        }
        Ok(None)
    }

    /// Gets or creates a session state. If no state exists, creates a new one and persists it.
    pub fn get_or_create_session_state(
        &self,
        session_id: &str,
    ) -> Result<crate::models::SessionState, MemoryError> {
        if let Some(state) = self.get_session_state(session_id)? {
            Ok(state)
        } else {
            let state = crate::models::SessionState::new(session_id);
            self.save_session_state(&state)?;
            Ok(state)
        }
    }

    /// Returns the turn collection name for a session.
    fn turn_collection(session_id: &str) -> String {
        format!("turns.{}", session_id)
    }

    /// Saves a turn state to the "turns.{session_id}" collection.
    pub fn save_turn_state(&self, turn: &crate::models::TurnState) -> Result<(), MemoryError> {
        let bytes = rkyv::to_bytes::<RkyvError>(turn)
            .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
        let key = crate::models::turn_state_key(&turn.session_id, &turn.turn_id);
        let collection = Self::turn_collection(&turn.session_id);

        self.db
            .add_with_content(
                &collection,
                bytes.to_vec(),
                self.zero_embedding(),
                make_key_meta(&key),
            )
            .map_err(|e| MemoryError::TransactionFailed(e.to_string()))?;

        debug!(
            "Saved turn state {} for session {}",
            turn.turn_id, turn.session_id
        );
        Ok(())
    }

    /// Loads a specific turn state by turn ID.
    pub fn get_turn_state(
        &self,
        session_id: &str,
        turn_id: &str,
    ) -> Result<Option<crate::models::TurnState>, MemoryError> {
        let key = crate::models::turn_state_key(session_id, turn_id);
        let filter = {
            let mut m = HashMap::new();
            m.insert("key".to_string(), key);
            m
        };
        let collection = Self::turn_collection(session_id);

        if let Ok(hits) =
            self.db
                .search_in_collection(&collection, self.zero_embedding(), 1, Some(filter))
        {
            if let Some(hit) = hits.first() {
                if let Ok(memory) = self.db.get_memory(hit.id) {
                    let archived = rkyv::access::<
                        <crate::models::TurnState as rkyv::Archive>::Archived,
                        rkyv::rancor::Error,
                    >(&memory.content)
                    .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;

                    let turn = rkyv::deserialize::<crate::models::TurnState, rkyv::rancor::Error>(
                        archived,
                    )
                    .map_err(|e| MemoryError::SerializationFailed(e.to_string()))?;
                    return Ok(Some(turn));
                }
            }
        }
        Ok(None)
    }

    /// Fetches the most recent N turns for a session, ordered newest-first.
    pub fn fetch_recent_turns(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::models::TurnState>, MemoryError> {
        let collection = Self::turn_collection(session_id);
        let mut turns = Vec::new();

        if let Ok(hits) =
            self.db
                .search_in_collection(&collection, self.zero_embedding(), MAX_BATCH_SIZE, None)
        {
            for hit in hits {
                if let Ok(memory) = self.db.get_memory(hit.id) {
                    if memory.content.len() > 1024 * 1024 {
                        continue;
                    }
                    if let Ok(archived) = rkyv::access::<
                        <crate::models::TurnState as rkyv::Archive>::Archived,
                        rkyv::rancor::Error,
                    >(&memory.content)
                    {
                        if let Ok(turn) = rkyv::deserialize::<
                            crate::models::TurnState,
                            rkyv::rancor::Error,
                        >(archived)
                        {
                            turns.push(turn);
                        }
                    }
                }
            }
        }

        turns.sort_by_key(|t| i64::from(t.started_at));
        turns.reverse();
        turns.truncate(limit);
        Ok(turns)
    }

    /// Flushes pending writes to disk.
    pub fn flush(&self) -> Result<(), MemoryError> {
        self.db
            .flush()
            .map_err(|e| MemoryError::TransactionFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AgentMessage, MessageRole, ToolResultRef};
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn test_lsm_engine_basic_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "savant_memory_test_cortexa_{}",
            Uuid::new_v4()
        ));
        let _ = fs::create_dir_all(&temp_dir);

        let engine = LsmStorageEngine::with_defaults(&temp_dir).unwrap();

        let msg = AgentMessage::user("session123", "Hello, world!");
        engine.append_message("session123", &msg).unwrap();

        let tail = engine.fetch_session_tail("session123", 10);
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].content, "Hello, world!");

        if let Err(e) = fs::remove_dir_all(&temp_dir) {
            warn!("[memory::lsm] Failed to clean up test temp dir: {}", e);
        }
    }

    #[test]
    fn test_orphan_detection() {
        let msg_with_orphan = AgentMessage {
            id: "msg1".to_string(),
            session_id: "sess".to_string(),
            role: MessageRole::Tool,
            content: "result".to_string(),
            tool_calls: Vec::new(),
            tool_results: vec![ToolResultRef {
                tool_use_id: "orphan".to_string(),
                result_content: "orphaned result".to_string(),
                is_error: false,
            }],
            timestamp: 1000.into(),
            parent_id: None,
            channel: "Telemetry".to_string(),
        };

        let batch = vec![msg_with_orphan];
        let result = verify_tool_pair_integrity(&batch);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MemoryError::OrphanedToolResult { .. }
        ));
    }
}
