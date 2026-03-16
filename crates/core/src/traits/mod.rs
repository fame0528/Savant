use crate::error::SavantError;
use crate::types::{ChatChunk, ChatMessage, EventFrame};
pub use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;

/// Channel Adapter Trait
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    /// Retrieve the adapter name.
    fn name(&self) -> &str;

    /// Send an event frame (outbound).
    async fn send_event(&self, event: EventFrame) -> Result<(), SavantError>;

    /// Handle an incoming event (inbound).
    async fn handle_event(&self, event: EventFrame) -> Result<(), SavantError>;
}

/// Language Model Provider Trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Request a chat completion, returning a standardized stream of chunks.
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, SavantError>> + Send>>, SavantError>;
}

/// Memory Backend Trait (LSM-tree / Vector / KV)
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Store a message in persistent session memory.
    async fn store(&self, agent_id: &str, message: &ChatMessage) -> Result<(), SavantError>;

    /// Retrieve relevant context from memory.
    async fn retrieve(
        &self,
        agent_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ChatMessage>, SavantError>;

    /// Finalize and optimize memory state.
    async fn consolidate(&self, agent_id: &str) -> Result<(), SavantError>;
}

#[async_trait]
impl<M: MemoryBackend + ?Sized> MemoryBackend for std::sync::Arc<M> {
    async fn store(&self, agent_id: &str, message: &ChatMessage) -> Result<(), SavantError> {
        self.as_ref().store(agent_id, message).await
    }

    async fn retrieve(
        &self,
        agent_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ChatMessage>, SavantError> {
        self.as_ref().retrieve(agent_id, query, limit).await
    }

    async fn consolidate(&self, agent_id: &str) -> Result<(), SavantError> {
        self.as_ref().consolidate(agent_id).await
    }
}

/// Tool/Capability Trait (OpenClaw/ZeroClaw compatible)
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name.
    fn name(&self) -> &str;

    /// Detailed description for LLM guidance.
    fn description(&self) -> &str;

    /// Explicit capabilities granted to this tool.
    fn capabilities(&self) -> crate::types::CapabilityGrants {
        crate::types::CapabilityGrants::default()
    }

    /// Execute the tool with a JSON payload.
    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError>;
}

/// OMEGA-VII: Symbolic Browser Projection Trait
/// 
/// Decouples browser interaction from mutable state, allowing for 
/// Intent-Substrate Coherence (ISC) verification.
#[async_trait]
pub trait SymbolicBrowser: Send + Sync {
    /// Projects the current DOM into a symbolic representation.
    async fn project_dom(&self) -> Result<serde_json::Value, SavantError>;

    /// Proves that a browser action matches the intended cognitive outcome.
    async fn prove_intent_coherence(
        &self, 
        action: &str, 
        selector: &str, 
        intent_matrix: serde_json::Value
    ) -> Result<bool, SavantError>;

    /// Executes the action on the substrate only after verification.
    async fn execute_verified(&self, action: serde_json::Value) -> Result<String, SavantError>;
}
