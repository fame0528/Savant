use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::{ChatMessage, ChatRole, EventFrame};
use std::sync::Arc;
use tracing::{info, warn};

/// Nostr channel configuration.
#[derive(Debug, Clone)]
pub struct NostrConfig {
    pub relays: Vec<String>,
    pub private_key: Option<String>, // hex-encoded nsec
}

/// Nostr channel adapter.
/// Connects to Nostr relays via WebSocket, publishes text notes (kind 1).
pub struct NostrAdapter {
    config: NostrConfig,
    nexus: Arc<savant_core::bus::NexusBridge>,
}

impl NostrAdapter {
    pub fn new(config: NostrConfig, nexus: Arc<savant_core::bus::NexusBridge>) -> Self {
        Self { config, nexus }
    }

    /// Publishes a text note (kind 1) to all configured relays.
    async fn publish_note(&self, text: &str) -> Result<(), SavantError> {
        // NIP-01 event structure
        let event = serde_json::json!({
            "kind": 1,
            "content": text,
            "created_at": chrono::Utc::now().timestamp(),
            "tags": [],
            "pubkey": "",  // Would be derived from private_key
            "id": "",      // Would be computed from content hash
            "sig": "",     // Would be signed with private_key
        });

        let msg = serde_json::json!(["EVENT", event]);

        for relay in &self.config.relays {
            match tokio_tungstenite::connect_async(relay).await {
                Ok((ws, _)) => {
                    let (mut write, _) = ws.split();
                    if let Err(e) = write
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            msg.to_string(),
                        ))
                        .await
                    {
                        warn!("[NOSTR] Failed to publish to {}: {}", relay, e);
                    }
                }
                Err(e) => warn!("[NOSTR] Failed to connect to {}: {}", relay, e),
            }
        }
        Ok(())
    }

    /// Subscribes to a relay for incoming messages.
    async fn subscribe_relay(relay: &str, nexus: Arc<savant_core::bus::NexusBridge>) {
        match tokio_tungstenite::connect_async(relay).await {
            Ok((ws, _)) => {
                let (mut write, mut read) = ws.split();

                // Subscribe to text notes
                let sub = serde_json::json!(["REQ", "savant", {"kinds": [1], "limit": 10}]);
                let _ = write
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        sub.to_string(),
                    ))
                    .await;

                use futures::StreamExt;
                while let Some(Ok(msg)) = read.next().await {
                    if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                            if parsed[0].as_str() == Some("EVENT") {
                                let event = &parsed[2];
                                let pubkey = event["pubkey"].as_str().unwrap_or("unknown");
                                let content = event["content"].as_str().unwrap_or("");
                                if !content.is_empty() {
                                    let sid =
                                        savant_core::session::SessionMapper::map("nostr", pubkey);
                                    let chat_msg = ChatMessage {
                                        is_telemetry: false,
                                        role: ChatRole::User,
                                        content: content.to_string(),
                                        sender: Some(format!("nostr:{}", pubkey)),
                                        recipient: Some("savant".into()),
                                        agent_id: None,
                                        session_id: Some(sid),
                                        channel: savant_core::types::AgentOutputChannel::Chat,
                                    };
                                    let frame = EventFrame {
                                        event_type: "chat.message".into(),
                                        payload: serde_json::to_string(&chat_msg)
                                            .unwrap_or_default(),
                                    };
                                    let _ = nexus.event_bus.send(frame);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => warn!("[NOSTR] Subscribe failed for {}: {}", relay, e),
        }
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!(
                "[NOSTR] Starting Nostr adapter (relays: {:?})",
                self.config.relays
            );

            // Subscribe to all relays
            for relay in &self.config.relays {
                let nexus = self.nexus.clone();
                let relay = relay.clone();
                tokio::spawn(async move {
                    Self::subscribe_relay(&relay, nexus).await;
                });
            }

            // Outbound listener
            let (mut event_rx, _) = self.nexus.subscribe().await;
            while let Ok(event) = event_rx.recv().await {
                if event.event_type == "chat.message" {
                    if let Ok(p) = serde_json::from_str::<serde_json::Value>(&event.payload) {
                        if p["recipient"]
                            .as_str()
                            .map_or(false, |r| r == "nostr:broadcast")
                            || p["role"].as_str() == Some("Assistant")
                        {
                            let text = p["content"].as_str().unwrap_or("");
                            if let Err(e) = self.publish_note(text).await {
                                warn!("[NOSTR] Publish error: {}", e);
                            }
                        }
                    }
                }
            }
        })
    }
}

#[async_trait]
impl ChannelAdapter for NostrAdapter {
    fn name(&self) -> &str {
        "nostr"
    }
    async fn send_event(&self, _e: EventFrame) -> Result<(), SavantError> {
        Ok(())
    }
    async fn handle_event(&self, _e: EventFrame) -> Result<(), SavantError> {
        Ok(())
    }
}
