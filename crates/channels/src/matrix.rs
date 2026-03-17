use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::EventFrame;
use matrix_sdk::{Client, config::SyncSettings};
use std::sync::Arc;
use tracing::{info, error, warn};
use async_trait::async_trait;

use crate::pool::InboxPool;

/// MatrixAdapter provides an industrial-grade bridge to the Matrix network.
/// Supports E2EE, multi-room orchestration, and swarm-wide event routing.
pub struct MatrixAdapter {
    client: Arc<Client>,
}

impl MatrixAdapter {
    /// Creates a new MatrixAdapter and starts the sync loop.
    pub async fn new(homeserver: &str, access_token: &str, user_id: &str) -> Result<Self, SavantError> {
        let client = Client::builder()
            .homeserver_url(homeserver)
            .build()
            .await
            .map_err(|e| SavantError::AuthError(format!("Matrix builder error: {}", e)))?;

        let user_id = matrix_sdk::ruma::UserId::parse(user_id)
            .map_err(|e| SavantError::AuthError(format!("Invalid UserID: {}", e)))?;

        let session = matrix_sdk::Session {
            access_token: access_token.to_string(),
            refresh_token: None,
            user_id,
            device_id: "SAVANT-NODE".into(),
        };

        client.restore_session(session).await.map_err(|e| SavantError::AuthError(format!("Matrix session restoration failed: {}", e)))?;

        let adapter = Self {
            client: Arc::new(client),
        };

        Ok(adapter)
    }

    /// Starts the background sync loop for the Matrix client.
    pub fn spawn_sync_worker(&self, pool: Arc<InboxPool>) {
        let client = self.client.clone();
        tokio::spawn(async move {
            info!("Matrix sync worker ignited.");
            
            // Add a message event handler
            client.add_event_handler(move |ev: matrix_sdk::event_handler::Ctx<matrix_sdk::ruma::events::room::message::SyncRoomMessageEvent>| {
                let pool = pool.clone();
                async move {
                    if let matrix_sdk::ruma::events::room::message::MessageType::Text(text) = &ev.content.msgtype {
                        info!("Matrix message received ({}): {}", ev.sender, text.body);
                        
                        let event = EventFrame {
                            event_type: "matrix_message".to_string(),
                            payload: format!("{}: {}", ev.sender, text.body),
                        };
                        
                        pool.submit_inbound(event).await;
                    }
                }
            });

            if let Err(e) = client.sync(SyncSettings::default()).await {
                error!("Matrix sync loop fatal error: {}", e);
            }
        });
    }
}

#[async_trait]
impl ChannelAdapter for MatrixAdapter {
    fn name(&self) -> &str {
        "matrix"
    }

    async fn send_event(&self, event: EventFrame) -> Result<(), SavantError> {
        info!("Matrix processing outbound event: {}", event.event_type);
        
        match event.event_type.as_str() {
            "message.send" => {
                let payload: serde_json::Value = serde_json::from_str(&event.payload)
                    .map_err(|e| SavantError::InvalidInput(format!("Invalid Matrix payload: {}", e)))?;
                
                let room_id_str = payload.get("room_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SavantError::InvalidInput("Missing room_id in Matrix payload".into()))?;
                
                let text = payload.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SavantError::InvalidInput("Missing text in Matrix payload".into()))?;

                let room_id = <&matrix_sdk::ruma::RoomId>::try_from(room_id_str)
                    .map_err(|e| SavantError::InvalidInput(format!("Invalid Matrix RoomID: {}", e)))?;

                if let Some(room) = self.client.get_room(room_id) {
                    use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
                    let content = RoomMessageEventContent::text_plain(text);
                    room.send(content).await
                        .map_err(|e| SavantError::Unknown(format!("Failed to send Matrix message: {}", e)))?;
                    info!("Matrix message sent to {}", room_id_str);
                } else {
                    return Err(SavantError::Unknown(format!("Room not found or not joined: {}", room_id_str)));
                }
            }
            _ => {
                warn!("Matrix: Unhandled event type for send_event: {}", event.event_type);
            }
        }
        Ok(())
    }

    async fn handle_event(&self, event: EventFrame) -> Result<(), SavantError> {
        self.send_event(event).await
    }
}
