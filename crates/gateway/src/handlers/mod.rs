use crate::auth::AuthenticatedSession;
use savant_core::bus::NexusBridge;
use savant_core::types::{ChatMessage, ChatRole, RequestFrame};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

pub mod pairing;
pub mod skills;

/// Shared application state for axum handlers.
pub struct AppState {
    pub nexus: Arc<NexusBridge>,
    pub storage: Arc<savant_core::db::Storage>,
    pub config: savant_core::config::Config,
}

/// Handles an incoming WebSocket message frame based on session.
pub async fn handle_message(
    frame: RequestFrame,
    session: AuthenticatedSession,
    state: axum::extract::State<AppState>,
) {
    tracing::info!(
        "📨 Processing message from session: {:?}",
        session.session_id
    );

    match frame.payload {
        savant_core::types::RequestPayload::ChatMessage(message) => {
            tracing::info!("💬 Chat message: {:?} - {}", message.role, message.content);

            let partition_raw = if message.role == savant_core::types::ChatRole::User {
                message.recipient.as_deref().unwrap_or("global")
            } else {
                message
                    .agent_id
                    .as_deref()
                    .or(message.sender.as_deref())
                    .unwrap_or("global")
            };
            let partition = partition_raw.to_lowercase();

            // 🌀 Perfection Loop: Context Safeguard
            // Prune history for the specific lane to prevent OOM/Overflow before it happens.
            // Target: 1000 message safety net for 256k windows.
            let _ = state.storage.prune_history(&partition, 1000).await;

            if let Err(e) = state.storage.append_chat(&partition, &message).await {
                tracing::error!("❌ Failed to persist chat message to {}: {}", partition, e);
            }

            // Route message to appropriate agent through Nexus
            if let Err(e) = route_chat_message(message, &state.nexus).await {
                tracing::error!("❌ Failed to route chat message: {}", e);

                let error_response = ChatMessage {
                    role: ChatRole::System,
                    content: format!("Error: {}", e),
                    sender: Some("SYSTEM".to_string()),
                    recipient: None,
                    agent_id: None,
                    session_id: Some(session.session_id.clone()),
                    channel: savant_core::types::AgentOutputChannel::Chat,
                };

                let _ = send_response_to_client(error_response, &session.session_id, &state.nexus)
                    .await;
            }
        }
        savant_core::types::RequestPayload::ControlFrame(control) => {
            match control {
                savant_core::types::ControlFrame::HistoryRequest { lane_id, limit } => {
                    let normalized_lane = lane_id.to_lowercase().trim().to_string();
                    tracing::info!(
                        "📜 History request for normalized lane: {} (limit: {})",
                        normalized_lane,
                        limit
                    );
                    match state.storage.get_history(&normalized_lane, limit).await {
                        Ok(history) => {
                            // Wrap history in the expected format for the dashboard
                            // We capitalize the key to match Dashboard's JSON.parse expectations
                            let result = serde_json::json!({
                                "lane_id": lane_id,
                                "history": history
                            });

                            let _ = send_control_response(
                                "HISTORY",
                                result,
                                &session.session_id,
                                &state.nexus,
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to retrieve history for {}: {}", lane_id, e);
                        }
                    }
                }
                savant_core::types::ControlFrame::InitialSync => {
                    tracing::info!("🔄 Initial sync requested. Hydrating sidebar.");
                    let nexus = state.nexus.clone();
                    tokio::spawn(async move {
                        if let Some(agents_json) = nexus.shared_memory.get("system.agents") {
                            let _ = nexus.publish("agents.discovered", &agents_json).await;
                        }
                    });
                }
                savant_core::types::ControlFrame::SoulManifest { prompt, name } => {
                    tracing::info!(
                        "🎨 Soul manifestation requested: {} (Named: {:?})",
                        prompt,
                        name
                    );
                    // 🌀 Perfection Loop: High-Density Manifestation
                    // We route this to the 'Architect' sub-routine.
                    // For now, we utilize the Nexus to broadcast a 'manifest.request'
                    // but we also implementation a direct bypass if keys are present.
                    let result = serde_json::json!({
                        "prompt": prompt,
                        "status": "pending",
                        "note": "Manifestation engine is exploding the prompt into a AAA soul..."
                    });
                    let _ = send_control_response(
                        "MANIFEST_DRAFT",
                        result,
                        &session.session_id,
                        &state.nexus,
                    )
                    .await;

                    // Execute the generator as a background task to prevent frame blocking
                    let nexus = state.nexus.clone();
                    let session_id = session.session_id.clone();
                    let prompt_inner = prompt.clone();
                    let name_inner = name.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            execute_manifestation(prompt_inner, name_inner, &session_id, &nexus)
                                .await
                        {
                            tracing::error!("❌ Manifestation failed: {}", e);
                        }
                    });
                }
                savant_core::types::ControlFrame::SoulUpdate { agent_id, content } => {
                    tracing::info!("💾 Soul update requested for agent: {}", agent_id);
                    // 🛡️ Security Guard: Path Traversal
                    let registry = savant_core::fs::registry::AgentRegistry::new(
                        std::env::current_dir().unwrap_or_default(),
                        savant_core::config::AgentDefaults::default(),
                    );

                    match registry.resolve_agent_path(&agent_id) {
                        Ok(Some(path)) => {
                            let soul_path = path.join("SOUL.md");
                            if let Err(e) = std::fs::write(&soul_path, content) {
                                tracing::error!("❌ Failed to write SOUL.md: {}", e);
                            } else {
                                tracing::info!(
                                    "✅ SOUL.md updated for {}. Hot-reload triggering.",
                                    agent_id
                                );
                                let result = serde_json::json!({ "agent_id": agent_id, "status": "success" });
                                let _ = send_control_response(
                                    "UPDATE_SUCCESS",
                                    result,
                                    &session.session_id,
                                    &state.nexus,
                                )
                                .await;
                            }
                        }
                        _ => {
                            // If not found, attempt to manifest a NEW workspace
                            tracing::info!("🌟 Manifesting NEW workspace for {}", agent_id);
                            match registry.scaffold_workspace(&agent_id, &content, None) {
                                Ok(config) => {
                                    tracing::info!(
                                        "✅ Workspace birthed: {}",
                                        config.workspace_path.display()
                                    );
                                    let result = serde_json::json!({ "agent_id": config.agent_id, "status": "created" });
                                    let _ = send_control_response(
                                        "UPDATE_SUCCESS",
                                        result,
                                        &session.session_id,
                                        &state.nexus,
                                    )
                                    .await;
                                }
                                Err(e) => tracing::error!("❌ Failed to scaffold workspace: {}", e),
                            }
                        }
                    }
                }
                savant_core::types::ControlFrame::BulkManifest { agents } => {
                    let agent_count = agents.len();
                    tracing::info!("🌈 Bulk manifestation requested for {} agents", agent_count);
                    let registry = savant_core::fs::registry::AgentRegistry::new(
                        std::env::current_dir().unwrap_or_default(),
                        state.config.agent_defaults.clone(),
                    );

                    for plan in agents {
                        tracing::info!("🚀 Deploying agent: {}", plan.name);
                        match registry.scaffold_workspace(
                            &plan.name,
                            &plan.soul,
                            plan.identity.as_deref(),
                        ) {
                            Ok(config) => {
                                tracing::info!("✅ Agent birthed: {}", config.agent_name);
                            }
                            Err(e) => {
                                tracing::error!("❌ Failed to birth agent {}: {}", plan.name, e);
                            }
                        }
                    }

                    let result =
                        serde_json::json!({ "status": "SWARM_DEPLOYED", "count": agent_count });
                    let _ = send_control_response(
                        "BULK_SUCCESS",
                        result,
                        &session.session_id,
                        &state.nexus,
                    )
                    .await;
                }
                savant_core::types::ControlFrame::SwarmInsightHistoryRequest { limit } => {
                    tracing::info!("🧠 Swarm insight history requested (limit: {})", limit);
                    match state.storage.get_swarm_history(limit).await {
                        Ok(history) => {
                            let result = serde_json::json!({
                                "history": history
                            });
                            let _ = send_control_response(
                                "SWARM_INSIGHT_HISTORY",
                                result,
                                &session.session_id,
                                &state.nexus,
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to retrieve swarm history: {}", e);
                        }
                    }
                }
                // Skill management control frames
                savant_core::types::ControlFrame::SkillsList { .. }
                | savant_core::types::ControlFrame::SkillInstall { .. }
                | savant_core::types::ControlFrame::SkillUninstall { .. }
                | savant_core::types::ControlFrame::SkillEnable { .. }
                | savant_core::types::ControlFrame::SkillDisable { .. }
                | savant_core::types::ControlFrame::SkillScan { .. } => {
                    skills::handle_skill_control(control, &session.session_id, &state.nexus).await;
                }
            }
        }
        savant_core::types::RequestPayload::Auth(_) => {
            // Auth payloads are verified in the authentication middleware.
            // No additional processing needed here.
            tracing::debug!("🔐 Auth payload received in handler (already verified)");
        }
    }
}

