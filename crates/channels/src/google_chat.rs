use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::EventFrame;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct GoogleChatConfig {
    pub auth_token: String,
}

pub struct GoogleChatAdapter {
    config: GoogleChatConfig,
    http: reqwest::Client,
    nexus: Arc<savant_core::bus::NexusBridge>,
}

impl GoogleChatAdapter {
    pub fn new(config: GoogleChatConfig, nexus: Arc<savant_core::bus::NexusBridge>) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            nexus,
        }
    }

    async fn send_text(&self, space_id: &str, text: &str) -> Result<(), SavantError> {
        let resp = self
            .http
            .post(&format!(
                "https://chat.googleapis.com/v1/{}/messages",
                space_id
            ))
            .bearer_auth(&self.config.auth_token)
            .json(&serde_json::json!({ "text": text }))
            .send()
            .await
            .map_err(|e| SavantError::Unknown(e.to_string()))?;
        if !resp.status().is_success() {
            warn!("[GOOGLE_CHAT] Send failed: {}", resp.status());
        }
        Ok(())
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("[GOOGLE_CHAT] Starting Google Chat adapter");
            let (mut rx, _) = self.nexus.subscribe().await;
            while let Ok(event) = rx.recv().await {
                if event.event_type == "chat.message" {
                    if let Ok(p) = serde_json::from_str::<serde_json::Value>(&event.payload) {
                        if p["recipient"]
                            .as_str()
                            .map_or(false, |r| r.starts_with("googlechat:"))
                            || p["role"].as_str() == Some("Assistant")
                        {
                            let sid = p["session_id"].as_str().unwrap_or("");
                            if let Some(space) = sid.strip_prefix("googlechat:") {
                                let text = p["content"].as_str().unwrap_or("");
                                if let Err(e) = self.send_text(space, text).await {
                                    warn!("[GOOGLE_CHAT] {}", e);
                                }
                            }
                        }
                    }
                }
            }
        })
    }
}

#[async_trait]
impl ChannelAdapter for GoogleChatAdapter {
    fn name(&self) -> &str {
        "google_chat"
    }
    async fn send_event(&self, _e: EventFrame) -> Result<(), SavantError> {
        Ok(())
    }
    async fn handle_event(&self, _e: EventFrame) -> Result<(), SavantError> {
        Ok(())
    }
}
