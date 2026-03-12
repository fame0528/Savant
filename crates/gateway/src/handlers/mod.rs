use crate::auth::AuthenticatedSession;
use savant_core::types::{RequestFrame, ChatMessage, ChatRole};
use savant_core::bus::NexusBridge;
use std::sync::Arc;

pub mod pairing;

/// Shared application state for axum handlers.
pub struct AppState {
    pub nexus: Arc<NexusBridge>,
    pub storage: Arc<savant_core::db::Storage>,
}

/// Handles an incoming WebSocket message frame based on session.
pub async fn handle_message(frame: RequestFrame, session: AuthenticatedSession, state: axum::extract::State<AppState>) {
    tracing::info!("📨 Processing message from session: {:?}", session.session_id);
    
    // Parse the incoming message as JSON
    let chat_message: Result<ChatMessage, _> = serde_json::from_str(&frame.payload);
    
    match chat_message {
        Ok(message) => {
            tracing::info!("💬 Chat message: {:?} - {}", message.role, message.content);
            
            // Perfection Loop Enhancement: Route persistence to specific agent channel
            let channel = message.agent_id.as_deref()
                .or(message.recipient.as_deref())
                .unwrap_or("global");

            if let Err(e) = state.storage.append_chat(channel, &message) {
                tracing::error!("❌ Failed to persist chat message to {}: {}", channel, e);
            }

            // Route message to appropriate agent through Nexus
            if let Err(e) = route_chat_message(message, &state.nexus).await {
                tracing::error!("❌ Failed to route chat message: {}", e);
                
                // Send error response back to client
                let error_response = ChatMessage {
                    role: ChatRole::System,
                    content: format!("Error: {}", e),
                    sender: Some("SYSTEM".to_string()),
                    recipient: None,
                    agent_id: Some("SYSTEM".to_string()),
                };
                
                if let Err(send_err) = send_response_to_client(error_response, &session.session_id, &state.nexus).await {
                    tracing::error!("❌ Failed to send error response: {}", send_err);
                }
            }
        }
        Err(e) => {
            tracing::error!("❌ Invalid chat message format: {}", e);
            
            // Send error response
            let error_response = ChatMessage {
                role: ChatRole::System,
                content: "Invalid message format. Please send valid JSON.".to_string(),
                sender: Some("SYSTEM".to_string()),
                recipient: None,
                agent_id: Some("SYSTEM".to_string()),
            };
            
            if let Err(send_err) = send_response_to_client(error_response, &session.session_id, &state.nexus).await {
                tracing::error!("❌ Failed to send format error response: {}", send_err);
            }
        }
    }
}

/// Routes chat message to appropriate agent
async fn route_chat_message(message: ChatMessage, nexus: &Arc<NexusBridge>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // For now, broadcast to all available agents
    // In production, this would route to specific agents based on logic
    let event_payload = serde_json::to_string(&message)?;
    
    nexus.publish("chat.message", &event_payload).await?;
    
    tracing::info!("📤 Chat message routed to agents");
    Ok(())
}

/// Sends response back to client session
async fn send_response_to_client(
    response: ChatMessage, 
    session_id: &savant_core::types::SessionId, 
    nexus: &Arc<NexusBridge>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response_payload = serde_json::to_string(&response)?;
    
    // Send to specific client session
    nexus.publish(&format!("session.{}.response", session_id.0), &response_payload).await?;
    
    tracing::info!("📤 Response sent to client session: {:?}", session_id);
    Ok(())
}

#[cfg(test)]
mod benches {
    // criterion benchmark stub
}