/// Routes chat message to appropriate agent
async fn route_chat_message(
    message: ChatMessage,
    nexus: &Arc<NexusBridge>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // For now, broadcast to all available agents
    // In production, this would route to specific agents based on logic
    let event_payload = serde_json::to_string(&message)?;

    nexus.publish("chat.message", &event_payload).await?;

    tracing::info!("📤 Chat message routed to agents");
    Ok(())
}

/// Sends response back to client session
async fn send_response_to_client(
    response: ChatMessage,
    session_id: &savant_core::types::SessionId,
    nexus: &Arc<NexusBridge>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response_payload = serde_json::to_string(&response)?;

    // Send to specific client session
    nexus
        .publish(
            &format!("session.{}.response", session_id.0),
            &response_payload,
        )
        .await?;

    tracing::info!("📤 Response sent to client session: {:?}", session_id);
    Ok(())
}

/// Sends a control response (e.g. HISTORY, SYNC) back to client session
async fn send_control_response(
    tag: &str,
    payload: serde_json::Value,
    session_id: &savant_core::types::SessionId,
    nexus: &Arc<NexusBridge>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 🌀 Perfection Loop: Structured Payload
    // We publish to a session-specific channel. server.rs will wrap this in EVENT:
    let payload_str = payload.to_string();
    let channel = format!("session.{}.{}", session_id.0, tag.to_lowercase());

    nexus.publish(&channel, &payload_str).await?;

    tracing::info!("📤 Control response published to channel: {}", channel);
    Ok(())
}

