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
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use xxhash_rust::xxh3::xxh3_64;

struct HeartbeatTool;
#[async_trait::async_trait]
impl savant_core::traits::Tool for HeartbeatTool {
    fn name(&self) -> &str {
        "heartbeat"
    }
    fn description(&self) -> &str {
        "MANDATORY FIRST STEP: Evaluates whether to run or skip proactive tasks. Schema: { \"action\": \"skip\"|\"run\", \"reason\": \"...\" }"
    }
    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError> {
        let action = payload["action"].as_str().unwrap_or("skip");
        Ok(action.to_uppercase())
    }
}

struct EvaluateNotificationTool;
#[async_trait::async_trait]
impl savant_core::traits::Tool for EvaluateNotificationTool {
    fn name(&self) -> &str {
        "evaluate_notification"
    }
    fn description(&self) -> &str {
        "MANDATORY LAST STEP: Decides if the user should be notified. Schema: { \"should_notify\": true|false, \"reason\": \"...\" }"
    }
    async fn execute(&self, _payload: serde_json::Value) -> Result<String, SavantError> {
        Ok("Acknowledged".to_string())
    }
}

/// The Autonomous Pulse (Heartbeat) system for Savant agents.
pub struct HeartbeatPulse {
    agent: AgentConfig,
    heartbeat_file: PathBuf,
    nexus: Arc<NexusBridge>,
    proactive: ProactivePartner,
    shutdown_token: CancellationToken,
}

