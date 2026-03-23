use crate::auth;
use crate::lanes::SessionLane;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use lru::LruCache;
use savant_core::bus::NexusBridge;
use savant_core::config::Config;
use savant_core::db::Storage;
use savant_core::error::SavantError;
use savant_core::types::{RequestFrame, SessionId};
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use tower_http::cors::{Any, CorsLayer};

/// Shared state for the gateway server.
pub struct GatewayState {
    pub config: Config,
    pub sessions: DashMap<SessionId, Arc<SessionLane>>,
    pub nexus: Arc<NexusBridge>,
    pub storage: Arc<Storage>,
    pub avatar_cache: TokioMutex<LruCache<String, (Vec<u8>, String)>>,
    /// Persistent gateway Ed25519 signing key (generated once at startup)
    pub gateway_signing_key: ed25519_dalek::SigningKey,
}

/// Starts the axum gateway server.
pub async fn start_gateway(
    config: Config,
    nexus: Arc<NexusBridge>,
    storage: Arc<Storage>,
) -> Result<(), SavantError> {
    let addr = format!("{}:{}", config.server.host, config.server.port)
        .parse::<SocketAddr>()
        .map_err(|e| SavantError::Unknown(format!("Invalid address: {}", e)))?;

    let state = Arc::new(GatewayState {
        config: config.clone(),
        sessions: DashMap::new(),
        nexus,
        storage,
        avatar_cache: TokioMutex::new(LruCache::new(
            NonZeroUsize::new(100).expect("100 is non-zero"),
        )),
        gateway_signing_key: ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .route("/api/agents/:name/image", get(agent_image_handler))
        .route(
            "/api/settings",
            get(settings_get_handler).post(settings_post_handler),
        )
        .route(
            "/api/settings/reset",
            get(settings_reset_handler).post(settings_reset_handler),
        )
        .route("/api/models", get(models_get_handler))
        .route(
            "/api/mcp/servers",
            get(crate::handlers::mcp::list_servers_handler),
        )
        .route(
            "/api/mcp/servers/install",
            axum::routing::post(crate::handlers::mcp::install_server_handler),
        )
        .route(
            "/api/mcp/servers/add",
            axum::routing::post(crate::handlers::mcp::add_server_handler),
        )
        .route(
            "/api/mcp/servers/remove",
            axum::routing::post(crate::handlers::mcp::remove_server_handler),
        )
        .route(
            "/api/mcp/servers/uninstall",
            axum::routing::post(crate::handlers::mcp::uninstall_server_handler),
        )
        .route(
            "/api/mcp/servers/info",
            get(crate::handlers::mcp::server_info_handler),
        )
        .route("/live", get(|| async { "OK" }))
        .route("/ready", get(|| async { "OK" }))
        .route(
            "/api/setup/check",
            get(crate::handlers::setup::setup_check_handler),
        )
        .route(
            "/api/setup/install-model",
            axum::routing::post(crate::handlers::setup::setup_install_model_handler),
        )
        .layer(cors)
        .with_state(state);

    tracing::info!("Gateway server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .await
        .map_err(SavantError::IoError)?;

    Ok(())
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<GatewayState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<GatewayState>) {
    tracing::info!("New WebSocket connection established");
    let (mut sender, mut receiver) = socket.split();

    // 1. Authentication Phase
    let auth_frame = match receiver.next().await {
        Some(Ok(Message::Text(text))) => match serde_json::from_str::<RequestFrame>(&text) {
            Ok(frame) => frame,
            Err(e) => {
                tracing::error!("Failed to parse auth frame: {}", e);
                return;
            }
        },
        Some(Ok(Message::Close(_))) => {
            tracing::debug!("WebSocket closed during auth phase");
            return;
        }
        _ => return,
    };

    let dashboard_key = state.config.server.dashboard_api_key.as_deref();
    let session_context = match auth::authenticate(&auth_frame, dashboard_key).await {
        Ok(ctx) => ctx,
        Err(e) => {
            tracing::error!("Authentication failed: {}", e);
            let _ = sender
                .send(Message::Text("Authentication failed".to_string()))
                .await;
            return;
        }
    };

    let session_id = session_context.session_id.clone();
    tracing::info!("Session authenticated: {}", session_id.0);

    // 2. Sovereign Handshake Ignition: Send current agents immediately upon auth
    // This ensures zero-latency sidebar population for the Dashboard.
    let initial_agents = state.nexus.shared_memory.get("system.agents");

    let agents_payload = if let Some(json) = initial_agents {
        json
    } else {
        // Perfection Enhancement: Send empty discovery to acknowledge sync
        serde_json::json!({ "status": "SWARM_PENDING", "agents": [] }).to_string()
    };

    let event = savant_core::types::EventFrame {
        event_type: "agents.discovered".to_string(),
        payload: agents_payload,
    };
    let msg = match serde_json::to_string(&event) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to serialize agents.discovered event: {}", e);
            return;
        }
    };
    let _ = sender.send(Message::Text(format!("EVENT:{}", msg))).await;
    tracing::info!(
        "🚀 Sovereign Ignition: Hydrated sidebar for session {}",
        session_id.0
    );

    // 3. Outgoing Message Hub
    // We use a central MPSC to funnel both Lane responses and Swarm telemetry
    let (outgoing_tx, mut outgoing_rx) = tokio::sync::mpsc::channel::<Message>(100);

    // 3. Session Setup
    let (lane, lane_rx, mut res_rx, limit) = SessionLane::new(
        state.config.server.lane_capacity,
        state.config.server.max_lane_concurrency,
    );
    let lane = Arc::new(lane);

    state.sessions.insert(session_id.clone(), lane.clone());
    SessionLane::spawn_consumer(
        lane_rx,
        lane.response_tx.clone(),
        limit,
        state.nexus.clone(),
    );

    // 4. Task 1: Forward Lane Responses to Outgoing Hub
    let out_tx = outgoing_tx.clone();
    let mut lane_fwd_task = tokio::spawn(async move {
        while let Some(response) = res_rx.recv().await {
            let msg = match serde_json::to_string(&response) {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!("Failed to serialize lane response: {}", e);
                    continue;
                }
            };
            let _ = out_tx.send(Message::Text(msg)).await;
        }
    });

    // 5. Consolidated Swarm Telemetry Task
    let out_tx = outgoing_tx.clone();
    let storage_clone = state.storage.clone();
    let mut event_rx = state.nexus.event_bus.subscribe();
    let session_id_telemetry = session_id.clone();

    let mut telemetry_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            // 🌀 Perfection Loop: Unified Protocol
            let mut outbound_event = event.clone();

            match event.event_type.as_str() {
                "chat.message" | "chat.chunk" => {
                    // Protocol is already standardized at the Agent layer
                }
                t if t.starts_with("system.") => {
                    // System-wide configuration or status updates
                }
                t if t.starts_with("session.") => {
                    let parts: Vec<&str> = t.split('.').collect();
                    outbound_event.event_type = parts.get(2).unwrap_or(&"response").to_string();
                    if !t.starts_with(&format!("session.{}.", session_id_telemetry.0)) {
                        continue;
                    }
                }
                _ => {}
            }

            // Persistence for dialog ONLY
            if outbound_event.event_type == "chat.message" {
                if let Ok(msg) =
                    serde_json::from_str::<savant_core::types::ChatMessage>(&outbound_event.payload)
                {
                    if msg.channel == savant_core::types::AgentOutputChannel::Chat {
                        if let Err(e) = crate::persistence::GatewayPersistence::persist_chat(
                            &storage_clone,
                            &msg,
                        )
                        .await
                        {
                            tracing::warn!("Failed to persist chat message: {}", e);
                        }
                    }
                }
            } else if outbound_event.event_type == "learning.insight" {
                if let Ok(learning) = serde_json::from_str::<savant_core::learning::EmergentLearning>(
                    &outbound_event.payload,
                ) {
                    let msg = savant_core::types::ChatMessage {
                        is_telemetry: false,
                        role: savant_core::types::ChatRole::System,
                        content: format!("Insight: {}", learning.content),
                        sender: Some("ALD".to_string()),
                        recipient: None,
                        agent_id: None,
                        session_id: Some(savant_core::types::SessionId("learnings".to_string())),
                        channel: savant_core::types::AgentOutputChannel::Telemetry,
                    };
                    if let Err(e) =
                        crate::persistence::GatewayPersistence::persist_chat(&storage_clone, &msg)
                            .await
                    {
                        tracing::warn!("Failed to persist learning insight: {}", e);
                    }
                }
            }

            let msg = match serde_json::to_string(&outbound_event) {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!("Failed to serialize telemetry event: {}", e);
                    continue;
                }
            };
            let _ = out_tx.send(Message::Text(format!("EVENT:{}", msg))).await;
        }
    });

    // 5b. Debug Log Forwarding Task — streams tracing output to dashboard
    let out_tx = outgoing_tx.clone();
    let mut debug_log_task = tokio::spawn(async move {
        let mut log_rx = savant_core::bus::subscribe_debug_logs();
        while let Ok(log_msg) = log_rx.recv().await {
            let event = savant_core::types::EventFrame {
                event_type: "debug.log".to_string(),
                payload: serde_json::json!({ "message": log_msg }).to_string(),
            };
            if let Ok(msg) = serde_json::to_string(&event) {
                let _ = out_tx.send(Message::Text(format!("EVENT:{}", msg))).await;
            }
        }
    });

    // 6. Task 3: Central WebSocket Sender
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = outgoing_rx.recv().await {
            if let Err(e) = sender.send(msg).await {
                tracing::error!("WS send failure: {}", e);
                break;
            }
        }
    });

    // 7. Task 4: WebSocket Receiver
    let storage = state.storage.clone();
    let nexus_inner = state.nexus.clone();
    let config_inner = state.config.clone();
    let mut recv_task = tokio::spawn({
        let session_id = session_id.clone();
        let session_context_clone = session_context.clone();
        async move {
            while let Some(msg_result) = receiver.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        if let Ok(frame) = serde_json::from_str::<RequestFrame>(&text) {
                            if frame.session_id == session_id {
                                crate::handlers::handle_message(
                                    frame,
                                    session_context_clone.clone(),
                                    axum::extract::State(crate::handlers::AppState {
                                        nexus: nexus_inner.clone(),
                                        storage: storage.clone(),
                                        config: config_inner.clone(),
                                    }),
                                )
                                .await;
                            }
                        }
                    }
                    Ok(Message::Ping(_data)) => {
                        // Axum handles Ping→Pong automatically
                    }
                    Ok(Message::Close(_)) => {
                        tracing::debug!("WebSocket close received");
                        break;
                    }
                    Ok(_) => {
                        // Binary, Pong — ignore
                    }
                    Err(e) => {
                        tracing::error!("WebSocket receive error: {}", e);
                        break;
                    }
                }
            }
        }
    });

    // 8. Wait for connection closure
    tokio::select! {
        _ = (&mut lane_fwd_task) => {},
        _ = (&mut telemetry_task) => {},
        _ = (&mut debug_log_task) => {},
        _ = (&mut send_task) => {},
        _ = (&mut recv_task) => {},
    }

    // 9. Cleanup
    lane_fwd_task.abort();
    telemetry_task.abort();
    debug_log_task.abort();
    send_task.abort();
    recv_task.abort();
    state.sessions.remove(&session_id);
    tracing::info!("Session closed: {}", session_id.0);
}

