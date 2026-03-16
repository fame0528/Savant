//! Async Backend Adapter
//!
//! This module provides an async implementation of the `MemoryBackend` trait
//! (from `savant_core::traits`) using the synchronous `MemoryEngine`.
//!
//! The adapter spawns blocking tasks on the Tokio runtime to ensure that
//! I/O operations don't block the async executor.

use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::debug;

use crate::engine::MemoryEngine;
use crate::models::AgentMessage;

use savant_core::error::SavantError;
use savant_core::traits::MemoryBackend;
use savant_core::types::ChatMessage;

/// Async wrapper around MemoryEngine that implements the MemoryBackend trait.
///
/// This type is cheap to clone (Arc) and can be shared across tasks.
pub struct AsyncMemoryBackend {
    engine: Arc<MemoryEngine>,
}

impl AsyncMemoryBackend {
    /// Creates a new async backend from a synchronous engine.
    pub fn new(engine: Arc<MemoryEngine>) -> Self {
        Self { engine }
    }

    /// Gets a reference to the underlying engine.
    pub fn engine(&self) -> Arc<MemoryEngine> {
        Arc::clone(&self.engine)
    }
}

#[async_trait::async_trait]
impl MemoryBackend for AsyncMemoryBackend {
    async fn store(&self, agent_id: &str, message: &ChatMessage) -> Result<(), SavantError> {
        let engine = Arc::clone(&self.engine);
        // Clone data to move into the blocking closure (must be 'static)
        let agent_id_owned = agent_id.to_string();
        let message_clone = message.clone();

        // Offload blocking I/O to a spawn_blocking task
        spawn_blocking(move || {
            // Convert ChatMessage -> AgentMessage (internal prioritization logic)
            let agent_msg = AgentMessage::from_chat(&message_clone, &agent_id_owned);
            
            // AAA: Unified Context Harmony - Use the effective session ID for substrate partitioning
            let sid = agent_msg.session_id.clone();

            // Append to transcript
            engine
                .append_message(&sid, &agent_msg)
                .map_err(|e| SavantError::Unknown(e.to_string()))?;

            // If message has embedding, index it for semantic search
            // For now we skip embedding generation - that would be integrated with fastembed
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
        // AAA: Unified Context Harmony - Ensure retrieval uses sanitized ID
        let sid = savant_core::session::sanitize_session_id(agent_id);
        let _query_owned = query.to_string(); // move into blocking task

        let engine = self.engine.clone();
        spawn_blocking(move || {
            // Retrieve recent messages by session_id
            let tail = engine.fetch_session_tail(&sid, limit);

            // Convert AgentMessage -> ChatMessage
            let chat_messages: Vec<ChatMessage> =
                tail.into_iter().map(|msg: AgentMessage| msg.to_chat()).collect();

            Ok(chat_messages)
        })
        .await
        .map_err(|e| SavantError::Unknown(format!("Task join error: {}", e)))?
    }

    async fn consolidate(&self, agent_id: &str) -> Result<(), SavantError> {
        // Consolidation logic (compaction) would go here.
        // For now this is a no-op placeholder. Future implementation:
        // 1. Fetch all messages for session
        // 2. Apply summarization/distillation
        // 3. Call engine.atomic_compact with the new batch
        debug!("Consolidation requested for agent {}", agent_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_backend_store_and_retrieve() {
        // This test requires a temporary storage directory
        let temp_dir = std::env::temp_dir().join("savant_async_backend_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine =
            MemoryEngine::with_defaults(&temp_dir).expect("Failed to init engine");
        let backend = AsyncMemoryBackend::new(engine);

        let chat_msg = ChatMessage {
            role: savant_core::types::ChatRole::User,
            content: "Test message".to_string(),
            sender: None,
            recipient: None,
            agent_id: None,
            session_id: Some(savant_core::types::SessionId("test_session".to_string())),
            channel: savant_core::types::AgentOutputChannel::Chat,
        };

        // Store
        backend.store("test_session", &chat_msg).await.unwrap();

        // Retrieve
        let retrieved = backend.retrieve("test_session", "", 10).await.unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].content, "Test message");

        // Cleanup
        std::fs::remove_dir_all(temp_dir).unwrap();
    }
}