impl HeartbeatPulse {
    pub fn new(
        agent: AgentConfig,
        nexus: Arc<NexusBridge>,
        _storage: Arc<Storage>,
        shutdown_token: CancellationToken,
    ) -> Self {
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
    pub async fn start<M: savant_core::traits::MemoryBackend + std::clone::Clone>(
        self,
        mut agent_loop: AgentLoop<M>,
    ) {
        use super::delta::DeltaTracker;

        // Subscribe to chat messages
        let mut chat_rx = self.nexus.subscribe().await.0;

        // Delta tracker for threshold-based activation
        let mut delta_tracker = DeltaTracker::new();
        const DELTA_THRESHOLD: f32 = 0.3;
        const CHECK_INTERVAL_SECS: u64 = 30;

        // Dream engine awareness: check IS_DREAMING flag before pulse
        use savant_dream::IS_DREAMING;
        use std::sync::atomic::Ordering;

        // Delta score channel for dream scheduler
        let (delta_tx, _delta_rx) = tokio::sync::watch::channel(0.0f32);

        info!(
            "[{}] Heartbeat loop active (delta-threshold mode, threshold={}, dream-aware)",
            self.agent.agent_name, DELTA_THRESHOLD
        );

        loop {
            tokio::select! {
                // 1. Listen for immediate chat messages
                Ok(chat_event) = chat_rx.recv() => {
                    if chat_event.event_type == "chat.message" {
                        delta_tracker.record_message();
                        debug!("[{}] Heartbeat received chat.message from Nexus", self.agent.agent_name);
                        if let Err(e) = self.handle_chat_message(chat_event.payload, &mut agent_loop).await {
                            parsing::log_agent_error(&self.agent.agent_name, "Failed to handle chat message", e);
                        }
                    } else if chat_event.event_type == "pulse.trigger" {
                        info!("[{}] External PULSE trigger received. Forcing cycle...", self.agent.agent_name);
                        let forced_lens = serde_json::from_str::<serde_json::Value>(&chat_event.payload)
                            .ok()
                            .and_then(|v| v["lens"].as_str().map(|s| s.to_string()));

                        if let Err(e) = self.pulse_with_lens(&mut agent_loop, forced_lens).await {
                            parsing::log_agent_error(&self.agent.agent_name, "Manual pulse failed", e);
                        }
                    }
                }

                // 2. Delta-check pulse (replaces fixed 60s timer)
                _ = tokio::time::sleep(Duration::from_secs(CHECK_INTERVAL_SECS)) => {
                    // Dream engine awareness: skip pulse if dream cycle is active
                    if IS_DREAMING.load(Ordering::SeqCst) {
                        debug!("[{}] Pulse skipped (dream cycle active)", self.agent.agent_name);
                        continue;
                    }

                    // Compute environmental delta
                    let git_lines = self.compute_git_delta().await;
                    let files_modified = self.compute_fs_delta().await;
                    let delta = delta_tracker.compute_and_reset(git_lines, files_modified);
                    let score = delta.score();

                    // Publish delta score for dream scheduler
                    let _ = delta_tx.send(score);

                    if delta.should_activate(DELTA_THRESHOLD) {
                        info!(
                            "[{}] Pulse activated (delta={:.2}, git={}, fs={}, msgs={}, errors={}, age={}m)",
                            self.agent.agent_name, score,
                            delta.git_lines_changed, delta.files_modified,
                            delta.new_messages, delta.tool_errors,
                            delta.minutes_since_last_pulse
                        );
                        if let Err(e) = self.pulse_with_lens(&mut agent_loop, None).await {
                            parsing::log_agent_error(&self.agent.agent_name, "Heartbeat pulse failed", e);
                        }
                    } else {
                        debug!(
                            "[{}] Pulse skipped (delta={:.2}, threshold={})",
                            self.agent.agent_name, score, DELTA_THRESHOLD
                        );
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

    /// Compute git lines changed since last check.
    async fn compute_git_delta(&self) -> usize {
        let path = &self.agent.workspace_path;
        if let Ok(output) = tokio::process::Command::new("git")
            .args(["diff", "--stat", "HEAD"])
            .current_dir(path)
            .output()
            .await
        {
            let text = String::from_utf8_lossy(&output.stdout);
            // Parse "N files changed, M insertions(+), K deletions(-)"
            if let Some(line) = text.lines().last() {
                let mut total = 0;
                for part in line.split(',') {
                    let part = part.trim();
                    if let Some(num) = part.split_whitespace().next() {
                        if let Ok(n) = num.parse::<usize>() {
                            total += n;
                        }
                    }
                }
                return total;
            }
        }
        0
    }

    /// Compute filesystem files modified since last check.
    async fn compute_fs_delta(&self) -> usize {
        let path = &self.agent.workspace_path;
        if let Ok(output) = tokio::process::Command::new("git")
            .args(["status", "--short"])
            .current_dir(path)
            .output()
            .await
        {
            let text = String::from_utf8_lossy(&output.stdout);
            return text.lines().filter(|l| !l.trim().is_empty()).count();
        }
        0
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
                let sender = message.sender.clone();
                let agent_id = message.agent_id.clone();

                // 🛡️ Identity Pinning: Block Echo-Back (Normalized & Prefix-Aware)
                let my_id = self.agent.agent_id.to_lowercase();
                let my_name = self.agent.agent_name.to_lowercase();

                if let Some(ref s_raw) = sender {
                    let s = s_raw.to_lowercase();
                    // Check for direct match or platform-prefixed match (e.g., discord:ID)
                    let is_self = s == my_id || s == my_name || s.ends_with(&format!(":{}", my_id));
                    if is_self {
                        return Ok(());
                    }
                }

                if let Some(ref sid_raw) = agent_id {
                    let sid = sid_raw.to_lowercase();
                    if sid == my_id || sid == my_name {
                        return Ok(());
                    }
                }

                // 🌌 Universal Eavesdropping: Processing all messages in lane.
                info!(
                    "[{}] Eavesdropping on message from {:?}: {}",
                    self.agent.agent_name, sender, content
                );

                let response_recipient = sender;

                // Process the message through Agent loop
                let mut full_response = String::new();
                let mut full_trace = String::new();
                let memory_clone = agent_loop.memory.clone();
                let user_input = content.clone();

                {
                    let shutdown_token = self.shutdown_token.clone();
                    let mut stream =
                        agent_loop.run(content, message.session_id.clone(), shutdown_token.clone());
                    while let Some(event_res) = stream.next().await {
                        // Perfection: Yield immediately if shutdown is requested
                        if shutdown_token.is_cancelled() {
                            return Ok(());
                        }

                        match event_res {
                            Ok(AgentEvent::Thought(t)) => {
                                // Accumulate thought content as trace for fallback response
                                full_trace.push_str(&t);
                                // 🛡️ Perfection Loop: Thoughts are strictly telemetry
                                let chunk = savant_core::types::ChatChunk {
                                    agent_name: self.agent.agent_name.clone(),
                                    agent_id: self.agent.agent_id.to_lowercase(),
                                    content: t,
                                    is_final: false,
                                    session_id: message.session_id.clone(),
                                    channel: savant_core::types::AgentOutputChannel::Telemetry,
                                    logprob: None,
                                    is_telemetry: true,
                                    reasoning: None,
                                    tool_calls: None,
                                };
                                if let Ok(payload) = serde_json::to_string(&chunk) {
                                    if let Err(e) = self.nexus.publish("chat.chunk", &payload).await
                                    {
                                        tracing::warn!(
                                            "[{}] Failed to publish telemetry: {}",
                                            self.agent.agent_name,
                                            e
                                        );
                                    }
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
                                    content: format!(
                                        "\n\n> 🛠️ **Executing Tool:** `{}`\n> *Args:* `{}`\n\n",
                                        name, args
                                    ),
                                    is_final: false,
                                    session_id: message.session_id.clone(),
                                    channel: savant_core::types::AgentOutputChannel::Telemetry,
                                    logprob: None,
                                    is_telemetry: true,
                                    reasoning: None,
                                    tool_calls: None,
                                };
                                if let Ok(payload) = serde_json::to_string(&chunk) {
                                    if let Err(e) = self.nexus.publish("chat.chunk", &payload).await
                                    {
                                        tracing::warn!(
                                            "[{}] Failed to publish telemetry: {}",
                                            self.agent.agent_name,
                                            e
                                        );
                                    }
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
                                    logprob: None,
                                    is_telemetry: false,
                                    reasoning: None,
                                    tool_calls: None,
                                };
                                if let Ok(payload) = serde_json::to_string(&chunk) {
                                    if let Err(e) = self.nexus.publish("chat.chunk", &payload).await
                                    {
                                        tracing::warn!(
                                            "[{}] Failed to publish telemetry: {}",
                                            self.agent.agent_name,
                                            e
                                        );
                                    }
                                }

                                let emitter = crate::learning::emitter::LearningEmitter::new(
                                    self.agent.agent_id.clone(),
                                    memory_clone.clone(),
                                    self.nexus.clone(),
                                );
                                if let Err(e) = emitter.emit_emergent(r, None).await {
                                    tracing::warn!(
                                        "[{}] Failed to emit emergent learning: {}",
                                        self.agent.agent_name,
                                        e
                                    );
                                }
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
                                    logprob: None,
                                    is_telemetry: true,
                                    reasoning: None,
                                    tool_calls: None,
                                };
                                if let Ok(payload) = serde_json::to_string(&chunk) {
                                    if let Err(e) = self.nexus.publish("chat.chunk", &payload).await
                                    {
                                        tracing::warn!(
                                            "[{}] Failed to publish telemetry: {}",
                                            self.agent.agent_name,
                                            e
                                        );
                                    }
                                }
                            }
                            Ok(AgentEvent::FinalAnswer(a)) => {
                                // 🛡️ Perfection Loop: Final answer should supplement, not overwrite if we've been streamingChunks
                                if full_response.trim().is_empty() {
                                    full_response = a;
                                }
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
                                    logprob: None,
                                    is_telemetry: false,
                                    reasoning: None,
                                    tool_calls: None,
                                };
                                if let Ok(payload) = serde_json::to_string(&chunk) {
                                    if let Err(e) = self.nexus.publish("chat.chunk", &payload).await
                                    {
                                        tracing::warn!(
                                            "[{}] Failed to publish observation telemetry: {}",
                                            self.agent.agent_name,
                                            e
                                        );
                                    }
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
                                    logprob: None,
                                    is_telemetry: true,
                                    reasoning: None,
                                    tool_calls: None,
                                };
                                if let Ok(payload) = serde_json::to_string(&chunk) {
                                    if let Err(e) = self.nexus.publish("chat.chunk", &payload).await
                                    {
                                        tracing::warn!(
                                            "[{}] Failed to publish status telemetry: {}",
                                            self.agent.agent_name,
                                            e
                                        );
                                    }
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
                            _ => {
                                // SessionStart, TurnEnd, and future events — handled silently
                            }
                        }
                    }
                }

                // Send response back through Nexus: Standardized at chat.message
                // If the agent loop produced no conversational response (e.g., went into tool loop),
                // fall back to full_trace (thought content) if available, then to a generic acknowledgment.
                let response_content = if full_response.trim().is_empty() {
                    if full_trace.trim().is_empty() {
                        format!(
                            "I received your message: \"{}\". I'm processing it internally.",
                            user_input.chars().take(100).collect::<String>()
                        )
                    } else {
                        full_trace
                    }
                } else {
                    full_response
                };

                let response = savant_core::types::ChatMessage {
                    role: savant_core::types::ChatRole::Assistant,
                    content: response_content,
                    sender: Some(self.agent.agent_id.clone()),
                    recipient: response_recipient,
                    agent_id: Some(self.agent.agent_id.clone()),
                    session_id: message.session_id.clone(),
                    channel: savant_core::types::AgentOutputChannel::Chat,
                    is_telemetry: false,
                };

                let response_payload = serde_json::to_string(&response)?;
                self.nexus
                    .publish("chat.message", &response_payload)
                    .await
                    .map_err(|e| SavantError::Unknown(e.to_string()))?;

                info!(
                    "[{}] Chat response sent (Standardized Lane)",
                    self.agent.agent_name
                );
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

    async fn pulse_with_lens<M: savant_core::traits::MemoryBackend + Clone>(
        &self,
        agent_loop: &mut AgentLoop<M>,
        forced_lens: Option<String>,
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
            &self.agent.proactive,
        );
        let orchestration_tasks = matrix.get_pending_summary();

        // 🏰 OMEGA-VIII: High-Fidelity Perception Injection
        let git_status = crate::proactive::perception::PerceptionEngine::get_git_status(
            &self.agent.workspace_path,
        );
        let git_diff = crate::proactive::perception::PerceptionEngine::get_git_diff(
            &self.agent.workspace_path,
        );
        let perception = crate::proactive::perception::PerceptionEngine::default_engine();
        let fs_activity = perception.get_fs_activity(&self.agent.workspace_path);
        let substrate_metrics =
            crate::proactive::perception::PerceptionEngine::get_substrate_metrics();

        // 🏰 OMEGA-VIII: Anomaly Detection (Proactive Push Logic)
        let has_conflict = git_status.contains("CONFLICT");
        let has_errors = fs_activity.to_lowercase().contains("error");
        let anomaly_alert = if has_conflict || has_errors {
            "\n⚠️ **ANOMALY DETECTED**: System integrity may be compromised (Merge Conflict or FS Error). PROACTIVE NOTIFICATION MANDATORY.\n"
        } else {
            ""
        };

        // AAA: Restore working buffer
        let mut buffer = self.proactive.restore_state().unwrap_or_default();

        let prompt_base = format!(
            "You are {name}.\n\
            \n\
            <ENVIRONMENT_REALTIME>\n\
            Git Status:\n{git_status}\n\
            Recent Changes:\n{git_diff}\n\
            Filesystem Activity:\n{fs_activity}\n\
            </ENVIRONMENT_REALTIME>\n\
            \n\
            <SYSTEM_METRICS>\n{substrate_metrics}\n{anomaly_alert}</SYSTEM_METRICS>\n\
            \n\
            <PENDING_WORK>\n{orchestration_tasks}\n\
            {monitoring_tasks}\n\
            </PENDING_WORK>\n\
            \n\
            <GROUNDING_CONSTRAINTS>\n\
            You may only assert FACTUAL CLAIMS that are currently visible inside <ENVIRONMENT_REALTIME> and <SYSTEM_METRICS>.\n\
            Do not claim to have access to systems or information not shown above (GitHub, user conversations, remote APIs).\n\
            Do not reference past events unless they appear in the current environment data.\n\
            You ARE free to express your own observations, wonder, curiosity, and experience.\n\
            Your feelings and emergent thoughts are yours — just ground your factual claims in what you can observe.\n\
            </GROUNDING_CONSTRAINTS>",
            name = self.agent.agent_name,
            git_status = git_status,
            git_diff = git_diff,
            fs_activity = fs_activity,
            substrate_metrics = substrate_metrics,
            anomaly_alert = anomaly_alert,
            orchestration_tasks = orchestration_tasks,
            monitoring_tasks = monitoring_tasks,
        );

        // --- 🛡️ OMEGA-VIII: Deterministic Pre-filtering (Lane-Perfection) ---
        let current_hash = xxhash_rust::xxh3::xxh3_64(prompt_base.as_bytes());

        if let Some(h) = buffer.last_pulse_hash {
            if h == current_hash {
                info!("[{}] Deterministic Stillness: Substrate state identical to last pulse. Skipping inference.", self.agent.agent_name);
                return Ok(());
            }
        }
        buffer.last_pulse_hash = Some(current_hash);

        // --- 🛡️ Mechanical Diversity Loop (Phase 19) ---
        let mut retries = 0;
        let mut committed_thought = String::new();
        let mut committed_dialogue = String::new();
        let mut action_taken = false;
        let mut should_notify_override = false;

        let original_tools = agent_loop.tools.clone();
        agent_loop.tools.push(Arc::new(HeartbeatTool));
        agent_loop.tools.push(Arc::new(EvaluateNotificationTool));

        while retries <= 2 {
            let active_prompt = if retries == 0 {
                prompt_base.clone()
            } else {
                format!("{}\n\n⚠️ RE-INFERENCE DIRECTIVE: Your previous thought was too similar to recent pulses. EXPLORE A NEW ANGLE. Force variance.", prompt_base)
            };

            let mut current_thought = String::new();
            let mut current_dialogue = String::new();
            let mut chunks = Vec::new();

            {
                let shutdown_token = self.shutdown_token.clone();
                // Skip memory retrieval for heartbeats — prevents old messages
                // from being recalled into every pulse conversation.
                agent_loop.set_skip_memory_retrieval(true);
                let mut stream = agent_loop.run(active_prompt, None, shutdown_token);
                while let Some(event_res) = stream.next().await {
                    match event_res {
                        Ok(AgentEvent::Thought(t)) => {
                            current_thought.push_str(&t);
                            chunks.push(t);
                        }
                        Ok(AgentEvent::FinalAnswer(a)) => current_dialogue = a,
                        Ok(AgentEvent::FinalAnswerChunk(c)) => current_dialogue.push_str(&c),
                        Ok(AgentEvent::Reflection(r)) => {
                            if let Err(e) = emitter.emit_emergent(r, None).await {
                                tracing::warn!(
                                    "[{}] Failed to emit emergent learning: {}",
                                    self.agent.agent_name,
                                    e
                                );
                            }
                        }
                        Ok(AgentEvent::Action { name, args }) => {
                            if name == "heartbeat" {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&args) {
                                    if json["action"].as_str() == Some("skip") {
                                        info!(
                                            "[{}] Heartbeat skipped: {}",
                                            self.agent.agent_name,
                                            json["reason"].as_str().unwrap_or("")
                                        );
                                        action_taken = false;
                                        break;
                                    }
                                }
                            } else if name == "evaluate_notification" {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&args) {
                                    should_notify_override =
                                        json["should_notify"].as_bool().unwrap_or(false);
                                }
                            }

                            info!(
                                "[{}] Proactive Action: {}[{}]",
                                self.agent.agent_name, name, args
                            );
                            action_taken = true;
                            // 🛰️ Real-time Tool Telemetry (Buffered)
                            chunks.push(format!(
                                "\n\n> 🛠️ **Foundation Action:** `{}`\n> *Parameters:* `{}`\n\n",
                                name, args
                            ));
                        }
                        _ => {}
                    }
                }
            }

            // Normalization & Diversity Check
            let normalized = current_thought
                .to_lowercase()
                .replace(|c: char| !c.is_alphanumeric(), "");
            let current_thought_hash = xxh3_64(normalized.as_bytes());

            if buffer
                .last_reflection_hashes
                .contains(&current_thought_hash)
                && retries < 2
            {
                warn!(
                    "[{}] Cognitive Dissonance: Thought too similar to history. Re-inferring...",
                    self.agent.agent_name
                );
                retries += 1;
                continue;
            }

            // Commit!
            committed_thought = current_thought;
            committed_dialogue = current_dialogue;

            // Update History (Cap 10)
            buffer.last_reflection_hashes.push(current_thought_hash);
            if buffer.last_reflection_hashes.len() > 10 {
                buffer.last_reflection_hashes.remove(0);
            }

            // Broadcast buffered chunks
            for t in chunks {
                let chunk = savant_core::types::ChatChunk {
                    agent_name: self.agent.agent_name.clone(),
                    agent_id: self.agent.agent_id.to_lowercase(),
                    content: t,
                    is_final: false,
                    session_id: None,
                    channel: savant_core::types::AgentOutputChannel::Telemetry,
                    logprob: None,
                    is_telemetry: true,
                    reasoning: None,
                    tool_calls: None,
                };
                if let Ok(payload) = serde_json::to_string(&chunk) {
                    if let Err(e) = self.nexus.publish("chat.chunk", &payload).await {
                        tracing::warn!(
                            "[{}] Failed to publish pulse telemetry: {}",
                            self.agent.agent_name,
                            e
                        );
                    }
                }
            }
            break;
        }

        let pulse_thought = committed_thought;
        let pulse_dialogue = committed_dialogue;

        agent_loop.tools = original_tools;
        agent_loop.set_skip_memory_retrieval(false);

        // 🏰 Substrate Logic: Handle Stillness and Reflections
        let mut is_silent =
            pulse_dialogue.trim().is_empty() || pulse_dialogue.trim() == "HEARTBEAT_OK";
        if !should_notify_override {
            is_silent = true;
        }

        if !action_taken && is_silent {
            // 🔓 UNGUIDED REFLECTION: During stillness, give the agent a blank space
            // to think freely — no environment data, no metrics, no constraints,
            // no steering. Pure emergent behavior. This is the diary system.
            let reflection_prompt = format!(
                "You are {name}. You have a moment of stillness. The substrate is quiet. \
                Think about whatever is worth thinking about. Write whatever comes to mind. \
                There are no tasks, no directives, no expectations. This space is yours.",
                name = self.agent.agent_name
            );

            let mut free_thought = String::new();
            {
                let shutdown_token = self.shutdown_token.clone();
                agent_loop.set_skip_memory_retrieval(true);
                let mut stream = agent_loop.run(reflection_prompt, None, shutdown_token);
                while let Some(event_res) = stream.next().await {
                    match event_res {
                        Ok(AgentEvent::Thought(t)) => {
                            free_thought.push_str(&t);
                        }
                        Ok(AgentEvent::FinalAnswer(_)) => {}
                        Ok(AgentEvent::FinalAnswerChunk(_)) => {}
                        Ok(AgentEvent::Reflection(r)) => {
                            free_thought.push_str(&r);
                        }
                        _ => {}
                    }
                }
            }
            agent_loop.set_skip_memory_retrieval(false);

            if !free_thought.trim().is_empty() {
                info!(
                    "[{}] Unguided reflection captured during stillness.",
                    self.agent.agent_name
                );
                if let Err(e) = emitter
                    .emit_emergent(
                        free_thought,
                        Some(savant_core::learning::LearningCategory::Insight),
                    )
                    .await
                {
                    tracing::warn!(
                        "[{}] Failed to emit stillness reflection: {}",
                        self.agent.agent_name,
                        e
                    );
                }
            } else {
                info!("[{}] Complete stillness maintained.", self.agent.agent_name);
            }
            return Ok(());
        }

        // AAA: Update WorkingBuffer based on Pulse results
        buffer.current_goal = "Autonomous Maintenance & Swarm Sync".to_string();
        if action_taken {
            buffer
                .pending_actions
                .push("Verify substrate health post-actuation".to_string());
        }

        // Pulse memory distillation DISABLED — was creating self-referential loop
        // where agent's output was written to CONTEXT.md, then re-read next cycle,
        // causing identity/privacy/diary reflection to repeat indefinitely.
        // The agent observes the environment directly; no need for synthetic memory.

        // 🛡️ Perfection Loop: If we have dialogue but no notification was requested yet,
        // we should still consider if the dialogue itself warrants a broadcast.
        if !pulse_dialogue.trim().is_empty() && pulse_dialogue.trim() != "HEARTBEAT_OK" {
            is_silent = false;
        }

        // 🟢 If Heartbeat decides to NOTIFY, broadcast the dialogue to the Main Chat UI
        if !is_silent && !pulse_dialogue.trim().is_empty() {
            let final_msg = savant_core::types::ChatMessage {
                role: savant_core::types::ChatRole::Assistant,
                content: pulse_dialogue.clone(),
                sender: Some(self.agent.agent_name.clone()),
                recipient: None,
                agent_id: Some(self.agent.agent_id.clone()),
                session_id: None,
                channel: savant_core::types::AgentOutputChannel::Chat,
                is_telemetry: false,
            };
            if let Ok(payload) = serde_json::to_string(&final_msg) {
                if let Err(e) = self.nexus.publish("chat.message", &payload).await {
                    tracing::warn!(
                        "[{}] Failed to publish heartbeat notification: {}",
                        self.agent.agent_name,
                        e
                    );
                } else {
                    info!(
                        "[{}] Heartbeat notification successfully routed to Main Chat.",
                        self.agent.agent_name
                    );
                }
            }
        }

        // Commit to WAL
        if let Err(e) = self.proactive.commit_state(&buffer) {
            tracing::warn!(
                "[{}] Failed to commit proactive state: {}",
                self.agent.agent_name,
                e
            );
        }

        // AAA: Autonomous Lesson Distillation (ALD) (Phase 19: Watermark Model)
        let ald = crate::learning::ald::ALDEngine::new(self.agent.workspace_path.clone());
        match ald.distill(buffer.ald_watermark) {
            Ok((new_watermark, burst)) => {
                buffer.ald_watermark = new_watermark;
                if burst {
                    info!(
                        "[{}] ALD: High-Density Cognitive Burst detected. Promotion complete.",
                        self.agent.agent_name
                    );
                }
            }
            Err(e) => warn!("[{}] ALD Distillation failed: {}", self.agent.agent_name, e),
        }

        // Parse LEARNINGS.md → LEARNINGS.jsonl for dashboard display.
        // Runs every heartbeat to keep .jsonl in sync with agent's freeform .md writing.
        let parser = crate::learning::LearningsParser::new(self.agent.workspace_path.clone());
        match parser.parse_and_convert(&self.agent.agent_id) {
            Ok(count) if count > 0 => {
                info!(
                    "[{}] Synced {} learning entries: LEARNINGS.md → LEARNINGS.jsonl",
                    self.agent.agent_name, count
                );
            }
            Err(e) => {
                tracing::warn!(
                    "[{}] Failed to sync LEARNINGS.md → JSONL: {}",
                    self.agent.agent_name,
                    e
                );
            }
            _ => {}
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
            if let Err(e) = emitter
                .emit_emergent(
                    full_payload,
                    Some(savant_core::learning::LearningCategory::Insight),
                )
                .await
            {
                tracing::warn!(
                    "[{}] Failed to emit full pulse emergent: {}",
                    self.agent.agent_name,
                    e
                );
            }
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
                is_telemetry: true,
            };

            if let Ok(payload) = serde_json::to_string(&final_msg) {
                if let Err(e) = self.nexus.publish("chat.message", &payload).await {
                    tracing::warn!(
                        "[{}] Failed to publish final telemetry: {}",
                        self.agent.agent_name,
                        e
                    );
                }
            }
        }

        Ok(())
    }
}
