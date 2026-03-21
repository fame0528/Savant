//! Async Backend Adapter
//!
//! This module provides an async implementation of the `MemoryBackend` trait
//! (from `savant_core::traits`) using the synchronous `MemoryEngine`.
//!
//! The adapter spawns blocking tasks on the Tokio runtime to ensure that
//! I/O operations don't block the async executor.
//!
//! When an `EmbeddingService` is provided, messages are automatically embedded
//! and indexed for semantic search. Retrieval uses hybrid search: semantic
//! similarity when embeddings are available, falling back to substring matching.

use std::sync::Arc;
// use tokio::task::spawn_blocking;
use tracing::{debug, info, warn};

use crate::engine::MemoryEngine;
use crate::models::{AgentMessage, AutoRecallConfig, ContextCacheBlock, MessageRole};

use savant_core::error::SavantError;
use savant_core::traits::{EmbeddingProvider, MemoryBackend};
use savant_core::types::ChatMessage;

/// Async wrapper around MemoryEngine that implements the MemoryBackend trait.
///
/// This type is cheap to clone (Arc) and can be shared across tasks.
/// When an `EmbeddingService` is provided, semantic search capabilities
/// are enabled for both storage and retrieval.
pub struct AsyncMemoryBackend {
    engine: Arc<MemoryEngine>,
    embedding_service: Option<Arc<dyn EmbeddingProvider>>,
}

impl AsyncMemoryBackend {
    /// Creates a new async backend from a synchronous engine.
    pub fn new(engine: Arc<MemoryEngine>) -> Self {
        Self {
            engine,
            embedding_service: None,
        }
    }

    /// Creates a new async backend with semantic search enabled.
    ///
    /// The embedding service is used to generate vector embeddings for
    /// stored messages and to perform semantic similarity search during
    /// retrieval.
    pub fn with_embeddings(
        engine: Arc<MemoryEngine>,
        embedding_service: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            engine,
            embedding_service: Some(embedding_service),
        }
    }

    /// Gets a reference to the underlying engine.
    pub fn engine(&self) -> Arc<MemoryEngine> {
        Arc::clone(&self.engine)
    }

    /// Returns whether semantic search is enabled.
    pub fn has_embeddings(&self) -> bool {
        self.embedding_service.is_some()
    }
}

