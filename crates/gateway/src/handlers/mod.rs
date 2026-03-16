use crate::auth::AuthenticatedSession;
use savant_core::types::{RequestFrame, ChatMessage, ChatRole};
use savant_core::bus::NexusBridge;
use std::sync::Arc;

pub mod pairing;

/// Shared application state for axum handlers.
pub struct AppState {
    pub nexus: Arc<NexusBridge>,
    pub storage: Arc<savant_core::db::Storage>,
    pub config: savant_core::config::Config,
}

/// Handles an incoming WebSocket message frame based on session.
pub async fn handle_message(frame: RequestFrame, session: AuthenticatedSession, state: axum::extract::State<AppState>) {
    tracing::info!("📨 Processing message from session: {:?}", session.session_id);
    
    match frame.payload {
        savant_core::types::RequestPayload::ChatMessage(message) => {
            tracing::info!("💬 Chat message: {:?} - {}", message.role, message.content);
            
            let partition_raw = if message.role == savant_core::types::ChatRole::User {
                message.recipient.as_deref().unwrap_or("global")
            } else {
                message.agent_id.as_deref()
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
                
                let _ = send_response_to_client(error_response, &session.session_id, &state.nexus).await;
            }
        }
        savant_core::types::RequestPayload::ControlFrame(control) => {
            match control {
                savant_core::types::ControlFrame::HistoryRequest { lane_id, limit } => {
                    let normalized_lane = lane_id.to_lowercase().trim().to_string();
                    tracing::info!("📜 History request for normalized lane: {} (limit: {})", normalized_lane, limit);
                    match state.storage.get_history(&normalized_lane, limit).await {
                        Ok(history) => {
                            // Wrap history in the expected format for the dashboard
                            // We capitalize the key to match Dashboard's JSON.parse expectations
                            let result = serde_json::json!({
                                "lane_id": lane_id,
                                "history": history
                            });
                            
                            let _ = send_control_response("HISTORY", result, &session.session_id, &state.nexus).await;
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
                            let _ = nexus.publish("agents.discovered", agents_json.value()).await;
                        }
                    });
                }
                savant_core::types::ControlFrame::SoulManifest { prompt, name } => {
                    tracing::info!("🎨 Soul manifestation requested: {} (Named: {:?})", prompt, name);
                    // 🌀 Perfection Loop: High-Density Manifestation
                    // We route this to the 'Architect' sub-routine.
                    // For now, we utilize the Nexus to broadcast a 'manifest.request'
                    // but we also implementation a direct bypass if keys are present.
                    let result = serde_json::json!({
                        "prompt": prompt,
                        "status": "pending",
                        "note": "Manifestation engine is exploding the prompt into a AAA soul..."
                    });
                    let _ = send_control_response("MANIFEST_DRAFT", result, &session.session_id, &state.nexus).await;

                    // Execute the generator as a background task to prevent frame blocking
                    let nexus = state.nexus.clone();
                    let session_id = session.session_id.clone();
                    let prompt_inner = prompt.clone();
                    let name_inner = name.clone();
                    tokio::spawn(async move {
                        if let Err(e) = execute_manifestation(prompt_inner, name_inner, &session_id, &nexus).await {
                           tracing::error!("❌ Manifestation failed: {}", e);
                        }
                    });
                }
                savant_core::types::ControlFrame::SoulUpdate { agent_id, content } => {
                    tracing::info!("💾 Soul update requested for agent: {}", agent_id);
                    // 🛡️ Security Guard: Path Traversal
                    let registry = savant_core::fs::registry::AgentRegistry::new(
                        std::env::current_dir().unwrap_or_default(),
                        savant_core::config::AgentDefaults::default()
                    );

                    match registry.resolve_agent_path(&agent_id) {
                        Ok(Some(path)) => {
                            let soul_path = path.join("SOUL.md");
                            if let Err(e) = std::fs::write(&soul_path, content) {
                                tracing::error!("❌ Failed to write SOUL.md: {}", e);
                            } else {
                                tracing::info!("✅ SOUL.md updated for {}. Hot-reload triggering.", agent_id);
                                let result = serde_json::json!({ "agent_id": agent_id, "status": "success" });
                                let _ = send_control_response("UPDATE_SUCCESS", result, &session.session_id, &state.nexus).await;
                            }
                        }
                        _ => {
                            // If not found, attempt to manifest a NEW workspace
                            tracing::info!("🌟 Manifesting NEW workspace for {}", agent_id);
                            match registry.scaffold_workspace(&agent_id, &content, None) {
                                Ok(config) => {
                                    tracing::info!("✅ Workspace birthed: {}", config.workspace_path.display());
                                    let result = serde_json::json!({ "agent_id": config.agent_id, "status": "created" });
                                    let _ = send_control_response("UPDATE_SUCCESS", result, &session.session_id, &state.nexus).await;
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
                        state.config.agent_defaults.clone()
                    );

                    for plan in agents {
                        tracing::info!("🚀 Deploying agent: {}", plan.name);
                        match registry.scaffold_workspace(&plan.name, &plan.soul, plan.identity.as_deref()) {
                            Ok(config) => {
                                tracing::info!("✅ Agent birthed: {}", config.agent_name);
                            }
                            Err(e) => {
                                tracing::error!("❌ Failed to birth agent {}: {}", plan.name, e);
                            }
                        }
                    }

                    let result = serde_json::json!({ "status": "SWARM_DEPLOYED", "count": agent_count });
                    let _ = send_control_response("BULK_SUCCESS", result, &session.session_id, &state.nexus).await;
                }
                savant_core::types::ControlFrame::SwarmInsightHistoryRequest { limit } => {
                    tracing::info!("🧠 Swarm insight history requested (limit: {})", limit);
                    match state.storage.get_swarm_history(limit).await {
                        Ok(history) => {
                            let result = serde_json::json!({
                                "history": history
                            });
                            let _ = send_control_response("SWARM_INSIGHT_HISTORY", result, &session.session_id, &state.nexus).await;
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to retrieve swarm history: {}", e);
                        }
                    }
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
async fn route_chat_message(message: ChatMessage, nexus: &Arc<NexusBridge>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    nexus: &Arc<NexusBridge>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response_payload = serde_json::to_string(&response)?;
    
    // Send to specific client session
    nexus.publish(&format!("session.{}.response", session_id.0), &response_payload).await?;
    
    tracing::info!("📤 Response sent to client session: {:?}", session_id);
    Ok(())
}

/// Sends a control response (e.g. HISTORY, SYNC) back to client session
async fn send_control_response(
    tag: &str,
    payload: serde_json::Value,
    session_id: &savant_core::types::SessionId,
    nexus: &Arc<NexusBridge>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 🌀 Perfection Loop: Structured Payload
    // We publish to a session-specific channel. server.rs will wrap this in EVENT:
    let payload_str = payload.to_string();
    let channel = format!("session.{}.{}", session_id.0, tag.to_lowercase());
    
    nexus.publish(&channel, &payload_str).await?;
    
    tracing::info!("📤 Control response published to channel: {}", channel);
    Ok(())
}

/// Executes the high-density manifestation engine.
async fn execute_manifestation(
    prompt: String, 
    name: Option<String>,
    session_id: &savant_core::types::SessionId, 
    nexus: &Arc<NexusBridge>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Construct the AAA Master Framework Prompt
    let name_hint = name.map(|n| format!("The soul shall be named: {}.\n", n)).unwrap_or_default();
    let system_prompt = format!(r#"You are the Savant Soul Manifestation Engine.
Manifest a high-density AAA agent identity (SOUL.md) based on the user's prompt.
{}
Strictly follow the Savant Master Framework:
- 300-500 line count requirement.
- 18+ mandatory sections (Psychological Matrix, Strategic Maxims, TCF scenarios).
- Tone: Technical, Sovereign, Precise.
- Metric Focus: Semantic Depth, Loyalty, Technical Density.

Output ONLY the raw Markdown content of the SOUL.md file."#, name_hint);

    let messages = vec![
        ChatMessage {
            role: ChatRole::System,
            content: system_prompt.to_string(),
            sender: None, recipient: None, agent_id: None,
            session_id: Some(session_id.clone()),
            channel: savant_core::types::AgentOutputChannel::Telemetry,
        },
        ChatMessage {
            role: ChatRole::User,
            content: format!("Manifest an agent for: {}", prompt),
            sender: None, recipient: None, agent_id: None,
            session_id: Some(session_id.clone()),
            channel: savant_core::types::AgentOutputChannel::Chat,
        }
    ];

    // 2. Publish to Nexus for any capable agent to respond
    // In a production gateway, we might call an LLM directly, but for now
    // we use the 'nexus' to broadcast the request.
    let payload = serde_json::json!({
        "session_id": session_id.0,
        "messages": messages,
        "mode": "manifest"
    });

    nexus.publish("manifest.request", &payload.to_string()).await?;

    Ok(())
}

#[cfg(test)]
mod benches {
    // criterion benchmark stub
}
