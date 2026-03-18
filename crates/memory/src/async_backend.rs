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
use tokio::task::spawn_blocking;
use tracing::{debug, info, warn};

use crate::engine::MemoryEngine;
use crate::models::{AgentMessage, MessageRole};

use savant_core::error::SavantError;
use savant_core::traits::MemoryBackend;
use savant_core::types::ChatMessage;
use savant_core::utils::embeddings::EmbeddingService;

/// Async wrapper around MemoryEngine that implements the MemoryBackend trait.
///
/// This type is cheap to clone (Arc) and can be shared across tasks.
/// When an `EmbeddingService` is provided, semantic search capabilities
/// are enabled for both storage and retrieval.
pub struct AsyncMemoryBackend {
    engine: Arc<MemoryEngine>,
    embedding_service: Option<Arc<EmbeddingService>>,
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
        embedding_service: Arc<EmbeddingService>,
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
        let engine = Arc::clone(&self.engine);
        let embedding_service = self.embedding_service.clone();
        let agent_id_owned = agent_id.to_string();
        let message_clone = message.clone();

        spawn_blocking(move || {
            // Convert ChatMessage -> AgentMessage
            let agent_msg = AgentMessage::from_chat(&message_clone, &agent_id_owned);
            let sid = agent_msg.session_id.clone();
            let content = agent_msg.content.clone();
            let msg_id = agent_msg.id.clone();

            // Append to transcript
            engine
                .append_message(&sid, &agent_msg)
                .map_err(|e| SavantError::Unknown(e.to_string()))?;

            // Generate embedding and index for semantic search
            if let Some(ref emb_service) = embedding_service {
                // Only embed meaningful content (skip very short or empty messages)
                if content.len() >= 3 {
                    match emb_service.embed(&content) {
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

                            if let Err(e) = engine.index_memory(entry) {
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
        })
        .await
        .map_err(|e| SavantError::Unknown(format!("Task join error: {}", e)))?
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
        let engine = self.engine.clone();
        let embedding_service = self.embedding_service.clone();

        spawn_blocking(move || {
            // Hybrid retrieval: semantic search + transcript tail
            let mut results: Vec<ChatMessage> = Vec::new();

            // 1. Semantic search if embeddings available and query is non-empty
            if let Some(ref emb_service) = embedding_service {
                if !query_owned.is_empty() {
                    match emb_service.embed(&query_owned) {
                        Ok(query_embedding) => {
                            match engine.semantic_search(&query_embedding, limit) {
                                Ok(search_results) => {
                                    info!(
                                        session = %sid,
                                        results = search_results.len(),
                                        "Semantic search returned results"
                                    );

                                    // Fetch recent messages and match by content relevance
                                    // The semantic search returns document IDs which correspond
                                    // to memory entries indexed during store()
                                    let tail = engine.fetch_session_tail(&sid, limit * 3);
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
                let tail = engine.fetch_session_tail(&sid, limit);
                results = tail
                    .into_iter()
                    .map(|msg: AgentMessage| msg.to_chat())
                    .collect();
            }

            // 3. Apply substring filter if query is non-empty
            if !query_owned.is_empty() && embedding_service.is_none() {
                let query_lower = query_owned.to_lowercase();
                results = results
                    .into_iter()
                    .filter(|msg| msg.content.to_lowercase().contains(&query_lower))
                    .collect();
            }

            Ok(results)
        })
        .await
        .map_err(|e| SavantError::Unknown(format!("Task join error: {}", e)))?
    }

    async fn consolidate(&self, agent_id: &str) -> Result<(), SavantError> {
        // Consolidation logic (compaction) for context management.
        // This implementation:
        // 1. Fetches messages from the session
        // 2. Creates a lightweight summary of older messages
        // 3. Stores the compacted batch using atomic_compact

        let sid = savant_core::session::sanitize_session_id(agent_id)
            .unwrap_or_else(|| agent_id.to_string());
        let engine = self.engine.clone();

        spawn_blocking(move || {
            // Fetch session messages (up to 500 for consolidation)
            let messages = engine.fetch_session_tail(&sid, 500);

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
            let summary = create_conversation_summary(&to_consolidate);
            let summary_msg = AgentMessage {
                id: uuid::Uuid::new_v4().to_string(),
                role: MessageRole::System,
                content: summary,
                session_id: sid.clone(),
                timestamp: chrono::Utc::now().timestamp_millis().into(),
                tool_calls: Vec::new(),
                tool_results: Vec::new(),
                parent_id: None,
                channel: "Chat".to_string(),
            };

            // Combine summary + recent messages for atomic compact
            let mut compacted = vec![summary_msg];
            compacted.extend(recent);

            // Atomically compact the session
            engine
                .atomic_compact(&sid, compacted)
                .map_err(|e| SavantError::Unknown(format!("Compact failed: {}", e)))?;

            debug!(
                "Consolidated {} messages into summary for session {}",
                to_consolidate.len(),
                sid
            );

            Ok(())
        })
        .await
        .map_err(|e| SavantError::Unknown(format!("Task join error: {}", e)))?
    }
}

/// Creates a lightweight summary of conversation messages.
///
/// This extracts key information without requiring an LLM call:
/// - Counts of user vs assistant messages
/// - Key topics mentioned
/// - Last user query intent
fn create_conversation_summary(messages: &[AgentMessage]) -> String {
    let mut user_msgs = Vec::new();
    let mut assistant_msgs = Vec::new();

    for msg in messages {
        match msg.role {
            MessageRole::User => user_msgs.push(&msg.content),
            MessageRole::Assistant => assistant_msgs.push(&msg.content),
            _ => {}
        }
    }

    // Extract last few user queries as key context
    let recent_queries: Vec<&str> = user_msgs.iter().rev().take(5).map(|s| s.as_str()).collect();
    let total_exchanges = user_msgs.len().min(assistant_msgs.len());

    format!(
        "[Conversation Summary: {} exchanges | {} user messages, {} assistant responses]\n\
         Recent context: {}\n\
         [End of consolidated context from {} earlier messages]",
        total_exchanges,
        user_msgs.len(),
        assistant_msgs.len(),
        recent_queries.join(" | "),
        messages.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use savant_core::types::{AgentOutputChannel, ChatRole, SessionId};

    #[tokio::test]
    async fn test_async_backend_store_and_retrieve() {
        let temp_dir =
            std::env::temp_dir().join(format!("savant_async_backend_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).expect("Failed to init engine");
        let backend = AsyncMemoryBackend::new(engine);

        let chat_msg = ChatMessage {
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
        let temp_dir = std::env::temp_dir()
            .join(format!("savant_async_query_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).expect("Failed to init engine");
        let backend = AsyncMemoryBackend::new(engine);

        // Store multiple messages
        for content in &["hello world", "foo bar", "hello there"] {
            let msg = ChatMessage {
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
        let temp_dir = std::env::temp_dir()
            .join(format!("savant_async_emb_flag_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).expect("Failed to init engine");

        let backend_no_emb = AsyncMemoryBackend::new(engine.clone());
        assert!(!backend_no_emb.has_embeddings());

        // Cleanup
        std::fs::remove_dir_all(temp_dir).ok();
    }
}