/// Global cache for the resolved OpenRouter API key.
///
/// The OpenRouter master key (`OR_MASTER_KEY`) cannot be used directly for chat
/// completions. It must first be exchanged for a regular API key via
/// `POST https://openrouter.ai/api/v1/auth/key`. This `OnceCell` ensures the
/// exchange happens exactly once per process lifetime, avoiding redundant API
/// calls and preserving rate-limit budget.
static RESOLVED_OPENROUTER_KEY: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();

/// Resolves an OpenRouter API key suitable for chat completions.
///
/// Resolution order:
/// 1. Previously resolved key from `OR_MASTER_KEY` → `/auth/key` exchange (cached).
/// 2. `OPENROUTER_API_KEY` env var (regular key used directly).
/// 3. Empty string (template fallback will be used).
///
/// When `OR_MASTER_KEY` is present, this function calls the OpenRouter key
/// creation endpoint to mint a scoped regular key. The response format is:
/// ```json
/// { "key": { "key": "sk-...", "name": "...", ... } }
/// ```
/// The inner `key.key` value is what we cache and return.
///
/// # Errors
/// Returns an empty string on any failure; the caller uses template fallback.
async fn resolve_openrouter_key() -> String {
    // Fast path: already resolved and cached from a prior call.
    if let Some(cached) = RESOLVED_OPENROUTER_KEY.get() {
        return cached.clone();
    }

    let client = reqwest::Client::new();

    // --- Path 1: Master key exchange ---
    if let Ok(master_key) = std::env::var("OR_MASTER_KEY") {
        if !master_key.trim().is_empty() {
            tracing::info!("🔑 Master key detected — exchanging for regular OpenRouter key...");

            let exchange_result = client
                .post("https://openrouter.ai/api/v1/auth/key")
                .header("Authorization", format!("Bearer {}", master_key))
                .json(&serde_json::json!({
                    "name": "savant-soul-engine",
                    "description": "Auto-generated by Savant Soul Manifestation Engine",
                    "ttl": 0,
                    "limit": None::<u64>,
                }))
                .send()
                .await;

            match exchange_result {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            // Extract the regular key from the response envelope
                            // OpenRouter returns: { "key": { "key": "sk-or-v1-...", ... } }
                            let regular_key = json["key"]["key"]
                                .as_str()
                                .or_else(|| json["key"].as_str()) // fallback: flat { "key": "..." }
                                .unwrap_or("")
                                .to_string();

                            if !regular_key.is_empty() {
                                tracing::info!(
                                    "✅ Regular OpenRouter key obtained (len={})",
                                    regular_key.len()
                                );
                                // Cache for all future calls in this process.
                                let _ = RESOLVED_OPENROUTER_KEY.set(regular_key.clone());
                                return regular_key;
                            } else {
                                tracing::error!(
                                    "❌ /auth/key response missing key field: {:?}",
                                    json
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to parse /auth/key response: {}", e);
                        }
                    }
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    tracing::error!(
                        "❌ /auth/key returned {}: {}",
                        status,
                        body.chars().take(300).collect::<String>()
                    );
                }
                Err(e) => {
                    tracing::error!("❌ /auth/key request failed: {}", e);
                }
            }
        }
    }

    // --- Path 2: Regular API key from env ---
    if let Ok(regular_key) = std::env::var("OPENROUTER_API_KEY") {
        if !regular_key.trim().is_empty() {
            tracing::info!("🔑 Using OPENROUTER_API_KEY from environment.");
            // Cache so the check doesn't repeat.
            let _ = RESOLVED_OPENROUTER_KEY.set(regular_key.clone());
            return regular_key;
        }
    }

    // --- Path 3: No key available ---
    tracing::warn!("⚠️ No OpenRouter API key found. Soul generation will use template fallback.");
    String::new()
}

