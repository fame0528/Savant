use crate::error::SavantError;
use crate::traits::MemoryBackend;
use crate::types::ChatMessage;
use std::path::Path;
use tracing::info;

pub use savant_memory::MemoryEngine as SavantMemoryEngine;

/// Implementation of MemoryBackend using SavantMemoryEngine (Fjall).
pub struct FjallMemoryBackend {
    engine: SavantMemoryEngine,
}

impl FjallMemoryBackend {
    /// Creates a new memory backend with the given storage path.
    pub fn new(storage_path: impl AsRef<Path>) -> Result<Self, SavantError> {
        let engine = SavantMemoryEngine::with_defaults(storage_path)
            .map_err(|e| SavantError::Unknown(format!("Fjall init failed: {}", e)))?;
        Ok(Self { engine })
    }
}

#[async_trait::async_trait]
impl MemoryBackend for FjallMemoryBackend {
    async fn store(&self, agent_id: &str, message: &ChatMessage) -> Result<(), SavantError> {
        let agent_msg = savant_memory::AgentMessage::from_chat(message, agent_id);
        self.engine
            .append_message(agent_id, &agent_msg)
            .map_err(|e| SavantError::Unknown(e.to_string()))?;
        Ok(())
    }

    async fn retrieve(
        &self,
        agent_id: &str,
        _query: &str,
        limit: usize,
    ) -> Result<Vec<ChatMessage>, SavantError> {
        let messages = self.engine.fetch_session_tail(agent_id, limit);
        let chat_messages: Vec<ChatMessage> = messages
            .into_iter()
            .map(|msg| msg.to_chat())
            .collect::<Vec<ChatMessage>>();
        Ok(chat_messages)
    }

    async fn consolidate(&self, agent_id: &str) -> Result<(), SavantError> {
        info!("Consolidation requested for agent {}", agent_id);
        Ok(())
    }
}
