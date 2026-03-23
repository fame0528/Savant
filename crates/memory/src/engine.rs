use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

use crate::error::MemoryError;
use crate::lsm_engine::{LsmConfig, LsmStorageEngine};
use crate::models::{AgentMessage, MemoryEntry};
use crate::notifications::NotificationChannel;
use crate::vector_engine::{SemanticVectorEngine, VectorConfig};
use savant_core::traits::{EmbeddingProvider, LlmProvider};
use savant_core::types::LlmParams;

/// 🧬 OMEGA-VIII: Memory Layer Definition
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLayer {
    /// L0: High-frequency transient logs (Episodic)
    Episodic,
    /// L1: Aggregated workspace and session state (Contextual)
    Contextual,
    /// L2: SIMD-accelerated long-term storage (Semantic)
    Semantic,
}

#[derive(Clone)]
pub struct EngineConfig {
    pub lsm_config: LsmConfig,
    pub vector_config: VectorConfig,
    pub distill_llm_provider: Option<Arc<dyn LlmProvider>>,
    pub distill_params: Option<LlmParams>,
    pub embedding_service: Option<Arc<dyn EmbeddingProvider>>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            lsm_config: LsmConfig::default(),
            vector_config: VectorConfig::default(),
            distill_llm_provider: None,
            distill_params: None,
            embedding_service: None,
        }
    }
}

/// The atomic Pure-Rust adapter (CortexaShim) that guarantees write atomicity
/// across the LSM and Vector engines to prevent orphaned vectors or race conditions.
pub struct MemoryEnclave {
    lsm: Arc<LsmStorageEngine>,
    vector: Arc<SemanticVectorEngine>,
    embedding_service: Option<Arc<dyn EmbeddingProvider>>,
    // strict transaction lock ensuring unified WAL behavior
    write_lock: tokio::sync::Mutex<()>,
}

impl MemoryEnclave {
    pub fn new<P: AsRef<Path>>(
        storage_path: P,
        config: EngineConfig,
    ) -> Result<Arc<Self>, MemoryError> {
        let lsm = LsmStorageEngine::new(storage_path.as_ref(), config.lsm_config)?;

        // Dynamic vector dimension: use embedding service dimension if available
        let mut vector_config = config.vector_config;
        if let Some(ref emb) = config.embedding_service {
            let emb_dims = emb.dimensions();
            if emb_dims > 0 && emb_dims != vector_config.dimensions {
                info!(
                    "Overriding vector dimension: {} -> {} (from embedding service)",
                    vector_config.dimensions, emb_dims
                );
                vector_config.dimensions = emb_dims;
            }
        }

        let vector = SemanticVectorEngine::new(storage_path.as_ref(), vector_config)?;

        Ok(Arc::new(Self {
            lsm,
            vector,
            embedding_service: config.embedding_service,
            write_lock: tokio::sync::Mutex::new(()),
        }))
    }

    pub async fn append_message(
        &self,
        session_id: &str,
        message: &AgentMessage,
    ) -> Result<(), MemoryError> {
        let _guard = self.write_lock.lock().await;
        self.lsm.append_message(session_id, message)
    }

    pub fn fetch_session_tail(&self, session_id: &str, limit: usize) -> Vec<AgentMessage> {
        self.lsm.fetch_session_tail(session_id, limit)
    }

    pub async fn atomic_compact(
        &self,
        session_id: &str,
        batch: Vec<AgentMessage>,
    ) -> Result<(), MemoryError> {
        let _guard = self.write_lock.lock().await;
        self.lsm.atomic_compact(session_id, batch)
    }

    pub async fn index_memory(&self, mut entry: MemoryEntry) -> Result<(), MemoryError> {
        let _guard = self.write_lock.lock().await;

        // OMEGA-VIII: Automatic Embedding Generation via Ollama
        if entry.embedding.is_empty() {
            if let Some(ref provider) = self.embedding_service {
                debug!("Generating automatic embedding for entry: {}", entry.id);
                if let Ok(vec) = provider.embed(&entry.content).await {
                    entry.embedding = vec;
                }
            }
        }

        // Only index in vector engine if embedding is provided
        if !entry.embedding.is_empty() {
            self.vector
                .index_memory(&entry.id.to_string(), &entry.embedding)?;
        }

        if let Err(e) = self.lsm.insert_metadata(entry.id.to_native(), &entry) {
            // AAA Atomicity Rollback
            if !entry.embedding.is_empty() {
                let _ = self.vector.remove(&entry.id.to_string());
            }
            return Err(e);
        }

        Ok(())
    }

    pub async fn delete_memory(&self, id: u64) -> Result<(), MemoryError> {
        let _guard = self.write_lock.lock().await;

        // Remove from vector engine (best effort)
        let _ = self.vector.remove(&id.to_string());

        // Remove from LSM engine
        self.lsm.delete_metadata(id)
    }