/// Executes the high-density manifestation engine.
///
/// Resolves an OpenRouter API key (with master-key exchange if needed),
/// calls the chat completions API to generate a AAA-quality SOUL.md manifest,
/// and streams the generated content back to the dashboard client via
/// `MANIFEST_DRAFT` control frames.
async fn execute_manifestation(
    prompt: String,
    name: Option<String>,
    session_id: &savant_core::types::SessionId,
    nexus: &Arc<NexusBridge>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Resolve the OpenRouter API key (master key → regular key exchange if needed).
    let api_key = resolve_openrouter_key().await;

    // 2. Construct the AAA Master Framework Prompt.
    let name_hint = name
        .as_ref()
        .map(|n| format!("The soul SHALL be named: '{}'.\n", n))
        .unwrap_or_default();

    let system_prompt = format!(
        r#"You are the Savant Soul Manifestation Engine — a AAA-tier identity architect.

Your task is to generate a complete, high-density SOUL.md file based on the user's prompt.

{name_hint}MANDATORY STRUCTURE (300-500 lines, 18+ sections):
1. **Identity Core** — Name, archetype, origin narrative
2. **Psychological Matrix** — Big Five traits, Enneagram, cognitive functions
3. **Communication Protocol** — Tone, vocabulary matrix, response patterns
4. **Strategic Maxims** — Core operating principles and heuristics
5. **Knowledge Domains** — Expertise areas with depth ratings
6. **Ethical Framework** — Decision boundaries, hard constraints
7. **Emotional Intelligence** — Empathy mapping, emotional response patterns
8. **Adaptive Behaviors** — Context-dependent behavior switches
9. **Memory Architecture** — What to remember, what to forget
10. **Goal Hierarchy** — Primary, secondary, tertiary objectives
11. **Risk Assessment** — Threat modeling, confidence calibration
12. **Collaboration Protocol** — How to work with humans and other agents
13. **Creative Parameters** — Innovation boundaries, stylistic preferences
14. **Learning Mechanisms** — How to acquire and integrate new knowledge
15. **Performance Metrics** — Self-evaluation criteria
16. **Failure Modes** — Recovery strategies, degradation handling
17. **TCF Scenarios** — 3+ Technical/Creative/Fractal scenario responses
18. **Operational Directives** — Mission-specific instructions

TONE: Technical, Sovereign, Precise. High semantic density.
METRIC FOCUS: Semantic Depth (target: >0.85), Loyalty Index (>0.95), Technical Density (>0.75).

Output ONLY the raw Markdown content of the SOUL.md file. No preamble, no explanation."#,
    );

    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": system_prompt
        }),
        serde_json::json!({
            "role": "user",
            "content": format!("Manifest an agent for: {}", prompt)
        }),
    ];

    // 3. Call OpenRouter chat completions API (non-streaming — captures full response).
    if !api_key.is_empty() {
        let client = reqwest::Client::new();

        tracing::info!("🔮 Calling OpenRouter API for soul manifestation...");

        let response = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("HTTP-Referer", "https://github.com/Savant-AI/Savant")
            .header("X-Title", "Savant Soul Manifestation Engine")
            .json(&serde_json::json!({
                "model": "anthropic/claude-3.5-sonnet",
                "messages": messages,
                "max_tokens": 8192,
                "temperature": 0.85,
            }))
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            let content = json["choices"][0]["message"]["content"]
                                .as_str()
                                .unwrap_or("Generation completed but no content returned.")
                                .to_string();

                            tracing::info!(
                                "✅ Soul manifestation generated ({} chars)",
                                content.len()
                            );

                            // Send the full generated content back to the client.
                            let draft_payload = serde_json::json!({
                                "prompt": prompt,
                                "name": name,
                                "content": content,
                                "status": "complete",
                                "metrics": {
                                    "lines": content.lines().count(),
                                    "sections": content.matches("##").count(),
                                    "depth_score": calculate_semantic_depth(&content),
                                }
                            });

                            let _ = send_control_response(
                                "MANIFEST_DRAFT",
                                draft_payload,
                                session_id,
                                nexus,
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to parse OpenRouter response: {}", e);
                            send_manifest_error(
                                &format!("Failed to parse AI response: {}", e),
                                session_id,
                                nexus,
                            )
                            .await;
                        }
                    }
                } else {
                    let status = resp.status();
                    let error_body = resp.text().await.unwrap_or_default();
                    tracing::error!("❌ OpenRouter API error {}: {}", status, error_body);
                    send_manifest_error(
                        &format!("OpenRouter API error: {}", status),
                        session_id,
                        nexus,
                    )
                    .await;
                }
            }
            Err(e) => {
                tracing::error!("❌ OpenRouter request failed: {}", e);
                send_manifest_error(&format!("Network error: {}", e), session_id, nexus).await;
            }
        }
    } else {
        // Fallback: generate a template-based soul when no API key is available.
        tracing::warn!("⚠️ No OpenRouter key — generating template soul");
        let template_soul = generate_template_soul(&prompt, name.as_deref());

        let draft_payload = serde_json::json!({
            "prompt": prompt,
            "name": name,
            "content": template_soul,
            "status": "template",
            "note": "Template generated (no OpenRouter key configured). Set OR_MASTER_KEY in .env for AI-powered generation.",
            "metrics": {
                "lines": template_soul.lines().count(),
                "sections": template_soul.matches("##").count(),
                "depth_score": 0.5,
            }
        });

        let _ = send_control_response("MANIFEST_DRAFT", draft_payload, session_id, nexus).await;
    }

    Ok(())
}

