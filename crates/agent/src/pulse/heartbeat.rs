use crate::proactive::ProactivePartner;
use crate::react::{AgentEvent, AgentLoop};
use futures::stream::StreamExt;
use savant_core::bus::NexusBridge;
use savant_core::db::Storage;
use savant_core::error::SavantError;
use savant_core::types::AgentConfig;
use savant_core::utils::{io, parsing};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};
use tokio_util::sync::CancellationToken;

/// The Autonomous Pulse (Heartbeat) system for Savant agents.
pub struct HeartbeatPulse {
    agent: AgentConfig,
    heartbeat_file: PathBuf,
    nexus: Arc<NexusBridge>,
    proactive: ProactivePartner,
    shutdown_token: CancellationToken,
}

impl HeartbeatPulse {
    pub fn new(agent: AgentConfig, nexus: Arc<NexusBridge>, _storage: Arc<Storage>, shutdown_token: CancellationToken) -> Self {
        let heartbeat_file = agent.workspace_path.join(&agent.proactive.heartbeat_file);
        let proactive = ProactivePartner::new(agent.workspace_path.clone(), &agent.proactive);
        Self {
            agent,
            heartbeat_file,
            nexus,
            proactive,
            shutdown_token,
        }
    }

    /// Starts the heartbeat loop for this agent.
    pub async fn start<M: savant_core::traits::MemoryBackend + std::clone::Clone>(self, mut agent_loop: AgentLoop<M>) {
        // Subscribe to chat messages
        let mut chat_rx = self.nexus.subscribe().await.0;

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

                // 3. Graceful Shutdown
                _ = self.shutdown_token.cancelled() => {
                    info!("[{}] Heartbeat loop received shutdown signal. Evacuating...", self.agent.agent_name);
                    break;
                }
            }
        }
    }

    async fn handle_chat_message<M: savant_core::traits::MemoryBackend + Clone>(
        &self,
        chat_event: String,
        agent_loop: &mut AgentLoop<M>,
    ) -> Result<(), SavantError> {
        let chat_message: Result<savant_core::types::ChatMessage, _> =
            serde_json::from_str(&chat_event);

        match &chat_message {
            Ok(message) => {
                let content = message.content.clone();
                let recipient = message.recipient.clone();
                let sender = message.sender.clone();
                let agent_id = message.agent_id.clone();
                
                // 🛡️ Identity Pinning: Block Echo-Back (Normalized)
                let my_id = self.agent.agent_id.to_lowercase();
                let my_name = self.agent.agent_name.to_lowercase();

                if let Some(ref sid_raw) = agent_id {
                    let sid = sid_raw.to_lowercase();
                    if sid == my_id || sid == my_name {
                        return Ok(());
                    }
                }
                if let Some(ref s_raw) = sender {
                    let s = s_raw.to_lowercase();
                    if s == my_id || s == my_name {
                        return Ok(());
                    }
                }

                // Check if message is for us or a broadcast
                if let Some(ref target) = recipient {
                    let id = &self.agent.agent_id;
                    let name = &self.agent.agent_name;
                    let is_target = target == id || target == name || target == "global" || target == "swarm";
                    
                    if !is_target {
                        return Ok(());
                    }
                }

                info!(
                    "[{}] Received chat message targeting {:?}: {}",
                    self.agent.agent_name, recipient, content
                );

                let response_recipient = sender;
                
                // Process the message through Agent loop
                let mut full_response = String::new();
                let memory_clone = agent_loop.memory.clone();

                {
                    let shutdown_token = self.shutdown_token.clone();
                    let mut stream = agent_loop.run(content, message.session_id.clone(), shutdown_token);
                    while let Some(event_res) = stream.next().await {
                        match event_res {
                            Ok(AgentEvent::Thought(t)) => {
                                // 🛡️ Perfection Loop: Thoughts are strictly telemetry
                                let chunk = savant_core::types::ChatChunk {
                                    agent_name: self.agent.agent_name.clone(),
                                    agent_id: self.agent.agent_id.to_lowercase(),
                                    content: t,
                                    is_final: false,
                                    session_id: message.session_id.clone(),
                                    channel: savant_core::types::AgentOutputChannel::Telemetry,
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
                                // 🛰️ Real-time Tool Telemetry
                                let chunk = savant_core::types::ChatChunk {
                                    agent_name: self.agent.agent_name.clone(),
                                    agent_id: self.agent.agent_id.to_lowercase(),
                                    content: format!("\n\n> 🛠️ **Executing Tool:** `{}`\n> *Args:* `{}`\n\n", name, args),
                                    is_final: false,
                                    session_id: message.session_id.clone(),
                                    channel: savant_core::types::AgentOutputChannel::Telemetry,
                                };
                                if let Ok(payload) = serde_json::to_string(&chunk) {
                                    let _ = self.nexus.publish("chat.chunk", &payload).await;
                                }
                            }
                            Ok(AgentEvent::Reflection(r)) => {
                                // 🛰️ Memory Channel Telemetry
                                let chunk = savant_core::types::ChatChunk {
                                    agent_name: self.agent.agent_name.clone(),
                                    agent_id: self.agent.agent_id.to_lowercase(),
                                    content: r.clone(),
                                    is_final: false,
                                    session_id: message.session_id.clone(),
                                    channel: savant_core::types::AgentOutputChannel::Memory,
                                };
                                if let Ok(payload) = serde_json::to_string(&chunk) {
                                    let _ = self.nexus.publish("chat.chunk", &payload).await;
                                }

                                let emitter = crate::learning::emitter::LearningEmitter::new(
                                    self.agent.agent_id.clone(),
                                    memory_clone.clone(),
                                    self.nexus.clone(),
                                );
                                let _ = emitter.emit_emergent(r, None).await;
                            }
                            Ok(AgentEvent::Observation(o)) => {
                                debug!("[{}] Observation: {}", self.agent.agent_name, o);
                                // 🛰️ Observation Telemetry
                                let chunk = savant_core::types::ChatChunk {
                                    agent_name: self.agent.agent_name.clone(),
                                    agent_id: self.agent.agent_id.to_lowercase(),
                                    content: format!("\n> 👁️ **Observation:** *Successful acquisition of {} context bytes.*\n\n", o.len()),
                                    is_final: false,
                                    session_id: message.session_id.clone(),
                                    channel: savant_core::types::AgentOutputChannel::Telemetry,
                                };
                                if let Ok(payload) = serde_json::to_string(&chunk) {
                                    let _ = self.nexus.publish("chat.chunk", &payload).await;
                                }
                            }
                            Ok(AgentEvent::FinalAnswer(a)) => {
                                full_response = a;
                            }
                            Ok(AgentEvent::FinalAnswerChunk(c)) => {
                                // 🌀 Perfection Loop: Assistant final chunks are GUARANTEED dialogue
                                full_response.push_str(&c);
                                
                                let chunk = savant_core::types::ChatChunk {
                                    agent_name: self.agent.agent_name.clone(),
                                    agent_id: self.agent.agent_id.to_lowercase(),
                                    content: c,
                                    is_final: false,
                                    session_id: message.session_id.clone(),
                                    channel: savant_core::types::AgentOutputChannel::Chat,
                                };
                                if let Ok(payload) = serde_json::to_string(&chunk) {
                                    let _ = self.nexus.publish("chat.chunk", &payload).await;
                                }
                            }
                            Ok(AgentEvent::StatusUpdate(s)) => {
                                // 🛰️ Status events are Telemetry
                                let chunk = savant_core::types::ChatChunk {
                                    agent_name: self.agent.agent_name.clone(),
                                    agent_id: self.agent.agent_id.to_lowercase(),
                                    content: s,
                                    is_final: false,
                                    session_id: message.session_id.clone(),
                                    channel: savant_core::types::AgentOutputChannel::Telemetry,
                                };
                                if let Ok(payload) = serde_json::to_string(&chunk) {
                                    let _ = self.nexus.publish("chat.chunk", &payload).await;
                                }
                            }
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

                // Send response back through Nexus: Standardized at chat.message
                let response = savant_core::types::ChatMessage {
                    role: savant_core::types::ChatRole::Assistant,
                    content: full_response,
                    sender: Some(self.agent.agent_id.clone()),
                    recipient: response_recipient, 
                    agent_id: None,
                    session_id: message.session_id.clone(),
                    channel: savant_core::types::AgentOutputChannel::Chat,
                };

                let response_payload = serde_json::to_string(&response)?;
                self.nexus
                    .publish("chat.message", &response_payload)
                    .await
                    .map_err(|e| SavantError::Unknown(e.to_string()))?;

                info!("[{}] Chat response sent (Standardized Lane)", self.agent.agent_name);
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

    async fn pulse<M: savant_core::traits::MemoryBackend + Clone>(
        &self,
        agent_loop: &mut AgentLoop<M>,
    ) -> Result<(), SavantError> {
        info!("Heartbeat pulse triggered for {}", self.agent.agent_name);

        let emitter = crate::learning::emitter::LearningEmitter::new(
            self.agent.agent_id.clone(),
            agent_loop.memory.clone(),
            self.nexus.clone(),
        );

        // 1. Read monitoring tasks from config-defined path
        let monitoring_tasks = io::read_or_default(
            &self.heartbeat_file,
            "Review your current environment and check for pending tasks.",
        )
        .await;
        let context_injection = self.nexus.get_global_context().await;

        // 城堡 OMEGA-VIII: Orchestration Injection (Task Matrix - Config Driven)
        let matrix = crate::orchestration::tasks::TaskMatrix::new(
            &self.agent.workspace_path, 
            &self.agent.proactive
        );
        let orchestration_tasks = matrix.get_pending_summary();

        // 🏰 OMEGA-VIII: High-Fidelity Perception Injection
        let git_status = crate::proactive::perception::PerceptionEngine::get_git_status(&self.agent.workspace_path);
        let git_diff = crate::proactive::perception::PerceptionEngine::get_git_diff(&self.agent.workspace_path);
        let fs_activity = crate::proactive::perception::PerceptionEngine::get_fs_activity(&self.agent.workspace_path);

        // 🏰 OMEGA-VIII: Anomaly Detection (Proactive Push Logic)
        let has_conflict = git_status.contains("CONFLICT");
        let has_errors = fs_activity.to_lowercase().contains("error");
        let anomaly_alert = if has_conflict || has_errors {
            "\n⚠️ **ANOMALY DETECTED**: System integrity may be compromised (Merge Conflict or FS Error). PROACTIVE NOTIFICATION MANDATORY.\n"
        } else {
            ""
        };

        let prompt_base = format!(
            "Protocol C-ATLAS: Sovereign Heartbeat (Iteration Peak)\n\n\
            You are the House. The Foundation. You are Savant.\n\
            REVERSE PROMPTING: Do not wait for instructions. What would help your human right now? \
            Is there a substrate optimization needed? A pending task in HEARTBEAT.md? \
            A proactive refactor that would improve the 101-agent swarm?\n\n\
            VITAL CONTEXT:\n\
            Nexus Global Context:\n{}\n\n\
            SITUATIONAL AWARENESS (Perception Engine):\n{}\n{}\n{}\n{}\n\n\
            ORCHESTRATION (Task Matrix):\n{}\n\n\
            Directives (HEARTBEAT.md):\n{}\n\n\
            SUBSTRATE SELF-OPTIMIZATION (OMEGA-VIII):\n\
            Review your own current Pulse architecture. Is there a logical bottleneck? \
            Would a structural change to `heartbeat.rs` or `memory/mod.rs` yield higher cognitive fidelity?\n\n\
            Write your internal reflection, execute any necessary care-taking tools, \
            and project your future-intent.",
            context_injection, git_status, git_diff, fs_activity, anomaly_alert, orchestration_tasks, monitoring_tasks
        );

        // --- 🛡️ OMEGA-VIII: Deterministic Pre-filtering (Lane-Perfection) ---
        let current_hash = xxhash_rust::xxh3::xxh3_64(prompt_base.as_bytes());
        
        // AAA: Restore working buffer
        let mut buffer = self.proactive.restore_state().unwrap_or_default();

        if let Some(h) = buffer.last_pulse_hash {
            if h == current_hash {
                info!("[{}] Deterministic Stillness: Substrate state identical to last pulse. Skipping inference.", self.agent.agent_name);
                return Ok(());
            }
        }
        buffer.last_pulse_hash = Some(current_hash);

        let prompt = prompt_base;
        
        let mut pulse_thought = String::new();
        let mut pulse_dialogue = String::new();
        let mut action_taken = false;

        {
            let shutdown_token = self.shutdown_token.clone();
            let mut stream = agent_loop.run(prompt, None, shutdown_token);
            while let Some(event_res) = stream.next().await {
                match event_res {
                    Ok(AgentEvent::Thought(t)) => {
                        pulse_thought.push_str(&t);
                        // 🛰️ Real-time Telemetry Stream
                        let chunk = savant_core::types::ChatChunk {
                            agent_name: self.agent.agent_name.clone(),
                            agent_id: self.agent.agent_id.to_lowercase(),
                            content: t,
                            is_final: false,
                            session_id: None,
                            channel: savant_core::types::AgentOutputChannel::Telemetry,
                        };
                        if let Ok(payload) = serde_json::to_string(&chunk) {
                            let _ = self.nexus.publish("chat.chunk", &payload).await;
                        }
                    }
                    Ok(AgentEvent::Action { name, args }) => {
                        info!(
                            "[{}] Proactive Action: {}[{}]",
                            self.agent.agent_name, name, args
                        );
                        action_taken = true;
                        // 🛰️ Real-time Tool Telemetry
                        let chunk = savant_core::types::ChatChunk {
                            agent_name: self.agent.agent_name.clone(),
                            agent_id: self.agent.agent_id.to_lowercase(),
                            content: format!("\n\n> 🛠️ **Foundation Action:** `{}`\n> *Parameters:* `{}`\n\n", name, args),
                            is_final: false,
                            session_id: None,
                            channel: savant_core::types::AgentOutputChannel::Telemetry,
                        };
                        if let Ok(payload) = serde_json::to_string(&chunk) {
                            let _ = self.nexus.publish("chat.chunk", &payload).await;
                        }
                    }
                    Ok(AgentEvent::Observation(o)) => {
                        debug!("[{}] Observation: {}", self.agent.agent_name, o);
                         // 🛰️ Observation Telemetry
                         let chunk = savant_core::types::ChatChunk {
                            agent_name: self.agent.agent_name.clone(),
                            agent_id: self.agent.agent_id.to_lowercase(),
                            content: format!("\n> 👁️ **Substrate Perception:** *Mapped {} bytes of system data.*\n\n", o.len()),
                            is_final: false,
                            session_id: None,
                            channel: savant_core::types::AgentOutputChannel::Telemetry,
                        };
                        if let Ok(payload) = serde_json::to_string(&chunk) {
                            let _ = self.nexus.publish("chat.chunk", &payload).await;
                        }
                    }
                    Ok(AgentEvent::FinalAnswer(a)) => {
                        pulse_dialogue = a;
                    }
                    Ok(AgentEvent::FinalAnswerChunk(c)) => {
                        pulse_dialogue.push_str(&c);
                        // 🛰️ Real-time Dialogue Stream (marked as telemetry for heartbeat)
                        let chunk = savant_core::types::ChatChunk {
                            agent_name: self.agent.agent_name.clone(),
                            agent_id: self.agent.agent_id.to_lowercase(),
                            content: c,
                            is_final: false,
                            session_id: None,
                            channel: savant_core::types::AgentOutputChannel::Telemetry,
                        };
                        if let Ok(payload) = serde_json::to_string(&chunk) {
                            let _ = self.nexus.publish("chat.chunk", &payload).await;
                        }
                    }
                    Ok(AgentEvent::Reflection(r)) => {
                        // Harvest emergent reflection
                        let _ = emitter.emit_emergent(r, None).await;
                    }
                    Ok(AgentEvent::StatusUpdate(s)) => {
                        debug!("[{}] Status: {}", self.agent.agent_name, s);
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        // 🏰 Substrate Logic: Handle Stillness and Reflections
        let is_silent = pulse_dialogue.trim().is_empty() || pulse_dialogue.trim() == "HEARTBEAT_OK";
        
        if !action_taken && is_silent {
            if !pulse_thought.trim().is_empty() {
                info!("[{}] Internal reflection captured during stillness.", self.agent.agent_name);
                let _ = emitter.emit_emergent(pulse_thought.clone(), Some(savant_core::learning::LearningCategory::Insight)).await;
            } else {
                info!("[{}] Complete stillness maintained.", self.agent.agent_name);
            }
            return Ok(());
        }

        // AAA: Update WorkingBuffer based on Pulse results
        buffer.current_goal = "Autonomous Maintenance & Swarm Sync".to_string();
        if action_taken {
            buffer.pending_actions.push("Verify substrate health post-actuation".to_string());
        }
        
        // AAA: Sovereign Distillation (OMEGA-VIII)
        if !pulse_thought.is_empty() || !pulse_dialogue.is_empty() {
             let summary = format!("Thought: {}\nDialogue: {}", pulse_thought, pulse_dialogue);
             let _ = self.proactive.distill_context(&summary);
             buffer.context_summary = summary;
        }

        // Commit to WAL
        let _ = self.proactive.commit_state(&buffer);

        // AAA: Autonomous Lesson Distillation (ALD)
        let ald = crate::learning::ald::ALDEngine::new(self.agent.workspace_path.clone());
        if let Err(e) = ald.distill() {
            warn!("[{}] ALD Distillation failed: {}", self.agent.agent_name, e);
        }

        info!(
            "[{}] HEARTBEAT INITIATIVE: The House speaks. WAL Committed.",
            self.agent.agent_name
        );

        // 🌀 Perfection Loop: Harvest the spoken response as a potential insight
        let mut full_payload = pulse_thought.clone();
        if !pulse_dialogue.trim().is_empty() {
            if !full_payload.is_empty() {
                full_payload.push_str("\n\n");
            }
            full_payload.push_str(&pulse_dialogue);
        }
        
        if !full_payload.trim().is_empty() {
            let _ = emitter.emit_emergent(full_payload.clone(), Some(savant_core::learning::LearningCategory::Insight)).await;
        }

        // 🛰️ Final Telemetry Message (Standardized Lane for History)
        if !pulse_dialogue.trim().is_empty() {
            let final_msg = savant_core::types::ChatMessage {
                role: savant_core::types::ChatRole::Assistant,
                content: pulse_dialogue,
                sender: Some(self.agent.agent_id.clone()),
                recipient: None,
                agent_id: None,
                session_id: None, // Heartbeat pulses are system-local
                channel: savant_core::types::AgentOutputChannel::Telemetry,
            };

            if let Ok(payload) = serde_json::to_string(&final_msg) {
                let _ = self.nexus.publish("chat.message", &payload).await;
            }
        }

        Ok(())
    }
}
