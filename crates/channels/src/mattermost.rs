use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::{ChatMessage, ChatRole, EventFrame};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct MattermostConfig {
    pub server_url: String,
    pub token: String,
    pub channel_id: Option<String>,
}

pub struct MattermostAdapter {
    config: MattermostConfig,
    http: reqwest::Client,
    nexus: Arc<savant_core::bus::NexusBridge>,
}

impl MattermostAdapter {
    pub fn new(config: MattermostConfig, nexus: Arc<savant_core::bus::NexusBridge>) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            nexus,
        }
    }

    async fn send_text(&self, channel_id: &str, text: &str) -> Result<(), SavantError> {
        let resp = self
            .http
            .post(&format!("{}/api/v4/posts", self.config.server_url))
            .bearer_auth(&self.config.token)
            .json(&serde_json::json!({"channel_id": channel_id, "message": text}))
            .send()
            .await
            .map_err(|e| SavantError::Unknown(e.to_string()))?;
        if !resp.status().is_success() {
            warn!("[MATTERMOST] Send failed: {}", resp.status());
        }
        Ok(())
    }

    async fn poll_posts(&self, channel_id: &str) -> Result<Vec<serde_json::Value>, SavantError> {
        let resp: serde_json::Value = self
            .http
            .get(&format!(
                "{}/api/v4/channels/{}/posts",
                self.config.server_url, channel_id
            ))
            .bearer_auth(&self.config.token)
            .query(&[("per_page", "20")])
            .send()
            .await
            .map_err(|e| SavantError::Unknown(e.to_string()))?
            .json()
            .await
            .map_err(|e| SavantError::Unknown(e.to_string()))?;
        let posts = resp["order"].as_array().cloned().unwrap_or_default();
        let mut result = Vec::new();
        for id in posts {
            if let Some(id_str) = id.as_str() {
                if let Some(post) = resp["posts"].get(id_str) {
                    result.push(post.clone());
                }
            }
        }
        Ok(result)
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("[MATTERMOST] Starting Mattermost adapter");
            let (mut event_rx, _) = self.nexus.subscribe().await;
            let adapter = Arc::new(self);

            let send_adapter = adapter.clone();
            tokio::spawn(async move {
                while let Ok(event) = event_rx.recv().await {
                    if event.event_type == "chat.message" {
                        if let Ok(p) = serde_json::from_str::<serde_json::Value>(&event.payload) {
                            if p["recipient"]
                                .as_str()
                                .map_or(false, |r| r.starts_with("mattermost:"))
                                || p["role"].as_str() == Some("Assistant")
                            {
                                let sid = p["session_id"].as_str().unwrap_or("");
                                if let Some(ch) = sid.strip_prefix("mattermost:") {
                                    let text = p["content"].as_str().unwrap_or("");
                                    if let Err(e) = send_adapter.send_text(ch, text).await {
                                        warn!("[MATTERMOST] {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            });

            if let Some(ref ch) = adapter.config.channel_id {
                loop {
                    match adapter.poll_posts(ch).await {
                        Ok(posts) => {
                            for post in &posts {
                                let sender = post["user_id"].as_str().unwrap_or("unknown");
                                let message = post["message"].as_str().unwrap_or("");
                                if !message.is_empty() {
                                    let sid =
                                        savant_core::session::SessionMapper::map("mattermost", ch);
                                    let msg = ChatMessage {
                                        is_telemetry: false,
                                        role: ChatRole::User,
                                        content: message.to_string(),
                                        sender: Some(format!("mattermost:{}", sender)),
                                        recipient: Some("savant".into()),
                                        agent_id: None,
                                        session_id: Some(sid),
                                        channel: savant_core::types::AgentOutputChannel::Chat,
                                    };
                                    let frame = EventFrame {
                                        event_type: "chat.message".into(),
                                        payload: serde_json::to_string(&msg)
                                            .unwrap_or_else(|_| "{}".to_string()),
                                    };
                                    let _ = adapter.nexus.event_bus.send(frame);
                                }
                            }
                        }
                        Err(e) => warn!("[MATTERMOST] Poll error: {}", e),
                    }
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            } else {
                futures::future::pending::<()>().await;
            }
        })
    }
}

#[async_trait]
impl ChannelAdapter for MattermostAdapter {
    fn name(&self) -> &str {
        "mattermost"
    }
    async fn send_event(&self, _event: EventFrame) -> Result<(), SavantError> {
        Ok(())
    }
    async fn handle_event(&self, _event: EventFrame) -> Result<(), SavantError> {
        Ok(())
    }
}
