use savant_core::types::ChatMessage;
use savant_core::db::Storage;
use std::sync::Arc;
use tracing::{debug, error};

/// 🌀 GatewayPersistence: Logic for aligning multi-channel streams with UCH anchors.
pub struct GatewayPersistence;

impl GatewayPersistence {
    /// Determines the correct partition for a ChatMessage and persists it.
    /// AAA: Unified Context Harmony ensures that session_id always takes precedence.
    pub async fn persist_chat(storage: &Arc<Storage>, msg: &ChatMessage) -> Result<(), savant_core::error::SavantError> {
        // 🛡️ UCH Precedence: session_id > agent_id > sender > recipient
        let partition = if let Some(sid) = &msg.session_id {
            sid.0.clone()
        } else if let Some(aid) = &msg.agent_id {
            aid.clone()
        } else if let Some(sender) = &msg.sender {
            sender.clone()
        } else if let Some(recipient) = &msg.recipient {
            recipient.clone()
        } else {
            "global".to_string()
        };

        // Sanitize partition key for filesystem safety
        let partition = savant_core::session::sanitize_session_id(&partition);

        debug!(partition = %partition, "Persisting message to substrate");
        
        storage.append_chat(&partition, msg).await
            .map_err(|e| {
                error!(error = %e, "Substrate write failure");
                savant_core::error::SavantError::Unknown(e.to_string())
            })
    }
}