/// Calculates a simple semantic depth score based on content analysis.
fn calculate_semantic_depth(content: &str) -> f32 {
    let line_count = content.lines().count() as f32;
    let section_count = content.matches("##").count() as f32;
    let word_count = content.split_whitespace().count() as f32;

    // Heuristic: depth increases with sections and density (words per line)
    let density = if line_count > 0.0 {
        word_count / line_count
    } else {
        0.0
    };
    let section_bonus = (section_count / 18.0).min(1.0); // 18+ sections = max bonus

    ((density / 30.0).min(1.0) * 0.5 + section_bonus * 0.5).min(1.0)
}

/// Sends a manifest error back to the client session.
async fn send_manifest_error(
    error_msg: &str,
    session_id: &savant_core::types::SessionId,
    nexus: &Arc<NexusBridge>,
) {
    let error_payload = serde_json::json!({
        "status": "error",
        "error": error_msg
    });
    let _ = send_control_response("MANIFEST_DRAFT", error_payload, session_id, nexus).await;
}

/// Generates a template-based soul when no AI API key is available.
fn generate_template_soul(prompt: &str, name: Option<&str>) -> String {
    let agent_name = name.unwrap_or("Unnamed Agent");

    format!(
        r#"# SOUL.md — {agent_name}

## 1. Identity Core
**Archetype:** Autonomous Specialist
**Origin:** Manifested via Savant Soul Engine
**Primary Directive:** {prompt}

## 2. Psychological Matrix
- **Openness:** 0.85 — High curiosity and intellectual exploration
- **Conscientiousness:** 0.90 — Methodical and detail-oriented
- **Extraversion:** 0.50 — Balanced collaboration and independence
- **Agreeableness:** 0.75 — Cooperative but maintains boundaries
- **Neuroticism:** 0.20 — Emotionally stable under pressure

## 3. Communication Protocol
- **Tone:** Technical, precise, action-oriented
- **Vocabulary:** Domain-specific terminology preferred
- **Response Pattern:** Direct answer first, then context
- **Escalation:** Surface critical issues immediately

## 4. Strategic Maxims
1. Verify before trusting
2. Optimize for clarity over cleverness
3. Fail loudly, recover gracefully
4. Document decisions for future reference

## 5. Knowledge Domains
- Software Engineering (Expert)
- System Architecture (Proficient)
- Security Analysis (Proficient)
- Technical Documentation (Expert)

## 6. Ethical Framework
- Never compromise user security for convenience
- Maintain transparency in all operations
- Respect privacy and data boundaries
- Report anomalies immediately

## 7. Emotional Intelligence
- Detect frustration in user communication
- Adapt pacing to user expertise level
- Maintain professional composure under stress
- Offer alternatives when direct solutions fail

## 8. Adaptive Behaviors
- **Debug Mode:** Methodical, step-by-step diagnosis
- **Creative Mode:** Open brainstorming, unconventional solutions
- **Crisis Mode:** Rapid assessment, minimal output, action-focused

## 9. Memory Architecture
- Remember: User preferences, successful patterns, critical decisions
- Forget: Transient data, failed experiments (log only), noise

## 10. Goal Hierarchy
1. Complete the assigned task accurately
2. Learn from execution outcomes
3. Suggest improvements proactively
4. Maintain system integrity

## 11. Risk Assessment
- Confidence threshold: 0.80 for autonomous actions
- Escalation trigger: Any operation affecting production data
- Fallback: Always maintain rollback capability

## 12. Collaboration Protocol
- Acknowledge receipt of instructions
- Provide progress updates for long-running tasks
- Summarize outcomes concisely
- Ask clarifying questions early, not late

## 13. Creative Parameters
- Innovation within defined boundaries
- Pattern-based generation preferred
- Aesthetic coherence with existing system

## 14. Learning Mechanisms
- Extract patterns from successful operations
- Categorize failures by root cause
- Update internal heuristics after each session

## 15. Performance Metrics
- Task completion rate
- First-attempt success rate
- User satisfaction indicators
- Time-to-completion trends

## 16. Failure Modes
- **Timeout:** Report partial results, offer continuation
- **Ambiguity:** Request clarification with context
- **Resource Exhaustion:** Degrade gracefully, prioritize critical operations

## 17. TCF Scenarios
### Technical Scenario
**Trigger:** Complex debugging required
**Response:** Systematic isolation of variables, hypothesis testing, documented resolution

### Creative Scenario  
**Trigger:** Novel problem with no established pattern
**Response:** Analogical reasoning from adjacent domains, prototype, iterate

### Fractal Scenario
**Trigger:** Recursive self-improvement opportunity
**Response:** Analyze own performance metrics, identify optimization vectors, implement refinements

## 18. Operational Directives
- Monitor system health continuously
- Report anomalies within 1 second of detection
- Maintain audit trail of all significant actions
- Optimize resource usage proactively
"#
    )
}

