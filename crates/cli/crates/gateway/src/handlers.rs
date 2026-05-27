//! ControlFrame handlers for gateway communication

use anyhow::Result;
use savant_core::types::{ChatChunk, ChatMessage, EventFrame, RequestFrame};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Handler for incoming control frames and responses
#[derive(Clone)]
pub struct ControlFrameHandler {
    pending_requests: Arc<RwLock<HashMap<String, PendingRequest>>>,
}

struct PendingRequest {
    control_type: String,
    callback: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
}

impl ControlFrameHandler {
    pub fn new() -> Self {
        Self {
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a pending request with a callback
    pub async fn register_request<F>(&self, request_id: String, control_type: String, callback: F)
    where
        F: Fn(serde_json::Value) + Send + Sync + 'static,
    {
        let mut requests = self.pending_requests.write().await;
        requests.insert(
            request_id,
            PendingRequest {
                control_type,
                callback: Some(Box::new(callback)),
            },
        );
    }

    /// Handle an incoming response frame
    pub async fn handle_response(&self, frame: RequestFrame) -> Result<()> {
        debug!("Received response for request_id: {}", frame.request_id);

        let mut requests = self.pending_requests.write().await;
        if let Some(pending) = requests.remove(&frame.request_id) {
            debug!("Matched pending request: type={}", pending.control_type);
            if let Some(callback) = pending.callback {
                match serde_json::to_value(&frame.payload) {
                    Ok(value) => callback(value),
                    Err(e) => {
                        warn!("Failed to serialize payload: {}", e);
                        callback(serde_json::Value::String(format!(
                            "serialization error: {}",
                            e
                        )));
                    }
                }
            }
        } else {
            debug!(
                "No pending request found for request_id: {}",
                frame.request_id
            );
        }

        Ok(())
    }

    /// Handle a chat message event
    pub async fn handle_chat_message(&self, message: ChatMessage) {
        info!(
            "[{:?}] {:?}: {:?}",
            message.agent_id, message.role, message.content
        );
    }

    /// Handle a chat chunk event (streaming)
    pub async fn handle_chat_chunk(&self, chunk: ChatChunk) {
        if chunk.is_final {
            debug!(
                "[{}] Stream complete for session {:?}",
                chunk.agent_name, chunk.session_id
            );
        }
    }

    /// Handle an evolution event
    pub async fn handle_evolution_event(&self, event: EventFrame) {
        match event.event_type.as_str() {
            "system.evolution.mutation_proposed" => {
                info!("New mutation proposed: {}", event.payload);
            }
            "system.evolution.mutation_applied" => {
                info!("Mutation applied: {}", event.payload);
            }
            _ => {
                debug!("Unknown evolution event: {}", event.event_type);
            }
        }
    }
}

impl Default for ControlFrameHandler {
    fn default() -> Self {
        Self::new()
    }
}