async fn agent_image_handler(
    State(state): State<Arc<GatewayState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Validate name to prevent path traversal - only allow alphanumeric + hyphens + underscores
    if name.is_empty()
        || name.len() > 128
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Response::builder()
            .status(400)
            .body(axum::body::Body::from("Invalid agent name"))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(500)
                    .body(axum::body::Body::empty())
                    .expect("valid response builder")
            });
    }

    let name_lower = name.to_lowercase();

    // 1. Check Cache
    {
        let mut cache = state.avatar_cache.lock().await;
        if let Some((content, content_type)) = cache.get(&name_lower) {
            return Response::builder()
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(axum::body::Body::from(content.clone()))
                .unwrap_or_else(|_| {
                    tracing::error!("Failed to build cached image response for {}", name_lower);
                    Response::builder()
                        .status(500)
                        .body(axum::body::Body::empty())
                        .expect("valid response builder")
                });
        }
    }

    let workspaces_dir = state.config.resolve_path(&state.config.system.agents_path);
    let workspace_path = workspaces_dir.join(format!("workspace-{}", name_lower));
    let candidates = ["avatar.png", "avatar.jpg", "avatar.jpeg", "agentimg.png"];

    for filename in candidates {
        let file_path = workspace_path.join(filename);
        if file_path.exists() {
            if let Ok(content) = std::fs::read(&file_path) {
                let content_type = if filename.ends_with(".png") {
                    "image/png"
                } else {
                    "image/jpeg"
                }
                .to_string();

                // Update Cache
                {
                    let mut cache = state.avatar_cache.lock().await;
                    cache.put(name_lower.clone(), (content.clone(), content_type.clone()));
                }

                return Response::builder()
                    .header(header::CONTENT_TYPE, content_type)
                    .header(header::CACHE_CONTROL, "public, max-age=3600")
                    .body(axum::body::Body::from(content))
                    .unwrap_or_else(|_| {
                        tracing::error!("Failed to build image response for {}", name_lower);
                        Response::builder()
                            .status(500)
                            .body(axum::body::Body::empty())
                            .expect("valid response builder")
                    });
            }
        }
    }

    // Fallback: Generate dynamic SVG avatar
    let initial = name.chars().next().unwrap_or('?').to_uppercase();
    let svg = format!(
        r#"<svg width="100" height="100" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
            <rect width="100" height="100" fill="{bg}"/>
            <text x="50" y="65" font-family="Arial" font-size="50" font-weight="bold" fill="{accent}" text-anchor="middle">{initial}</text>
            <rect x="5" y="5" width="90" height="90" fill="none" stroke="{accent}" stroke-width="2" opacity="0.3"/>
        </svg>"#,
        bg = "#00141a",
        accent = "#00d5ff",
        initial = initial
    );

    Response::builder()
        .header(header::CONTENT_TYPE, "image/svg+xml")
        .body(axum::body::Body::from(svg))
        .unwrap_or_else(|_| {
            tracing::error!("Failed to build SVG response for {}", name);
            Response::builder()
                .status(500)
                .body(axum::body::Body::empty())
                .expect("valid response builder")
        })
}

