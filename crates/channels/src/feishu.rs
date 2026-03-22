use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::{ChatMessage, ChatRole, EventFrame};
use std::sync::Arc;
use std::time::Duration;

use tracing::{debug, info, warn};

/// Feishu/Lark channel configuration.
#[derive(Debug, Clone)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,
    pub verification_token: String,
}

/// Feishu/Lark channel adapter.
/// Communicates via Feishu Open Platform webhook + REST API.
pub struct FeishuAdapter {
    config: FeishuConfig,
    http: reqwest::Client,
    nexus: Arc<savant_core::bus::NexusBridge>,
    tenant_token: Arc<tokio::sync::Mutex<Option<String>>>,
}

impl FeishuAdapter {
    pub fn new(config: FeishuConfig, nexus: Arc<savant_core::bus::NexusBridge>) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            nexus,
            tenant_token: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Gets or refreshes the tenant_access_token.
    async fn get_token(&self) -> Result<String, SavantError> {
        {
            let lock = self.tenant_token.lock().await;
            if let Some(ref token) = *lock {
                return Ok(token.clone());
            }
        }

        let resp: serde_json::Value = self
            .http
            .post("https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal")
            .json(&serde_json::json!({
                "app_id": self.config.app_id,
                "app_secret": self.config.app_secret,
            }))
            .send()
            .await
            .map_err(|e| SavantError::Unknown(format!("Feishu token request failed: {}", e)))?
            .json()
            .await
            .map_err(|e| SavantError::Unknown(format!("Feishu token parse failed: {}", e)))?;

        let token = resp["tenant_access_token"]
            .as_str()
            .ok_or_else(|| {
                SavantError::Unknown("No tenant_access_token in Feishu response".into())
            })?
            .to_string();

        let mut lock = self.tenant_token.lock().await;
        *lock = Some(token.clone());
        Ok(token)
    }

    /// Sends a text message to a chat.
    async fn send_text(&self, chat_id: &str, text: &str) -> Result<(), SavantError> {
        let token = self.get_token().await?;

        let resp: serde_json::Value = self
            .http
            .post("https://open.feishu.cn/open-apis/im/v1/messages")
            .query(&[("receive_id_type", "chat_id")])
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "receive_id": chat_id,
                "msg_type": "text",
                "content": serde_json::json!({ "text": text }).to_string(),
            }))
            .send()
            .await
            .map_err(|e| SavantError::Unknown(format!("Feishu send failed: {}", e)))?
            .json()
            .await
            .map_err(|e| SavantError::Unknown(format!("Feishu response parse failed: {}", e)))?;

        if resp["code"].as_i64() != Some(0) {
            warn!("Feishu send error: {}", resp);
        }
        Ok(())
    }

    /// Polls for messages via long-polling endpoint.
    async fn poll_messages(&self) -> Result<Vec<serde_json::Value>, SavantError> {
        let token = self.get_token().await?;

        let resp: serde_json::Value = self
            .http
            .get("https://open.feishu.cn/open-apis/im/v1/messages")
            .bearer_auth(&token)
            .query(&[
                ("container_id_type", "chat"),
                ("container_id", ""),
                ("page_size", "20"),
            ])
            .send()
            .await
            .map_err(|e| SavantError::Unknown(format!("Feishu poll failed: {}", e)))?
            .json()
            .await
            .map_err(|e| SavantError::Unknown(format!("Feishu poll parse failed: {}", e)))?;

        Ok(resp["data"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }

    /// Starts the background polling + outbound loop.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("[FEISHU] Starting Feishu adapter");

            // Subscribe to Nexus for outbound messages
            let (mut event_rx, _) = self.nexus.subscribe().await;
            let nexus_out = self.nexus.clone();
            let http_out = self.http.clone();
            let _config_out = self.config.clone();
            let token_out = self.tenant_token.clone();

            // Outbound listener
            tokio::spawn(async move {
                while let Ok(event) = event_rx.recv().await {
                    if event.event_type == "chat.message" {
                        if let Ok(payload) =
                            serde_json::from_str::<serde_json::Value>(&event.payload)
                        {
                            let is_assistant = payload["role"].as_str() == Some("Assistant");
                            let is_for_feishu = payload["recipient"]
                                .as_str()
                                .map_or(false, |r| r.starts_with("feishu:"));
                            if is_assistant || is_for_feishu {
                                let session_id = payload["session_id"].as_str().unwrap_or("");
                                if let Some(chat_id) = session_id.strip_prefix("feishu:") {
                                    let text = payload["content"].as_str().unwrap_or("");
                                    // Inline send to avoid borrowing issues
                                    let resp = http_out
                                        .post("https://open.feishu.cn/open-apis/im/v1/messages")
                                        .query(&[("receive_id_type", "chat_id")])
                                        .bearer_auth(token_out.lock().await.as_deref().unwrap_or(""))
                                        .json(&serde_json::json!({
                                            "receive_id": chat_id,
                                            "msg_type": "text",
                                            "content": serde_json::json!({ "text": text }).to_string(),
                                        }))
                                        .send()
                                        .await;
                                    if let Err(e) = resp {
                                        warn!("[FEISHU] Failed to send: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            });

            // Inbound polling loop
            loop {
                match self.poll_messages().await {
                    Ok(messages) => {
                        for msg in messages {
                            let sender = msg["sender"]["sender_id"]["open_id"]
                                .as_str()
                                .unwrap_or("unknown");
                            let chat_id = msg["chat_id"].as_str().unwrap_or("unknown");
                            let content = msg["body"]["content"].as_str().unwrap_or("");

                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) {
                                let text = parsed["text"].as_str().unwrap_or(content);
                                if !text.is_empty() {
                                    let session_id =
                                        savant_core::session::SessionMapper::map("feishu", chat_id);
                                    let chat_msg = ChatMessage {
                                        is_telemetry: false,
                                        role: ChatRole::User,
                                        content: text.to_string(),
                                        sender: Some(format!("feishu:{}", sender)),
                                        recipient: Some("savant".to_string()),
                                        agent_id: None,
                                        session_id: Some(session_id),
                                        channel: savant_core::types::AgentOutputChannel::Chat,
                                    };
                                    let frame = EventFrame {
                                        event_type: "chat.message".to_string(),
                                        payload: serde_json::to_string(&chat_msg)
                                            .unwrap_or_default(),
                                    };
                                    let _ = nexus_out.event_bus.send(frame);
                                }
                            }
                        }
                    }
                    Err(e) => warn!("[FEISHU] Poll error: {}", e),
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        })
    }
}

#[async_trait]
impl ChannelAdapter for FeishuAdapter {
    fn name(&self) -> &str {
        "feishu"
    }

    async fn send_event(&self, event: EventFrame) -> Result<(), SavantError> {
        if event.event_type == "message.send" {
            if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&event.payload) {
                let chat_id = payload["chat_id"].as_str().unwrap_or("");
                let text = payload["text"].as_str().unwrap_or("");
                if !chat_id.is_empty() && !text.is_empty() {
                    return self.send_text(chat_id, text).await;
                }
            }
        }
        Ok(())
    }

    async fn handle_event(&self, event: EventFrame) -> Result<(), SavantError> {
        debug!("[FEISHU] Handling event: {}", event.event_type);
        self.send_event(event).await
    }
}