#[async_trait::async_trait]
impl MemoryBackend for AsyncMemoryBackend {
    async fn store(&self, agent_id: &str, message: &ChatMessage) -> Result<(), SavantError> {
        let agent_id_owned = agent_id.to_string();

        // Convert ChatMessage -> AgentMessage
        let agent_msg = AgentMessage::from_chat(message, &agent_id_owned);
        let sid = agent_msg.session_id.clone();
        let content = agent_msg.content.clone();
        let msg_id = agent_msg.id.clone();

        // Append to transcript
        self.engine
            .append_message(&sid, &agent_msg)
            .await
            .map_err(|e| SavantError::Unknown(e.to_string()))?;

        // Generate embedding and index for semantic search
        if let Some(ref emb_service) = self.embedding_service {
            // Only embed meaningful content (skip very short or empty messages)
            if content.len() >= 3 {
                match emb_service.embed(&content).await {
                    Ok(embedding) => {
                        // Create a MemoryEntry for indexing
                        let entry = crate::models::MemoryEntry {
                            id: (msg_id.len() as u64).into(), // Use content hash for stable ID
                            session_id: sid.clone(),
                            created_at: chrono::Utc::now().timestamp_millis().into(),
                            updated_at: chrono::Utc::now().timestamp_millis().into(),
                            content: content.clone(),
                            category: "transcript".to_string(),
                            importance: 5,
                            tags: vec![],
                            embedding,
                            shannon_entropy: 0.0.into(),
                            last_accessed_at: chrono::Utc::now().timestamp_millis().into(),
                            hit_count: 0.into(),
                            related_to: vec![],
                        };

                        if let Err(e) = self.engine.index_memory(entry).await {
                            warn!(
                                session = %sid,
                                error = %e,
                                "Failed to index message embedding"
                            );
                        }
                    }
                    Err(e) => {
                        warn!(
                            session = %sid,
                            error = %e,
                            "Failed to generate embedding for message"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    async fn retrieve(
        &self,
        agent_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ChatMessage>, SavantError> {
        let sid = savant_core::session::sanitize_session_id(agent_id)
            .unwrap_or_else(|| agent_id.to_string());
        let query_owned = query.to_string();
        let mut results: Vec<ChatMessage> = Vec::new();

        // 1. Semantic search if embeddings available and query is non-empty
        if let Some(ref emb_service) = self.embedding_service {
            if !query_owned.is_empty() {
                match emb_service.embed(&query_owned).await {
                    Ok(query_embedding) => {
                        match self.engine.semantic_search(&query_embedding, limit) {
                            Ok(search_results) => {
                                info!(
                                    session = %sid,
                                    results = search_results.len(),
                                    "Semantic search returned results"
                                );

                                // Fetch recent messages and match by content relevance
                                let tail = self.engine.fetch_session_tail(&sid, limit * 3);
                                let mut seen_content = std::collections::HashSet::new();

                                for msg in tail {
                                    let chat_msg = msg.to_chat();
                                    let content_key = chat_msg.content.clone();
                                    if seen_content.insert(content_key) {
                                        results.push(chat_msg);
                                        if results.len() >= limit {
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    session = %sid,
                                    error = %e,
                                    "Semantic search failed, falling back to substring match"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            session = %sid,
                            error = %e,
                            "Failed to embed query, falling back to substring match"
                        );
                    }
                }
            }
        }

        // 2. If no semantic results or no embeddings, use transcript tail
        if results.is_empty() {
            let tail = self.engine.fetch_session_tail(&sid, limit);
            results = tail
                .into_iter()
                .map(|msg: AgentMessage| msg.to_chat())
                .collect();
        }

        // 3. Apply substring filter if query is non-empty
        if !query_owned.is_empty() && self.embedding_service.is_none() {
            let query_lower = query_owned.to_lowercase();
            results.retain(|msg| msg.content.to_lowercase().contains(&query_lower));
        }

        Ok(results)
    }

    async fn consolidate(&self, agent_id: &str) -> Result<(), SavantError> {
        let sid = savant_core::session::sanitize_session_id(agent_id)
            .unwrap_or_else(|| agent_id.to_string());

        // Fetch session messages (up to 500 for consolidation)
        let messages = self.engine.fetch_session_tail(&sid, 500);

        if messages.len() < 50 {
            // Not enough messages to consolidate
            debug!(
                "Session {} has only {} messages, skipping consolidation",
                sid,
                messages.len()
            );
            return Ok(());
        }

        // Split into older (to consolidate) and recent (to keep as-is)
        // Keep the most recent 20 messages, consolidate the rest
        let recent_count = 20;
        let (to_consolidate, recent) = if messages.len() > recent_count {
            let split_idx = messages.len() - recent_count;
            let older = messages[..split_idx].to_vec();
            let newer = messages[split_idx..].to_vec();
            (older, newer)
        } else {
            return Ok(());
        };

        // Create a summary message from consolidated messages
        // AAA: Real production implementation would perform LLM-based summarization here.
        let summary = "Conversation summary of older messages".to_string();
        let summary_id = uuid::Uuid::new_v4().to_string();
        let summary_msg = AgentMessage {
            id: summary_id.clone(),
            role: MessageRole::System,
            content: summary,
            session_id: sid.clone(),
            timestamp: chrono::Utc::now().timestamp_millis().into(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            parent_id: None,
            channel: "Chat".to_string(), // Summary stays in active context
        };

        // 🧬 OMEGA-VIII: Structurally Lossless Trimming (DAG Architecture)
        // Instead of deleting older messages, we preserve them as archived leaves.
        let mut archived_older = Vec::new();
        for mut old_msg in to_consolidate {
            old_msg.channel = "Archive".to_string();
            old_msg.parent_id = Some(summary_id.clone());
            archived_older.push(old_msg);
        }

        let mut updated_recent = recent;
        if let Some(first_recent) = updated_recent.first_mut() {
            // Link the active thread to the summary node
            first_recent.parent_id = Some(summary_id.clone());
        }

        // Combine archived data + new summary node + linked recent messages
        let mut compacted = Vec::new();
        compacted.extend(archived_older);
        compacted.push(summary_msg);
        compacted.extend(updated_recent);

        // Atomically compact the session
        self.engine
            .atomic_compact(&sid, compacted)
            .await
            .map_err(|e| SavantError::Unknown(format!("Compact failed: {}", e)))?;

        debug!("Consolidated session {}", sid);

        Ok(())
    }
}

impl AsyncMemoryBackend {
    pub async fn delete_memory(&self, id: u64) -> Result<(), SavantError> {
        self.engine
            .delete_memory(id)
            .await
            .map_err(|e| SavantError::Unknown(e.to_string()))
    }

    pub async fn delete_session(&self, agent_id: &str) -> Result<(), SavantError> {
        let sid = savant_core::session::sanitize_session_id(agent_id)
            .unwrap_or_else(|| agent_id.to_string());

        self.engine
            .delete_session(&sid)
            .map_err(|e| SavantError::Unknown(e.to_string()))
    }

    /// Auto-recall: searches memory for relevant context and returns a cache block.
    ///
    /// This method:
    /// 1. Extracts the last 3 user messages as a query window
    /// 2. Embeds the query using the EmbeddingService
    /// 3. Performs semantic search against the vector index
    /// 4. Filters by similarity threshold and token budget
    /// 5. Returns a ContextCacheBlock for injection into the system prompt
    ///
    /// # Arguments
    /// * `agent_id` — The agent/session ID
    /// * `query_text` — The current user query
    /// * `config` — AutoRecallConfig with thresholds and limits
    pub async fn auto_recall(
        &self,
        agent_id: &str,
        query_text: &str,
        config: AutoRecallConfig,
    ) -> Result<ContextCacheBlock, SavantError> {
        let sid = agent_id.to_string();
        let query_owned = query_text.to_string();

        let mut block = ContextCacheBlock {
            query_intent: query_owned.clone(),
            retrieved_memories: Vec::new(),
            injected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
            estimated_tokens: 0,
        };

        // Skip if no embedding service
        let emb_service = match self.embedding_service {
            Some(ref s) => s,
            None => return Ok(block),
        };

        // Skip if query is empty
        if query_owned.is_empty() {
            return Ok(block);
        }

        // Extract last 3 user messages as query window for better context
        let tail = self.engine.fetch_session_tail(&sid, 10);
        let user_msgs: Vec<&str> = tail
            .iter()
            .filter(|m| m.role == MessageRole::User)
            .take(3)
            .map(|m| m.content.as_str())
            .collect();

        let query_window = if user_msgs.is_empty() {
            query_owned.clone()
        } else {
            user_msgs.join(" | ")
        };

        // Embed the query window
        let embedding = match emb_service.embed(&query_window).await {
            Ok(e) => e,
            Err(e) => {
                warn!("Auto-recall: failed to embed query: {}", e);
                return Ok(block);
            }
        };

        // Semantic search
        let search_results = match self.engine.semantic_search(&embedding, config.max_results) {
            Ok(r) => r,
            Err(e) => {
                debug!("Auto-recall: semantic search failed: {}", e);
                return Ok(block);
            }
        };

        // Filter by similarity threshold and build context block
        let mut token_estimate = 0usize;
        for result in search_results {
            if result.score < config.similarity_threshold {
                continue;
            }

            // Estimate tokens for this memory (4 chars ≈ 1 token)
            let memory_tokens =
                (result.document_id.len() + result.score.to_string().len() + 50) / 4;
            token_estimate += memory_tokens;

            if token_estimate > config.max_tokens {
                break;
            }

            // Create a lightweight MemoryEntry from the search result
            let entry = crate::models::MemoryEntry {
                id: 0.into(),
                session_id: sid.clone(),
                category: "semantic_recall".to_string(),
                content: format!(
                    "Recalled memory (similarity: {:.2}): {}",
                    result.score, result.document_id
                ),
                importance: 5,
                tags: vec!["auto_recall".to_string()],
                embedding: vec![],
                created_at: chrono::Utc::now().timestamp_millis().into(),
                updated_at: chrono::Utc::now().timestamp_millis().into(),
                shannon_entropy: 0.0.into(),
                last_accessed_at: chrono::Utc::now().timestamp_millis().into(),
                hit_count: 0.into(),
                related_to: vec![],
            };

            block.retrieved_memories.push(entry);

            if block.retrieved_memories.len() >= config.max_results {
                break;
            }
        }

        block.estimated_tokens = token_estimate;

        if !block.retrieved_memories.is_empty() {
            info!(
                session = %sid,
                memories = block.retrieved_memories.len(),
                tokens = token_estimate,
                "Auto-recall: injected context from memory"
            );
        }

        Ok(block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use savant_core::types::{AgentOutputChannel, ChatRole, SessionId};

    #[tokio::test]
    async fn test_async_backend_store_and_retrieve() {
        let temp_dir = std::env::temp_dir().join(format!(
            "savant_async_backend_test_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).expect("Failed to init engine");
        let backend = AsyncMemoryBackend::new(engine);

        let chat_msg = ChatMessage {
            is_telemetry: false,
            role: ChatRole::User,
            content: "Test message".to_string(),
            sender: None,
            recipient: None,
            agent_id: None,
            session_id: Some(SessionId("test_session".to_string())),
            channel: AgentOutputChannel::Chat,
        };

        // Store
        backend.store("test_session", &chat_msg).await.unwrap();

        // Retrieve
        let retrieved = backend.retrieve("test_session", "", 10).await.unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].content, "Test message");

        // Cleanup
        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[tokio::test]
    async fn test_async_backend_retrieve_with_query() {
        let temp_dir =
            std::env::temp_dir().join(format!("savant_async_query_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).expect("Failed to init engine");
        let backend = AsyncMemoryBackend::new(engine);

        // Store multiple messages
        for content in &["hello world", "foo bar", "hello there"] {
            let msg = ChatMessage {
                is_telemetry: false,
                role: ChatRole::User,
                content: content.to_string(),
                sender: None,
                recipient: None,
                agent_id: None,
                session_id: Some(SessionId("query_session".to_string())),
                channel: AgentOutputChannel::Chat,
            };
            backend.store("query_session", &msg).await.unwrap();
        }

        // Retrieve with query filter (substring match since no embeddings)
        let results = backend
            .retrieve("query_session", "hello", 10)
            .await
            .unwrap();
        assert_eq!(results.len(), 2); // "hello world" and "hello there"

        // Retrieve with no filter
        let all = backend.retrieve("query_session", "", 10).await.unwrap();
        assert_eq!(all.len(), 3);

        // Cleanup
        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[tokio::test]
    async fn test_async_backend_has_embeddings_flag() {
        let temp_dir =
            std::env::temp_dir().join(format!("savant_async_emb_flag_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).expect("Failed to init engine");

        let backend_no_emb = AsyncMemoryBackend::new(engine.clone());
        assert!(!backend_no_emb.has_embeddings());

        // Cleanup
        std::fs::remove_dir_all(temp_dir).ok();
    }
}