/// GET /api/settings - Returns current system settings
async fn settings_get_handler(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    let config = &state.config;
    let agents_dir = std::path::PathBuf::from(&config.system.agents_path);

    // Find first agent's model config
    let mut chat_model = String::new();
    let embedding_model =
        std::env::var("OLLAMA_EMBED_MODEL").unwrap_or_else(|_| "qwen3-embedding:4b".to_string());
    let vision_model =
        std::env::var("OLLAMA_VISION_MODEL").unwrap_or_else(|_| "qwen3-vl".to_string());

    if let Ok(entries) = std::fs::read_dir(&agents_dir) {
        for entry in entries.flatten() {
            let agent_json = entry.path().join("agent.json");
            if agent_json.exists() {
                if let Ok(content) = std::fs::read_to_string(&agent_json) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        chat_model = json["model"].as_str().unwrap_or("").to_string();
                    }
                }
                break;
            }
        }
    }

    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());

    let settings = serde_json::json!({
        "chat_model": chat_model,
        "embedding_model": embedding_model,
        "vision_model": vision_model,
        "ollama_url": ollama_url,
        "gateway_port": config.server.port,
        "agents_path": config.system.agents_path,
        "db_path": config.system.db_path,
        "temperature": config.ai.temperature,
        "top_p": config.ai.top_p,
        "frequency_penalty": config.ai.frequency_penalty,
        "presence_penalty": config.ai.presence_penalty,
    });

    Json(settings)
}

