use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::{ChatMessage, ChatRole, EventFrame};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// X (formerly Twitter) channel configuration.
#[derive(Debug, Clone)]
pub struct XConfig {
    pub bearer_token: String,
}

/// X (formerly Twitter) channel adapter.
/// Supports posting tweets and polling DMs.
pub struct XAdapter {
    config: XConfig,
    http: reqwest::Client,
    nexus: Arc<savant_core::bus::NexusBridge>,
}

impl XAdapter {
    pub fn new(config: XConfig, nexus: Arc<savant_core::bus::NexusBridge>) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            nexus,
        }
    }

    /// Posts a tweet.
    async fn post_tweet(&self, text: &str) -> Result<(), SavantError> {
        let resp = self
            .http
            .post("https://api.twitter.com/2/tweets")
            .bearer_auth(&self.config.bearer_token)
            .json(&serde_json::json!({"text": text}))
            .send()
            .await
            .map_err(|e| SavantError::Unknown(format!("X post failed: {}", e)))?;

        if !resp.status().is_success() {
            warn!("[X] Post failed: {}", resp.status());
        }
        Ok(())
    }

    /// Fetches recent DMs.
    async fn fetch_dms(&self) -> Result<Vec<serde_json::Value>, SavantError> {
        let resp: serde_json::Value = self
            .http
            .get("https://api.twitter.com/2/dm_conversations/with/messages")
            .bearer_auth(&self.config.bearer_token)
            .send()
            .await
            .map_err(|e| SavantError::Unknown(format!("X DM fetch failed: {}", e)))?
            .json()
            .await
            .map_err(|e| SavantError::Unknown(format!("X DM parse failed: {}", e)))?;

        Ok(resp["data"].as_array().cloned().unwrap_or_default())
    }

    /// Sends a DM.
    async fn send_dm(&self, recipient_id: &str, text: &str) -> Result<(), SavantError> {
        let resp = self
            .http
            .post("https://api.twitter.com/2/dm_conversations/with/messages")
            .bearer_auth(&self.config.bearer_token)
            .json(&serde_json::json!({
                "event": { "type": "message_create",
                    "message_create": { "target": { "recipient_id": recipient_id },
                        "message_data": { "text": text } } }
            }))
            .send()
            .await
            .map_err(|e| SavantError::Unknown(format!("X DM send failed: {}", e)))?;

        if !resp.status().is_success() {
            warn!("[X] DM send failed: {}", resp.status());
        }
        Ok(())
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("[X] Starting X adapter");
            let (mut event_rx, _) = self.nexus.subscribe().await;
            let adapter = Arc::new(self);

            // Outbound listener
            let outbound = adapter.clone();
            tokio::spawn(async move {
                while let Ok(event) = event_rx.recv().await {
                    if event.event_type == "chat.message" {
                        if let Ok(p) = serde_json::from_str::<serde_json::Value>(&event.payload) {
                            let is_for = p["recipient"]
                                .as_str()
                                .map_or(false, |r| r.starts_with("x:"));
                            let is_assistant = p["role"].as_str() == Some("Assistant");
                            if is_assistant || is_for {
                                let content = p["content"].as_str().unwrap_or("");
                                let sid = p["session_id"].as_str().unwrap_or("");
                                if let Some(target) = sid.strip_prefix("x:") {
                                    if target == "post" {
                                        let _ = outbound.post_tweet(content).await;
                                    } else {
                                        let _ = outbound.send_dm(target, content).await;
                                    }
                                }
                            }
                        }
                    }
                }
            });

            // DM polling loop
            loop {
                match adapter.fetch_dms().await {
                    Ok(dms) => {
                        for dm in dms {
                            let sender = dm["sender_id"].as_str().unwrap_or("unknown");
                            let text = dm["text"].as_str().unwrap_or("");
                            if !text.is_empty() {
                                let sid = savant_core::session::SessionMapper::map("x", sender);
                                let msg = ChatMessage {
                                    is_telemetry: false,
                                    role: ChatRole::User,
                                    content: text.to_string(),
                                    sender: Some(format!("x:{}", sender)),
                                    recipient: Some("savant".into()),
                                    agent_id: None,
                                    session_id: Some(sid),
                                    channel: savant_core::types::AgentOutputChannel::Chat,
                                };
                                let frame = EventFrame {
                                    event_type: "chat.message".into(),
                                    payload: serde_json::to_string(&msg).unwrap_or_default(),
                                };
                                let _ = adapter.nexus.event_bus.send(frame);
                            }
                        }
                    }
                    Err(e) => warn!("[X] DM poll error: {}", e),
                }
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        })
    }
}

#[async_trait]
impl ChannelAdapter for XAdapter {
    fn name(&self) -> &str {
        "x"
    }
    async fn send_event(&self, event: EventFrame) -> Result<(), SavantError> {
        if event.event_type == "message.send" {
            if let Ok(p) = serde_json::from_str::<serde_json::Value>(&event.payload) {
                let text = p["text"].as_str().unwrap_or("");
                let target = p["target"].as_str().unwrap_or("post");
                if target == "post" {
                    return self.post_tweet(text).await;
                } else {
                    return self.send_dm(target, text).await;
                }
            }
        }
        Ok(())
    }
    async fn handle_event(&self, event: EventFrame) -> Result<(), SavantError> {
        self.send_event(event).await
    }
}