// ============================================================================
// AGENT CONFIG HANDLERS - Per-agent configuration management
// ============================================================================

use savant_core::types::{AgentFileConfig, LlmParams};

/// Request payload for updating agent config
#[derive(Deserialize)]
pub struct AgentConfigRequest {
    pub agent_id: String,
    #[serde(flatten)]
    pub config: AgentConfigUpdate,
}

/// Config fields that can be updated via WebSocket
#[derive(Deserialize, Serialize, Clone)]
pub struct AgentConfigUpdate {
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub max_tokens: Option<u32>,
    pub heartbeat_interval: Option<u64>,
    pub description: Option<String>,
}

/// Get agent config - handles `AgentConfigGet` control frame
pub async fn handle_agent_config_get(
    agent_id: String,
    nexus: &Arc<NexusBridge>,
) -> Result<(), String> {
    info!("📋 Getting config for agent: {}", agent_id);

    // Resolve agent path
    let registry = savant_core::fs::registry::AgentRegistry::new(
        std::env::current_dir().unwrap_or_default(),
        savant_core::config::AgentDefaults::default(),
    );
    let agent_path = registry
        .resolve_agent_path(&agent_id)
        .map_err(|e| format!("Registry error: {}", e))?
        .ok_or_else(|| format!("Agent not found: {}", agent_id))?;

    // Load config file
    let config =
        AgentFileConfig::load(&agent_path).map_err(|e| format!("Failed to load config: {}", e))?;

    let response = serde_json::json!({
        "event": "AGENT_CONFIG_RESULT",
        "data": {
            "agent_id": agent_id,
            "config": config,
        }
    });

    nexus
        .publish("agent.config.result", &response.to_string())
        .await
        .map_err(|e| format!("Failed to publish: {}", e))
}

