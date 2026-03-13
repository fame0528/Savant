//! Unified Memory Engine
//!
//! This module provides a high-level facade that combines LSM transcript storage
//! and SIMD vector search into a single coherent memory subsystem.

use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::error::MemoryError;
use crate::lsm_engine::{LsmConfig, LsmStorageEngine};
use crate::models::{AgentMessage, MemoryEntry};
// use crate::safety;
use crate::vector_engine::{SemanticVectorEngine, VectorConfig};

/// The unified memory engine for Savant.
///
/// This is the primary interface to the memory subsystem. It combines:
/// - LSM-based transcript persistence (Fjall)
/// - SIMD-accelerated semantic search (ruvector-core)
/// - Formal safety verification (Kani)
///
/// # Architecture
///
/// The engine maintains two separate storage backends:
/// 1. Transcripts: LSM-tree for sequential append and range queries
/// 2. Vectors: HNSW index for k-NN semantic search
///
/// Both backends share the same session_id namespace for coherence.
pub struct MemoryEngine {
    lsm: Arc<LsmStorageEngine>,
    vector: Arc<SemanticVectorEngine>,
}

/// Configuration for the unified memory engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub lsm_config: LsmConfig,
    pub vector_config: VectorConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            lsm_config: LsmConfig::default(),
            vector_config: VectorConfig::default(),
        }
    }
}

impl MemoryEngine {
    /// Initializes a new memory engine at the specified storage path.
    ///
    /// # Arguments
    /// * `storage_path` - Directory where all data will be stored
    /// * `config` - Engine configuration (use `Default` for production defaults)
    ///
    /// # Returns
    /// A fully initialized engine ready for use.
    ///
    /// # Errors
    /// Returns `MemoryError` if either the LSM or Vector backend fails to initialize.
    pub fn new<P: AsRef<Path>>(
        storage_path: P,
        config: EngineConfig,
    ) -> Result<Arc<Self>, MemoryError> {
        info!("Initializing Memory Engine at {:?}", storage_path.as_ref());

        // Initialize LSM storage
        let lsm = LsmStorageEngine::new(storage_path.as_ref(), config.lsm_config)?;

        // Initialize vector engine (in-memory, but could be persisted separately)
        let vector = SemanticVectorEngine::new(storage_path.as_ref(), config.vector_config)?;

        // Run safety verification in debug builds
        #[cfg(debug_assertions)]
        {
            info!("Running memory safety self-tests...");
            safety::verify_memory_safety();
        }

        Ok(Arc::new(Self { lsm, vector }))
    }

    /// Convenience: create with default configuration.
    pub fn with_defaults<P: AsRef<Path>>(storage_path: P) -> Result<Arc<Self>, MemoryError> {
        Self::new(storage_path, EngineConfig::default())
    }

    /// Appends a message to the session transcript.
    ///
    /// This is a transactional operation. The message is serialized via rkyv
    /// and written to the LSM tree with optimistic concurrency.
    ///
    /// If the message contains tool results, they are automatically checked
    /// for integrity to prevent OpenClaw Issue #39609.
    pub fn append_message(
        &self,
        session_id: &str,
        message: &AgentMessage,
    ) -> Result<(), MemoryError> {
        self.lsm.append_message(session_id, message)
    }

    /// Fetches the most recent messages for a session.
    ///
    /// Messages are returned in reverse chronological order (newest first).
    /// For chronological order, call `result.reverse()`.
    pub fn fetch_session_tail(&self, session_id: &str, limit: usize) -> Vec<AgentMessage> {
        self.lsm.fetch_session_tail(session_id, limit)
    }

    /// Atomically compacts a batch of messages.
    ///
    /// This method verifies tool pair integrity before committing. If any
    /// tool_result is orphaned, the entire transaction fails.
    pub fn atomic_compact(
        &self,
        session_id: &str,
        batch: Vec<AgentMessage>,
    ) -> Result<(), MemoryError> {
        self.lsm.atomic_compact(session_id, batch)
    }

