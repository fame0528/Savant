use async_trait::async_trait;
use savant_core::error::SavantError;
use savant_core::traits::ChannelAdapter;
use savant_core::types::{ChatMessage, ChatRole, EventFrame};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tracing::{info, warn};

/// Twitch channel configuration.
#[derive(Debug, Clone)]
pub struct TwitchConfig {
    pub oauth_token: String, // oauth:xxxxx
    pub nickname: String,
    pub channel: String,
}

/// Twitch channel adapter.
/// Uses Twitch IRC (irc.chat.twitch.tv:6697) for chat integration.
pub struct TwitchAdapter {
    config: TwitchConfig,
    nexus: Arc<savant_core::bus::NexusBridge>,
}

impl TwitchAdapter {
    pub fn new(config: TwitchConfig, nexus: Arc<savant_core::bus::NexusBridge>) -> Self {
        Self { config, nexus }
    }

    /// Connects to Twitch IRC with TLS.
    async fn connect(
        &self,
    ) -> Result<
        (
            BufReader<tokio::io::ReadHalf<tokio_native_tls::TlsStream<TcpStream>>>,
            tokio::io::WriteHalf<tokio_native_tls::TlsStream<TcpStream>>,
        ),
        SavantError,
    > {
        let tcp = TcpStream::connect("irc.chat.twitch.tv:6697")
            .await
            .map_err(|e| SavantError::Unknown(format!("Twitch connect failed: {}", e)))?;

        let connector = tokio_native_tls::TlsConnector::from(
            native_tls::TlsConnector::new()
                .map_err(|e| SavantError::Unknown(format!("TLS init failed: {}", e)))?,
        );

        let tls = connector
            .connect("irc.chat.twitch.tv", tcp)
            .await
            .map_err(|e| SavantError::Unknown(format!("Twitch TLS failed: {}", e)))?;

        let (read_half, write_half) = tokio::io::split(tls);
        let reader = BufReader::new(read_half);

        Ok((reader, write_half))
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!(
                "[TWITCH] Starting Twitch adapter for #{}",
                self.config.channel
            );

            // Reconnection loop with exponential backoff
            let mut backoff = std::time::Duration::from_secs(1);
            let max_backoff = std::time::Duration::from_secs(60);

            loop {
                let (reader, mut writer) = match self.connect().await {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("[TWITCH] Connect failed: {}, retrying in {:?}", e, backoff);
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(max_backoff);
                        continue;
                    }
                };

                // Reset backoff on successful connection
                backoff = std::time::Duration::from_secs(1);

                // Authenticate
                if let Err(e) = writer
                    .write_all(format!("PASS {}\r\n", self.config.oauth_token).as_bytes())
                    .await
                {
                    tracing::warn!("[channels] IRC write failed: {}", e);
                }
                if let Err(e) = writer
                    .write_all(format!("NICK {}\r\n", self.config.nickname).as_bytes())
                    .await
                {
                    tracing::warn!("[channels] IRC write failed: {}", e);
                }
                if let Err(e) = writer
                    .write_all(format!("JOIN #{}\r\n", self.config.channel).as_bytes())
                    .await
                {
                    tracing::warn!("[channels] IRC write failed: {}", e);
                }
                if let Err(e) = writer
                    .write_all("CAP REQ :twitch.tv/commands\r\n".as_bytes())
                    .await
                {
                    tracing::warn!("[channels] IRC write failed: {}", e);
                }
                if let Err(e) = writer.flush().await {
                    tracing::warn!("[channels] IRC flush failed: {}", e);
                }

                info!("[TWITCH] Connected to #{}", self.config.channel);

                let (mut event_rx, _) = self.nexus.subscribe().await;
                let channel = self.config.channel.clone();
                let writer = Arc::new(tokio::sync::Mutex::new(writer));

                // Outbound listener
                let out_writer = writer.clone();
                let out_channel = channel.clone();
                tokio::spawn(async move {
                    while let Ok(event) = event_rx.recv().await {
                        if event.event_type == "chat.message" {
                            if let Ok(p) = serde_json::from_str::<serde_json::Value>(&event.payload)
                            {
                                if p["recipient"]
                                    .as_str()
                                    .map_or(false, |r| r.starts_with("twitch:"))
                                    && p["role"].as_str() == Some("Assistant")
                                {
                                    let text = p["content"].as_str().unwrap_or("");
                                    // Twitch max 500 chars
                                    let truncated: String = text.chars().take(500).collect();
                                    let msg =
                                        format!("PRIVMSG #{} :{}\r\n", out_channel, truncated);
                                    if let Err(e) =
                                        out_writer.lock().await.write_all(msg.as_bytes()).await
                                    {
                                        warn!("[TWITCH] Send error: {}", e);
                                    }
                                }
                            }
                        }
                    }
                });

                // Inbound reader loop — exits on disconnect to trigger reconnect
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    // Handle PING/PONG
                    if line.starts_with("PING") {
                        let pong = line.replace("PING", "PONG");
                        if let Err(e) = writer
                            .lock()
                            .await
                            .write_all(format!("{}\r\n", pong).as_bytes())
                            .await
                        {
                            tracing::warn!(
                                "[channels::twitch] Failed to send PONG response: {}",
                                e
                            );
                        }
                        continue;
                    }

                    // Parse PRIVMSG
                    if let Some(msg_part) =
                        line.strip_prefix(":").and_then(|s| s.split_once("PRIVMSG"))
                    {
                        if let Some((prefix, content)) = msg_part.1.split_once(" :") {
                            let sender = prefix.split('!').next().unwrap_or("unknown");
                            let text = content.trim();
                            if !text.is_empty() {
                                let sid =
                                    savant_core::session::SessionMapper::map("twitch", &channel);
                                let chat_msg = ChatMessage {
                                    is_telemetry: false,
                                    role: ChatRole::User,
                                    content: text.to_string(),
                                    sender: Some(format!("twitch:{}", sender)),
                                    recipient: Some("savant".into()),
                                    agent_id: None,
                                    session_id: Some(sid),
                                    channel: savant_core::types::AgentOutputChannel::Chat,
                                };
                                let frame = EventFrame {
                                    event_type: "chat.message".into(),
                                    payload: serde_json::to_string(&chat_msg).unwrap_or_default(),
                                };
                                if self.nexus.event_bus.send(frame).is_err() {
                                    tracing::warn!("[channels::twitch] Event bus send failed");
                                }
                            }
                        }
                    }
                }

                // Reader loop ended — connection lost, reconnect with backoff
                warn!("[TWITCH] Connection lost, reconnecting in {:?}", backoff);
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(max_backoff);
            }
        })
    }
}

#[async_trait]
impl ChannelAdapter for TwitchAdapter {
    fn name(&self) -> &str {
        "twitch"
    }
    async fn send_event(&self, _e: EventFrame) -> Result<(), SavantError> {
        Ok(())
    }
    async fn handle_event(&self, _e: EventFrame) -> Result<(), SavantError> {
        Ok(())
    }
}
