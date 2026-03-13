use crate::react::{AgentEvent, AgentLoop};
use futures::stream::StreamExt;
use savant_core::bus::NexusBridge;
use savant_core::db::Storage;
use savant_core::error::SavantError;
use savant_core::types::{AgentConfig, EventFrame};
use savant_core::utils::{io, parsing};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
//

/// The Autonomous Pulse (Heartbeat) system for Savant agents.
pub struct HeartbeatPulse {
    agent: AgentConfig,
    heartbeat_file: PathBuf,
    nexus: Arc<NexusBridge>,
    storage: Arc<Storage>,
}

impl HeartbeatPulse {
    pub fn new(agent: AgentConfig, nexus: Arc<NexusBridge>, storage: Arc<Storage>) -> Self {
        let heartbeat_file = agent.workspace_path.join("HEARTBEAT.md");
        Self {
            agent,
            heartbeat_file,
            nexus,
            storage,
        }
    }

    /// Starts the heartbeat loop for this agent.
    pub async fn start<M: savant_core::traits::MemoryBackend>(self, mut agent_loop: AgentLoop<M>) {
        // Subscribe to chat messages
        let mut chat_rx = self.nexus.subscribe("chat.message").await;

        info!(
            "[{}] Heartbeat loop active (Non-blocking mode)",
            self.agent.agent_name
        );

        loop {
            tokio::select! {
                // 1. Listen for immediate chat messages
                Ok(chat_event) = chat_rx.recv() => {
                    if chat_event.event_type == "chat.message" {
                        if let Err(e) = self.handle_chat_message(chat_event.payload, &mut agent_loop).await {
                            parsing::log_agent_error(&self.agent.agent_name, "Failed to handle chat message", e);
                        }
                    }
                }

                // 2. Perform periodic proactive pulse
                _ = tokio::time::sleep(Duration::from_secs(self.agent.heartbeat_interval)) => {
                    if let Err(e) = self.pulse(&mut agent_loop).await {
                        parsing::log_agent_error(&self.agent.agent_name, "Heartbeat pulse failed", e);
                    }
                }
            }
        }
    }

    async fn handle_chat_message<M: savant_core::traits::MemoryBackend>(
        &self,
        chat_event: String,
        agent_loop: &mut AgentLoop<M>,
    ) -> Result<(), SavantError> {
        let chat_message: Result<savant_core::types::ChatMessage, _> =
            serde_json::from_str(&chat_event);

        match chat_message {
            Ok(message) => {
                // Check if message is for us or a broadcast
                if let Some(ref target) = message.recipient {
                    if target != &self.agent.agent_id && target != &self.agent.agent_name {
                        return Ok(());
                    }
                }

                info!(
                    "[{}] Received chat message targeting {:?}: {}",
                    self.agent.agent_name, message.recipient, message.content
                );

                // Process the message through Agent loop
                let mut full_response = String::new();

                {
                    let mut stream = agent_loop.run(message.content);
                    while let Some(event_res) = stream.next().await {
                        match event_res {
                            Ok(AgentEvent::Thought(t)) => {
                                full_response.push_str(&t);

                                // Stream chunk to Nexus
                                let chunk = savant_core::types::ChatChunk {
                                    agent_name: self.agent.agent_name.clone(),
                                    agent_id: self.agent.agent_id.clone(),
                                    content: t,
                                    is_final: false,
                                };
                                if let Ok(payload) = serde_json::to_string(&chunk) {
                                    let _ = self.nexus.publish("chat.chunk", &payload).await;
                                }
                            }
                            Ok(AgentEvent::Action { name, args }) => {
                                info!(
                                    "[{}] Chat Action: {}[{}]",
                                    self.agent.agent_name, name, args
                                );
                            }
                            Ok(AgentEvent::FinalAnswer(a)) => {
                                if full_response.is_empty() {
                                    full_response = a;
                                }
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!(
                                    "[{}] Agent Loop Error: {}",
                                    self.agent.agent_name,
                                    e
                                );
                                return Err(e);
                            }
                        }
                    }
                }

                // Send response back through Nexus
                let response = savant_core::types::ChatMessage {
                    role: savant_core::types::ChatRole::Assistant,
                    content: full_response,
                    sender: Some(self.agent.agent_id.clone()),
                    recipient: None, // Responses are broadcast back
                    agent_id: Some(self.agent.agent_id.clone()),
                };

                let response_payload = serde_json::to_string(&response)?;
                self.nexus
                    .publish("chat.response", &response_payload)
                    .await
                    .map_err(|e| SavantError::Unknown(e.to_string()))?;

                info!("[{}] Chat response sent", self.agent.agent_name);
            }
            Err(e) => {
                info!(
                    "[{}] Invalid chat message format: {}",
                    self.agent.agent_name, e
                );
            }
        }

        Ok(())
    }