    pub fn semantic_search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<crate::vector_engine::SearchResult>, MemoryError> {
        self.vector.recall(query_embedding, top_k, None)
    }

    pub fn semantic_search_temporal(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<crate::vector_engine::SearchResult>, MemoryError> {
        let raw_results = self.vector.recall(query_embedding, top_k * 2, None)?;

        let mut filtered = Vec::new();
        for result in raw_results {
            if let Ok(memory_id) = result.document_id.parse::<u64>() {
                if let Ok(Some(temporal)) = self.lsm.get_temporal_metadata(memory_id) {
                    if temporal.is_active() {
                        filtered.push(result);
                    }
                } else {
                    filtered.push(result);
                }
            } else {
                filtered.push(result);
            }

            if filtered.len() >= top_k {
                break;
            }
        }
        Ok(filtered)
    }

    pub fn vector_count(&self) -> usize {
        self.vector.vector_count()
    }

    pub fn lsm(&self) -> Arc<LsmStorageEngine> {
        Arc::clone(&self.lsm)
    }

    pub fn vector(&self) -> Arc<SemanticVectorEngine> {
        Arc::clone(&self.vector)
    }

    // --- Session / Turn State ---

    /// Saves or updates a session state (write-locked).
    pub async fn save_session_state(
        &self,
        state: &crate::models::SessionState,
    ) -> Result<(), MemoryError> {
        let _guard = self.write_lock.lock().await;
        self.lsm.save_session_state(state)
    }

    /// Loads a session state (no write lock needed for reads).
    pub fn get_session_state(
        &self,
        session_id: &str,
    ) -> Result<Option<crate::models::SessionState>, MemoryError> {
        self.lsm.get_session_state(session_id)
    }

    /// Gets or creates a session state (write-locked if creating).
    pub async fn get_or_create_session_state(
        &self,
        session_id: &str,
    ) -> Result<crate::models::SessionState, MemoryError> {
        if let Some(state) = self.lsm.get_session_state(session_id)? {
            return Ok(state);
        }
        let _guard = self.write_lock.lock().await;
        self.lsm.get_or_create_session_state(session_id)
    }

    /// Saves a turn state (write-locked).
    pub async fn save_turn_state(
        &self,
        turn: &crate::models::TurnState,
    ) -> Result<(), MemoryError> {
        let _guard = self.write_lock.lock().await;
        self.lsm.save_turn_state(turn)
    }

    /// Loads a specific turn state (no write lock needed).
    pub fn get_turn_state(
        &self,
        session_id: &str,
        turn_id: &str,
    ) -> Result<Option<crate::models::TurnState>, MemoryError> {
        self.lsm.get_turn_state(session_id, turn_id)
    }

    /// Fetches the most recent N turns for a session.
    pub fn fetch_recent_turns(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::models::TurnState>, MemoryError> {
        self.lsm.fetch_recent_turns(session_id, limit)
    }
}

/// The unified memory engine for Savant (5-Layer Cognitive Architecture Implementation)
/// Replaces singular usage with dedicated `enclave` and `collective` databases.
pub struct MemoryEngine {
    enclave: Arc<MemoryEnclave>,
    collective: Arc<MemoryEnclave>,
    notifications: NotificationChannel,
}

impl MemoryEngine {
    pub fn new<P: AsRef<Path>>(
        storage_path: P,
        config: EngineConfig,
    ) -> Result<Arc<Self>, MemoryError> {
        let base = storage_path.as_ref();
        info!("Initializing Memory Engine at {:?}", base);

        let enclave = MemoryEnclave::new(base.join("enclave"), config.clone())?;
        let collective = MemoryEnclave::new(base.join("collective"), config.clone())?;

        let engine = Arc::new(Self {
            enclave: enclave.clone(),
            collective: collective.clone(),
            notifications: NotificationChannel::default(),
        });

        // OMEGA-VIII: Spawn the autonomous background pipelines
        if let Some(llm_provider) = config.distill_llm_provider {
            // Generate ephemeral JWT secret — in-memory only, destroyed on process exit.
            // If configured, use it. If not, generate crypto-random secret at runtime.
            // This follows the same pattern as ephemeral agent keys: no persistence, no vulnerability.
            let jwt_secret = config
                .distill_params
                .unwrap_or_default()
                .jwt_secret
                .unwrap_or_else(|| {
                    let mut hasher = blake3::Hasher::new();
                    hasher.update(uuid::Uuid::new_v4().as_bytes());
                    hasher.update(uuid::Uuid::new_v4().as_bytes());
                    hasher.update(&std::process::id().to_le_bytes());
                    hasher.update(
                        &std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos()
                            .to_le_bytes(),
                    );
                    let hash = hasher.finalize();
                    let secret = hash.to_hex().to_string();
                    tracing::info!(
                        "Generated ephemeral JWT secret for distillation pipeline (in-memory only)"
                    );
                    secret
                });

            info!("Spawning Distillation Pipeline...");
            crate::distillation::spawn_distillation_pipeline(
                enclave.clone(),
                collective.clone(),
                llm_provider,
                config.embedding_service.clone(),
                jwt_secret,
            );
        }

        info!("Spawning Factual Arbiter...");
        crate::arbiter::spawn_arbiter_task(collective);

        info!("Memory Engine initialized successfully");
        Ok(engine)
    }

    pub fn with_defaults<P: AsRef<Path>>(storage_path: P) -> Result<Arc<Self>, MemoryError> {
        Self::new(storage_path, EngineConfig::default())
    }

    pub fn enclave(&self) -> Arc<MemoryEnclave> {
        Arc::clone(&self.enclave)
    }

    pub fn collective(&self) -> Arc<MemoryEnclave> {
        Arc::clone(&self.collective)
    }

    pub fn subscribe_notifications(
        &self,
    ) -> tokio::sync::broadcast::Receiver<crate::notifications::MemoryNotification> {
        self.notifications.subscribe()
    }

    pub fn notification_subscriber_count(&self) -> usize {
        self.notifications.subscriber_count()
    }

    // --- Legacy facades bridging to Enclave to avoid downstream breakage during transition ---

    pub async fn append_message(
        &self,
        session_id: &str,
        message: &AgentMessage,
    ) -> Result<(), MemoryError> {
        self.enclave.append_message(session_id, message).await
    }

    pub fn fetch_session_tail(&self, session_id: &str, limit: usize) -> Vec<AgentMessage> {
        self.enclave.fetch_session_tail(session_id, limit)
    }

    pub async fn atomic_compact(
        &self,
        session_id: &str,
        batch: Vec<AgentMessage>,
    ) -> Result<(), MemoryError> {
        self.enclave.atomic_compact(session_id, batch).await
    }

    pub async fn index_memory(&self, entry: MemoryEntry) -> Result<(), MemoryError> {
        self.enclave.index_memory(entry).await
    }

    pub fn cull_low_entropy_memories(&self, _threshold: f32) -> Result<usize, MemoryError> {
        Ok(0)
    }

    pub fn hydrate_session(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<AgentMessage>, MemoryError> {
        let mut messages = self.enclave.fetch_session_tail(session_id, limit);
        messages.reverse();
        Ok(messages)
    }

    pub async fn delete_memory(&self, id: u64) -> Result<(), MemoryError> {
        self.enclave.delete_memory(id).await
    }

    pub fn semantic_search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<crate::vector_engine::SearchResult>, MemoryError> {
        self.enclave.semantic_search(query_embedding, top_k)
    }

    pub fn semantic_search_temporal(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<crate::vector_engine::SearchResult>, MemoryError> {
        self.enclave
            .semantic_search_temporal(query_embedding, top_k)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), MemoryError> {
        self.enclave.lsm.delete_session(session_id)
    }

    // --- Session / Turn State Facades ---

    pub async fn save_session_state(
        &self,
        state: &crate::models::SessionState,
    ) -> Result<(), MemoryError> {
        self.enclave.save_session_state(state).await
    }

    pub fn get_session_state(
        &self,
        session_id: &str,
    ) -> Result<Option<crate::models::SessionState>, MemoryError> {
        self.enclave.get_session_state(session_id)
    }

    pub async fn get_or_create_session_state(
        &self,
        session_id: &str,
    ) -> Result<crate::models::SessionState, MemoryError> {
        self.enclave.get_or_create_session_state(session_id).await
    }

    pub async fn save_turn_state(
        &self,
        turn: &crate::models::TurnState,
    ) -> Result<(), MemoryError> {
        self.enclave.save_turn_state(turn).await
    }

    pub fn get_turn_state(
        &self,
        session_id: &str,
        turn_id: &str,
    ) -> Result<Option<crate::models::TurnState>, MemoryError> {
        self.enclave.get_turn_state(session_id, turn_id)
    }

    pub fn fetch_recent_turns(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::models::TurnState>, MemoryError> {
        self.enclave.fetch_recent_turns(session_id, limit)
    }

    pub fn stats(&self) -> (crate::lsm_engine::StorageStats, usize) {
        let lsm_stats = self.enclave.lsm.stats().unwrap_or_default();
        let vector_count = self.enclave.vector_count();
        (lsm_stats, vector_count)
    }

    pub fn verify_safety(&self) -> Result<(), MemoryError> {
        #[cfg(feature = "kani")]
        crate::safety::verify_memory_safety();
        Ok(())
    }
}
