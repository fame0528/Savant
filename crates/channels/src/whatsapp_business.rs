use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::EventFrame;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct WhatsAppBusinessConfig {
    pub phone_number_id: String,
    pub access_token: String,
    pub webhook_verify_token: String,
}

pub struct WhatsAppBusinessAdapter {
    config: WhatsAppBusinessConfig,
    http: reqwest::Client,
    nexus: Arc<savant_core::bus::NexusBridge>,
}

impl WhatsAppBusinessAdapter {
    pub fn new(config: WhatsAppBusinessConfig, nexus: Arc<savant_core::bus::NexusBridge>) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            nexus,
        }
    }

    async fn send_text(&self, to: &str, text: &str) -> Result<(), SavantError> {
        let resp = self
            .http
            .post(&format!(
                "https://graph.facebook.com/v18.0/{}/messages",
                self.config.phone_number_id
            ))
            .bearer_auth(&self.config.access_token)
            .json(&serde_json::json!({
                "messaging_product": "whatsapp",
                "to": to,
                "type": "text",
                "text": { "body": text }
            }))
            .send()
            .await
            .map_err(|e| SavantError::Unknown(e.to_string()))?;
        if !resp.status().is_success() {
            warn!("[WABIZ] Send failed: {}", resp.status());
        }
        Ok(())
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("[WABIZ] Starting WhatsApp Business adapter");
            let (mut rx, _) = self.nexus.subscribe().await;
            while let Ok(event) = rx.recv().await {
                if event.event_type == "chat.message" {
                    if let Ok(p) = serde_json::from_str::<serde_json::Value>(&event.payload) {
                        if p["recipient"]
                            .as_str()
                            .map_or(false, |r| r.starts_with("wabiz:"))
                            || p["role"].as_str() == Some("Assistant")
                        {
                            let sid = p["session_id"].as_str().unwrap_or("");
                            if let Some(to) = sid.strip_prefix("wabiz:") {
                                let text = p["content"].as_str().unwrap_or("");
                                if let Err(e) = self.send_text(to, text).await {
                                    warn!("[WABIZ] {}", e);
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
impl ChannelAdapter for WhatsAppBusinessAdapter {
    fn name(&self) -> &str {
        "whatsapp_business"
    }
    async fn send_event(&self, _event: EventFrame) -> Result<(), SavantError> {
        Ok(())
    }
    async fn handle_event(&self, _event: EventFrame) -> Result<(), SavantError> {
        Ok(())
    }
}