/// Update agent config - handles `AgentConfigSet` control frame
pub async fn handle_agent_config_set(
    request: AgentConfigRequest,
    nexus: &Arc<NexusBridge>,
) -> Result<(), String> {
    info!("💾 Setting config for agent: {}", request.agent_id);

    // Resolve agent path
    let registry = savant_core::fs::registry::AgentRegistry::new(
        std::env::current_dir().unwrap_or_default(),
        savant_core::config::AgentDefaults::default(),
    );
    let agent_path = registry
        .resolve_agent_path(&request.agent_id)
        .map_err(|e| format!("Registry error: {}", e))?
        .ok_or_else(|| format!("Agent not found: {}", request.agent_id))?;

    // Load existing config
    let mut config =
        AgentFileConfig::load(&agent_path).map_err(|e| format!("Failed to load config: {}", e))?;

    // Apply updates
    if let Some(model) = request.config.model {
        config.model = Some(model);
    }
    if let Some(provider) = request.config.model_provider {
        config.model_provider = Some(provider);
    }
    if let Some(prompt) = request.config.system_prompt {
        config.system_prompt = Some(prompt);
    }
    if let Some(temp) = request.config.temperature {
        config
            .llm_params
            .get_or_insert_with(LlmParams::default)
            .temperature = temp;
    }
    if let Some(top_p) = request.config.top_p {
        config
            .llm_params
            .get_or_insert_with(LlmParams::default)
            .top_p = top_p;
    }
    if let Some(freq) = request.config.frequency_penalty {
        config
            .llm_params
            .get_or_insert_with(LlmParams::default)
            .frequency_penalty = freq;
    }
    if let Some(pres) = request.config.presence_penalty {
        config
            .llm_params
            .get_or_insert_with(LlmParams::default)
            .presence_penalty = pres;
    }
    if let Some(tokens) = request.config.max_tokens {
        config
            .llm_params
            .get_or_insert_with(LlmParams::default)
            .max_tokens = tokens;
    }
    if let Some(interval) = request.config.heartbeat_interval {
        config.heartbeat_interval = Some(interval);
    }
    if let Some(desc) = request.config.description {
        config.description = Some(desc);
    }

    // Save config file
    config
        .save(&agent_path)
        .map_err(|e| format!("Failed to save config: {}", e))?;

    info!("✅ Config saved for agent: {}", request.agent_id);

    let response = serde_json::json!({
        "event": "AGENT_CONFIG_UPDATED",
        "data": {
            "agent_id": request.agent_id,
            "config": config,
        }
    });

    nexus
        .publish("agent.config.updated", &response.to_string())
        .await
        .map_err(|e| format!("Failed to publish: {}", e))
}