    async fn pulse<M: savant_core::traits::MemoryBackend>(
        &self,
        agent_loop: &mut AgentLoop<M>,
    ) -> Result<(), SavantError> {
        info!("Heartbeat pulse triggered for {}", self.agent.agent_name);

        // 1. Read HEARTBEAT.md with utility
        let monitoring_tasks = io::read_or_default(
            &self.heartbeat_file,
            "Review your current environment and check for pending tasks.",
        )
        .await;

        let context_injection = self.nexus.get_global_context().await;

        let prompt = format!(
            "SYSTEM HEARTBEAT TICK\n\nTask: Evaluate your board and perform necessary actions.\n\n\
            Global Nexus Context:\n{}\n\n\
            Monitoring Checklist (HEARTBEAT.md):\n{}\n\n\
            Rule: If no action is required, your Final Answer MUST be exactly and only 'HEARTBEAT_OK'.",
            context_injection, monitoring_tasks
        );

        let mut full_response = String::new();
        let mut action_taken = false;

        {
            let mut stream = agent_loop.run(prompt);
            while let Some(event_res) = stream.next().await {
                match event_res {
                    Ok(AgentEvent::Thought(t)) => {
                        full_response.push_str(&t);
                    }
                    Ok(AgentEvent::Action { name, args }) => {
                        info!(
                            "[{}] Proactive Action: {}[{}]",
                            self.agent.agent_name, name, args
                        );
                        action_taken = true;
                    }
                    Ok(AgentEvent::FinalAnswer(a)) => {
                        full_response = a;
                    }
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
        }

        if !action_taken && full_response.trim() == "HEARTBEAT_OK" {
            info!(
                "[{}] Heartbeat nominal. Silence maintained.",
                self.agent.agent_name
            );
            return Ok(());
        }

        info!(
            "[{}] HEARTBEAT INITIATIVE: Agent is speaking up.",
            self.agent.agent_name
        );

        if let Err(e) = self.storage.append_chat(
            &self.agent.agent_id,
            &savant_core::types::ChatMessage {
                role: savant_core::types::ChatRole::Assistant,
                content: format!("[PROACTIVE HEARTBEAT] {}", full_response),
                sender: Some(self.agent.agent_id.clone()),
                recipient: None,
                agent_id: Some(self.agent.agent_id.clone()),
            },
        ) {
            parsing::log_agent_error(
                &self.agent.agent_name,
                "Failed to persist heartbeat to WAL",
                e,
            );
        }

        let event = EventFrame {
            event_type: "proactive_initiation".to_string(),
            payload: json!({
                "agent": self.agent.agent_name,
                "response": full_response,
                "action_taken": action_taken,
            })
            .to_string(),
        };

        if let Err(e) = self.nexus.event_bus.send(event) {
            parsing::log_agent_error(&self.agent.agent_name, "Failed to broadcast to Nexus", e);
        }

        // 2. Perform memory consolidation via MemoryBackend trait
        // Note: In Phase A we use the concrete MemoryManager directly.
        // In late iterations we'd use trait-based delegation.
        // MemoryBackend is implemented for MemoryManager.
        // agent_loop.memory is M which is MemoryManager.
        // agent_loop.memory.consolidate is available.

        Ok(())
    }
}