/// POST /api/settings - Updates system settings
#[derive(serde::Deserialize)]
struct SettingsUpdate {
    #[serde(default)]
    chat_model: Option<String>,
    #[serde(default)]
    vision_model: Option<String>,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    top_p: Option<f32>,
    #[serde(default)]
    frequency_penalty: Option<f32>,
    #[serde(default)]
    presence_penalty: Option<f32>,
    #[serde(default)]
    #[allow(dead_code)]
    ollama_url: Option<String>,
}

async fn settings_post_handler(
    State(state): State<Arc<GatewayState>>,
    Json(update): Json<SettingsUpdate>,
) -> impl IntoResponse {
    // Use in-memory config clone instead of re-reading from disk (prevents race conditions)
    let mut config = state.config.clone();

    // 1. Update Chat Model (Agent-specific)
    if let Some(model) = update.chat_model {
        let agents_dir = std::path::PathBuf::from(&state.config.system.agents_path);
        if let Ok(entries) = std::fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let agent_json = entry.path().join("agent.json");
                if agent_json.exists() {
                    if let Ok(content) = std::fs::read_to_string(&agent_json) {
                        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                            json["model"] = serde_json::Value::String(model.clone());
                            if let Ok(updated) = serde_json::to_string_pretty(&json) {
                                let _ = std::fs::write(&agent_json, updated);
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    // 2. AAA Validation & Range Clamping (Guardian Layer)
    let mut changed = false;
    let mut validation_notes = Vec::new();

    if let Some(v) = update.vision_model {
        config.ai.manifestation_model = Some(v);
        changed = true;
    }

    if let Some(v) = update.temperature {
        let clamped = v.clamp(0.0, 2.0);
        if (clamped - v).abs() > f32::EPSILON {
            validation_notes.push(format!("Temperature clamped from {} to {}", v, clamped));
        }
        config.ai.temperature = clamped;
        changed = true;
    }

    if let Some(v) = update.top_p {
        let clamped = v.clamp(0.0, 1.0);
        if (clamped - v).abs() > f32::EPSILON {
            validation_notes.push(format!("Top P clamped from {} to {}", v, clamped));
        }
        config.ai.top_p = clamped;
        changed = true;
    }

    if let Some(v) = update.frequency_penalty {
        let clamped = v.clamp(-2.0, 2.0);
        if (clamped - v).abs() > f32::EPSILON {
            validation_notes.push(format!(
                "Frequency Penalty clamped from {} to {}",
                v, clamped
            ));
        }
        config.ai.frequency_penalty = clamped;
        changed = true;
    }

    if let Some(v) = update.presence_penalty {
        let clamped = v.clamp(-2.0, 2.0);
        if (clamped - v).abs() > f32::EPSILON {
            validation_notes.push(format!(
                "Presence Penalty clamped from {} to {}",
                v, clamped
            ));
        }
        config.ai.presence_penalty = clamped;
        changed = true;
    }

    if changed {
        let config_path = savant_core::config::Config::primary_config_path();
        if let Err(e) = config.save(&config_path) {
            tracing::error!("❌ Failed to save config: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"status": "error", "message": e.to_string()})),
            )
                .into_response();
        }

        // Notify the Swarm via Nexus
        let _ = state
            .nexus
            .publish(
                "system.config.updated",
                &serde_json::json!({
                    "section": "ai",
                    "notes": validation_notes
                })
                .to_string(),
            )
            .await;
    }

    Json(serde_json::json!({
        "status": "ok",
        "notes": validation_notes
    }))
    .into_response()
}

/// Restores AI configuration to system defaults
async fn settings_reset_handler(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    // Use in-memory config clone instead of re-reading from disk
    let mut config = state.config.clone();

    // Apply defaults from savant_core::config::AiConfig::default()
    config.ai = savant_core::config::AiConfig::default();

    let config_path = savant_core::config::Config::primary_config_path();
    if let Err(e) = config.save(&config_path) {
        tracing::error!("❌ Failed to reset config: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"status": "error", "message": e.to_string()})),
        )
            .into_response();
    }

    // Notify the Swarm
    let _ = state
        .nexus
        .publish(
            "system.config.reset",
            &serde_json::json!({"section": "ai"}).to_string(),
        )
        .await;

    Json(serde_json::json!({"status": "ok", "message": "Settings restored to system defaults"}))
        .into_response()
}

/// Returns the list of available models and parameter descriptors
async fn models_get_handler() -> impl IntoResponse {
    let parameter_descriptors = savant_core::types::LlmParams::get_parameter_descriptors();

    // For now, we return the descriptors. We could also include the provider list
    // but the Tuning page primarily needs the descriptors.
    Json(serde_json::json!({
        "status": "ok",
        "parameter_descriptors": parameter_descriptors
    }))
    .into_response()
}

#[cfg(test)]
mod tests {
    // tests
}