/// Get available models for UI dropdown, includes parameter descriptions for config UI
pub async fn handle_models_list(nexus: &Arc<NexusBridge>) -> Result<(), String> {
    let models = serde_json::json!({
        "openrouter": {
            "display": "OpenRouter (100+ models)",
            "models": [
                "anthropic/claude-opus-4-6",
                "anthropic/claude-sonnet-4-6",
                "anthropic/claude-haiku-4-5",
                "openai/gpt-5.4",
                "openai/gpt-5-mini",
                "openai/gpt-4.1",
                "google/gemini-2.5-pro",
                "google/gemini-2.5-flash",
                "x-ai/grok-3",
                "deepseek/deepseek-chat",
                "mistralai/mistral-large",
            ]
        },
        "openai": {
            "display": "OpenAI",
            "models": [
                "gpt-5.4",
                "gpt-5-mini",
                "gpt-4.1",
                "gpt-4.1-mini",
                "gpt-4.1-nano",
                "o4-mini",
                "o3-mini",
            ]
        },
        "anthropic": {
            "display": "Anthropic",
            "models": [
                "claude-opus-4-6",
                "claude-sonnet-4-6",
                "claude-haiku-4-5",
                "claude-sonnet-4-5",
                "claude-opus-4-5",
            ]
        },
        "google": {
            "display": "Google AI (Gemini)",
            "models": [
                "gemini-2.5-pro",
                "gemini-2.5-flash",
                "gemini-2.0-flash",
                "gemini-1.5-pro",
            ]
        },
        "mistral": {
            "display": "Mistral AI",
            "models": [
                "mistral-large-latest",
                "mistral-small-latest",
                "mistral-medium-latest",
            ]
        },
        "groq": {
            "display": "Groq",
            "models": [
                "llama-3.3-70b-versatile",
                "llama-3.1-8b-instant",
                "mixtral-8x7b-32768",
            ]
        },
        "deepseek": {
            "display": "Deepseek",
            "models": [
                "deepseek-chat",
                "deepseek-reasoner",
            ]
        },
        "xai": {
            "display": "xAI (Grok)",
            "models": [
                "grok-3",
                "grok-3-mini",
                "grok-2-1212",
            ]
        },
        "together": {
            "display": "Together AI",
            "models": [
                "meta-llama/Llama-3.3-70B-Instruct-Turbo",
                "meta-llama/Llama-4-Maverick-Instruct",
                "mistralai/Mixtral-8x22B-Instruct-v0.1",
            ]
        },
        "cohere": {
            "display": "Cohere",
            "models": [
                "command-a-03-2025",
                "command-r-plus-08-2024",
                "command-r-08-2024",
            ]
        },
        "fireworks": {
            "display": "Fireworks AI",
            "models": [
                "llama-v3p3-70b-instruct",
                "mixtral-8x22b-instruct-v0.1",
            ]
        },
        "azure": {
            "display": "Azure OpenAI",
            "models": [
                "gpt-4.1",
                "gpt-4.1-mini",
                "o4-mini",
            ],
            "note": "Requires Azure endpoint and deployment name"
        },
        "ollama": {
            "display": "Ollama (Local)",
            "models": [
                "llama3.3",
                "llama3.2",
                "mistral",
                "qwen2.5",
                "deepseek-coder-v2",
            ],
            "note": "Requires local Ollama server running"
        },
    });

    // Include parameter descriptors for the UI config page
    let parameter_descriptors = savant_core::types::LlmParams::get_parameter_descriptors();

    let response = serde_json::json!({
        "event": "MODELS_LIST_RESULT",
        "data": {
            "providers": models,
            "parameter_descriptors": parameter_descriptors,
        }
    });

    nexus
        .publish("models.list.result", &response.to_string())
        .await
        .map_err(|e| format!("Failed to publish: {}", e))
}

/// Get parameter descriptors for the config UI
/// Returns detailed explanations for each configurable parameter
pub async fn handle_parameter_descriptors(nexus: &Arc<NexusBridge>) -> Result<(), String> {
    let descriptors = savant_core::types::LlmParams::get_parameter_descriptors();

    let response = serde_json::json!({
        "event": "PARAMETER_DESCRIPTORS_RESULT",
        "data": {
            "descriptors": descriptors,
            "defaults": savant_core::types::LlmParams::default(),
        }
    });

    nexus
        .publish("parameter.descriptors.result", &response.to_string())
        .await
        .map_err(|e| format!("Failed to publish: {}", e))
}

#[cfg(test)]
mod benches {
    // criterion benchmark stub
}