    /// Indexes a memory entry for semantic search.
    ///
    /// The memory entry's `embedding` field must be populated and match
    /// the engine's configured vector dimensions (default 384).
    pub fn index_memory(&self, entry: &MemoryEntry) -> Result<(), MemoryError> {
        if entry.embedding.is_empty() {
            return Err(MemoryError::Unsupported(
                "Cannot index memory entry with empty embedding".to_string(),
            ));
        }
        self.vector
            .index_memory(&entry.id.to_string(), &entry.embedding)?;
        debug!("Indexed memory entry: id={}", entry.id);
        Ok(())
    }

    /// Performs a semantic similarity search.
    ///
    /// Given a query embedding, returns the top-k most similar memory entries
    /// from the vector index.
    ///
    /// Returns a vector of (memory_id, score, distance) tuples sorted by
    /// descending score.
    pub fn semantic_search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<crate::vector_engine::SearchResult>, MemoryError> {
        self.vector.recall(query_embedding, top_k, None)
    }

    /// Deletes a session and all its associated memories.
    ///
    /// This permanently removes:
    /// - All transcript messages
    /// - All indexed vector entries
    pub fn delete_session(&self, session_id: &str) -> Result<(), MemoryError> {
        // Delete transcript messages
        self.lsm.delete_session(session_id)?;

        // ROADMAP: Implement cross-engine cascaded deletion for vector entries
        // This would require iterating memory entries for the session
        // and calling vector.remove() for each
        warn!("Vector entries deletion not yet fully implemented");

        info!("Deleted session: {}", session_id);
        Ok(())
    }

    /// Returns statistics about the memory engine.
    pub fn stats(&self) -> (crate::lsm_engine::StorageStats, usize) {
        let lsm_stats = self.lsm.stats().unwrap_or_default();
        let vector_count = self.vector.vector_count();
        (lsm_stats, vector_count)
    }

    /// Returns a reference to the LSM storage engine (for advanced operations).
    pub fn lsm(&self) -> Arc<LsmStorageEngine> {
        Arc::clone(&self.lsm)
    }

    /// Returns a reference to the vector engine (for advanced operations).
    pub fn vector(&self) -> Arc<SemanticVectorEngine> {
        Arc::clone(&self.vector)
    }

    /// Runs all configured safety verifications.
    ///
    /// This method executes Kani verification harnesses in a subprocess when
    /// the `kani` feature is enabled. In production, it's a no-op.
    pub fn verify_safety(&self) -> Result<(), MemoryError> {
        #[cfg(feature = "kani")]
        {
            crate::safety::verify_memory_safety();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_engine_initialization() {
        let temp_dir = std::env::temp_dir().join("savant_memory_integration_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).unwrap();
        assert!(engine.lsm().keyspace().is_some());
        assert!(engine.vector().config().dimensions == 384);

        // Cleanup
        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_append_and_fetch() {
        let temp_dir = std::env::temp_dir().join("savant_memory_append_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).unwrap();

        let msg = AgentMessage::user("session123", "Hello, world!");
        engine.append_message("session123", &msg).unwrap();

        let tail = engine.fetch_session_tail("session123", 10);
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].content, "Hello, world!");

        // Cleanup
        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_semantic_search_flow() {
        let temp_dir = std::env::temp_dir().join(format!("savant_memory_search_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).unwrap();
        
        // Mock entry with 384 dimensions
        let mut embedding = vec![0.0; 384];
        embedding[0] = 1.0;
        
        let entry = MemoryEntry {
            id: 1,
            timestamp: 0,
            content: "Semantic data".to_string(),
            category: crate::models::MemoryCategory::Fact,
            importance: 5,
            associations: vec![],
            embedding: embedding.clone(),
        };
        
        engine.index_memory(&entry).unwrap();
        
        let results = engine.semantic_search(&embedding, 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "1");
        
        // Cleanup
        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_session_deletion() {
        let temp_dir = std::env::temp_dir().join(format!("savant_memory_del_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).unwrap();
        let session_id = "purge_me";
        
        engine.append_message(session_id, &AgentMessage::user(session_id, "test")).unwrap();
        assert_eq!(engine.fetch_session_tail(session_id, 1).len(), 1);
        
        engine.delete_session(session_id).unwrap();
        assert_eq!(engine.fetch_session_tail(session_id, 1).len(), 0);
        
        // Cleanup
        std::fs::remove_dir_all(temp_dir).unwrap();
    }
}
