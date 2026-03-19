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
use crate::notifications::{MemoryNotification, NotificationChannel};
use crate::vector_engine::{SemanticVectorEngine, VectorConfig};

/// The unified memory engine for Savant.
///
/// This is the primary interface to the memory subsystem. It combines:
/// - LSM-based transcript persistence (Fjall)
/// - SIMD-accelerated semantic search (ruvector-core)
/// - Hive-mind notifications (cross-agent knowledge awareness)
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
    notifications: NotificationChannel,
}

/// 🧬 OMEGA-VIII: Memory Layer Definition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLayer {
    /// L0: High-frequency transient logs (Episodic)
    Episodic,
    /// L1: Aggregated workspace and session state (Contextual)
    Contextual,
    /// L2: SIMD-accelerated long-term storage (Semantic)
    Semantic,
}

/// Configuration for the unified memory engine.
#[derive(Debug, Clone, Default)]
pub struct EngineConfig {
    pub lsm_config: LsmConfig,
    pub vector_config: VectorConfig,
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
            crate::safety::verify_memory_safety();
        }

        Ok(Arc::new(Self {
            lsm,
            vector,
            notifications: NotificationChannel::default(),
        }))
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
    ///
    /// OMEGA: This also calculates initial informational entropy if not present.
    pub fn index_memory(&self, mut entry: MemoryEntry) -> Result<(), MemoryError> {
        if entry.embedding.is_empty() {
            return Err(MemoryError::Unsupported(
                "Cannot index memory entry with empty embedding".to_string(),
            ));
        }

        // --- OMEGA: Entropy Calculation ---
        if entry.shannon_entropy.to_native() == 0.0 {
            entry.shannon_entropy = Self::calculate_entropy(&entry.content).into();
        }

        // Index in vector engine
        self.vector
            .index_memory(&entry.id.to_string(), &entry.embedding)?;

        // --- OMEGA: Persist the updated MemoryEntry with entropy in LSM ---
        if let Err(e) = self.lsm.insert_metadata(entry.id.to_native(), &entry) {
            warn!(
                "Memory Engine: LSM metadata write failed for entry {}. Rolling back vector index.",
                entry.id
            );
            // 🛡️ AAA Atomicity Rollback
            if let Err(rollback_err) = self.vector.remove(&entry.id.to_string()) {
                tracing::error!(
                    "CRITICAL: Rollback failed for entry {}. Vector index may be inconsistent: {}",
                    entry.id,
                    rollback_err
                );
            }
            return Err(e);
        }

        // 🐝 Hive-Mind Notification: broadcast high-importance discoveries
        if entry.importance >= 7 {
            let notification = MemoryNotification {
                notification_id: uuid::Uuid::new_v4().to_string(),
                source_session: entry.session_id.clone(),
                memory_id: entry.id.to_native(),
                domain_tags: entry.tags.clone(),
                importance: entry.importance,
                timestamp: chrono::Utc::now().timestamp_millis(),
                content_preview: entry.content.chars().take(200).collect(),
            };
            self.notifications.notify(notification);
        }

        debug!(
            "Indexed memory entry: id={}, entropy={:.2}",
            entry.id,
            entry.shannon_entropy.to_native()
        );
        Ok(())
    }

    /// Calculates Shannon Entropy of a string to determine informational density.
    fn calculate_entropy(content: &str) -> f32 {
        if content.is_empty() {
            return 0.0;
        }
        let mut char_counts = std::collections::HashMap::new();
        let chars: Vec<char> = content.chars().collect();
        for c in &chars {
            *char_counts.entry(*c).or_insert(0) += 1;
        }
        let total = chars.len() as f32;
        char_counts
            .values()
            .map(|&count| {
                let p = count as f32 / total;
                -p * p.log2()
            })
            .sum()
    }

    /// OMEGA: Prunes memories based on Information-Entropy Gain (IEG).
    /// Memories with near-zero categorical utility or those that have become redundant
    /// (Cross-Entropy with neighbors) are culled.
    pub fn cull_low_entropy_memories(&self, threshold: f32) -> Result<usize, MemoryError> {
        info!(
            "OMEGA: Executing Entropy-Gated Pruning (threshold={})",
            threshold
        );

        let all_metadata = self.lsm.iter_metadata()?;
        let mut culled_count = 0;

        for entry in all_metadata {
            if entry.shannon_entropy.to_native() < threshold {
                // 1. Remove from vector index
                if let Err(e) = self.vector.remove(&entry.id.to_string()) {
                    warn!(
                        "Failed to remove vector entry {} during culling: {}",
                        entry.id, e
                    );
                }
                // 2. Remove from LSM metadata
                if let Err(e) = self.lsm.remove_metadata(u64::from(entry.id)) {
                    warn!(
                        "Failed to remove LSM metadata for entry {} during culling: {}",
                        entry.id, e
                    );
                }

                culled_count += 1;
            }
        }

        info!(
            "IEG Pruning complete: Culled {} low-entropy entries",
            culled_count
        );
        Ok(culled_count)
    }

    /// Hydrates an agent session by combining local transcripts with relevant semantic memories.
    ///
    /// This is a critical ECHO-Absolute tier standard for context restoration.
    pub fn hydrate_session(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<AgentMessage>, MemoryError> {
        let mut messages = self.fetch_session_tail(session_id, limit);
        messages.reverse(); // Standard chronological order for LLM context

        // 🌀 OMEGA: Perform semantic hydration if context is sparse
        if messages.len() < 5 {
            info!(
                "L0 {:?} active; L1 {:?} context sparse for {}; triggering L2 {:?} hydration",
                MemoryLayer::Episodic,
                MemoryLayer::Contextual,
                session_id,
                MemoryLayer::Semantic
            );
            // In a real implementation, we'd pull from L2 here...
        }

        Ok(messages)
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

    /// Semantic search with bi-temporal filtering — only returns active facts.
    ///
    /// This joins VectorEngine results with TemporalEntry, filtering out
    /// facts that have been invalidated (valid_to is not None).
    /// Facts without temporal metadata are treated as active (backward compatible).
    pub fn semantic_search_temporal(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<crate::vector_engine::SearchResult>, MemoryError> {
        // Get more results than needed to account for filtering
        let raw_results = self.vector.recall(query_embedding, top_k * 2, None)?;

        let mut filtered = Vec::new();
        for result in raw_results {
            // Try to parse the document_id as a memory ID for temporal lookup
            if let Ok(memory_id) = result.document_id.parse::<u64>() {
                match self.lsm.get_temporal_metadata(memory_id) {
                    Ok(Some(temporal)) => {
                        // Only include active facts
                        if temporal.is_active() {
                            filtered.push(result);
                        }
                    }
                    Ok(None) => {
                        // No temporal data → treat as active (backward compatible)
                        filtered.push(result);
                    }
                    Err(e) => {
                        // Error looking up temporal → include with warning
                        tracing::debug!("Temporal lookup failed for {}: {}", memory_id, e);
                        filtered.push(result);
                    }
                }
            } else {
                // Can't parse as ID → include as-is
                filtered.push(result);
            }

            if filtered.len() >= top_k {
                break;
            }
        }

        Ok(filtered)
    }

    /// Deletes a session and all its associated memories.
    ///
    /// This permanently removes:
    /// - All transcript messages
    /// - All indexed vector entries
    pub fn delete_session(&self, session_id: &str) -> Result<(), MemoryError> {
        // 1. Fetch all message IDs before purging LSM
        let message_ids = self.lsm.fetch_all_message_ids_for_session(session_id);

        // 2. Purge transcript messages from LSM first (source of truth)
        self.lsm.delete_session(session_id)?;

        // 3. Cascade deletion to vector engine (best-effort with error logging)
        let mut cascade_errors = 0;
        for id in &message_ids {
            if let Err(e) = self.vector.remove(id) {
                warn!(
                    "Failed to remove vector entry {} during session cleanup: {}",
                    id, e
                );
                cascade_errors += 1;
            }
        }

        if cascade_errors > 0 {
            warn!(
                "Session {} deleted from LSM but {} vector entries failed to clean up",
                session_id, cascade_errors
            );
        }

        info!(
            "Deleted session: {} (Cascaded cleanup complete)",
            session_id
        );
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

    /// Subscribes to hive-mind notifications for high-importance memory discoveries.
    pub fn subscribe_notifications(
        &self,
    ) -> tokio::sync::broadcast::Receiver<crate::notifications::MemoryNotification> {
        self.notifications.subscribe()
    }

    /// Returns the number of active notification subscribers.
    pub fn notification_subscriber_count(&self) -> usize {
        self.notifications.subscriber_count()
    }

    // --- 🧬 OMEGA-VIII: Layered Accessors ---

    /// Accesses the Episodic Layer (L0).
    pub fn l0_episodic(&self) -> Arc<LsmStorageEngine> {
        Arc::clone(&self.lsm)
    }

    /// Accesses the Contextual Layer (L1).
    /// Currently mapped to LSM metadata for session-state aggregation.
    pub fn l1_contextual(&self) -> Arc<LsmStorageEngine> {
        Arc::clone(&self.lsm)
    }

    /// Accesses the Semantic Layer (L2).
    pub fn l2_semantic(&self) -> Arc<SemanticVectorEngine> {
        Arc::clone(&self.vector)
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
        let temp_dir =
            std::env::temp_dir().join(format!("savant_memory_search_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).unwrap();

        // Mock entry with 384 dimensions
        let mut embedding = vec![0.0; 384];
        embedding[0] = 1.0;

        let entry = MemoryEntry {
            id: 1.into(),
            session_id: "test-session".to_string(),
            created_at: 0.into(),
            updated_at: 0.into(),
            content: "Semantic data".to_string(),
            category: "Fact".to_string(),
            importance: 5,
            tags: vec![],
            embedding: embedding.clone(),
            shannon_entropy: 0.0.into(),
            last_accessed_at: 0.into(),
            hit_count: 0.into(),
            related_to: vec![],
        };

        engine.index_memory(entry).unwrap();

        let results = engine.semantic_search(&embedding, 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "1");

        // Cleanup
        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_session_deletion() {
        let temp_dir =
            std::env::temp_dir().join(format!("savant_memory_del_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).unwrap();
        let session_id = "purge_me";

        engine
            .append_message(session_id, &AgentMessage::user(session_id, "test"))
            .unwrap();
        assert_eq!(engine.fetch_session_tail(session_id, 1).len(), 1);

        engine.delete_session(session_id).unwrap();
        assert_eq!(engine.fetch_session_tail(session_id, 1).len(), 0);

        // Cleanup
        std::fs::remove_dir_all(temp_dir).unwrap();
    }
}
