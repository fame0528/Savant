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
use crate::models::{AgentMessage, MessageRole};

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
        let sid = savant_core::session::sanitize_session_id(agent_id)
            .unwrap_or_else(|| agent_id.to_string());
        let query_owned = query.to_string();

        let engine = self.engine.clone();
        spawn_blocking(move || {
            // Retrieve recent messages by session_id
            let tail = engine.fetch_session_tail(&sid, limit);

            // Convert AgentMessage -> ChatMessage
            let chat_messages: Vec<ChatMessage> = tail
                .into_iter()
                .map(|msg: AgentMessage| msg.to_chat())
                .collect();

            // Filter by query if non-empty (case-insensitive substring match)
            if query_owned.is_empty() {
                Ok(chat_messages)
            } else {
                let query_lower = query_owned.to_lowercase();
                Ok(chat_messages
                    .into_iter()
                    .filter(|msg| msg.content.to_lowercase().contains(&query_lower))
                    .collect())
            }
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

    #[tokio::test]
    async fn test_async_backend_store_and_retrieve() {
        // This test requires a temporary storage directory
        let temp_dir = std::env::temp_dir().join("savant_async_backend_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let engine = MemoryEngine::with_defaults(&temp_dir).expect("Failed to init engine");
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
