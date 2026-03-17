use crate::auth;
use crate::lanes::SessionLane;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    http::header,
    response::{IntoResponse, Response},
    routing::get,
    Router,
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
        avatar_cache: TokioMutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())),
        gateway_signing_key: ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng),
    });

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .route("/api/agents/:name/image", get(agent_image_handler))
        .route("/live", get(|| async { "OK" }))
        .route("/ready", get(|| async { "OK" }))
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

    // 1. Authentication Phase (Stubbed for consistency)
    let auth_frame = match receiver.next().await {
        Some(Ok(Message::Text(text))) => match serde_json::from_str::<RequestFrame>(&text) {
            Ok(frame) => frame,
            Err(e) => {
                tracing::error!("Failed to parse auth frame: {}", e);
                return;
            }
        },
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
                        let _ = crate::persistence::GatewayPersistence::persist_chat(
                            &storage_clone,
                            &msg,
                        )
                        .await;
                    }
                }
            } else if outbound_event.event_type == "learning.insight" {
                if let Ok(learning) = serde_json::from_str::<savant_core::learning::EmergentLearning>(
                    &outbound_event.payload,
                ) {
                    let msg = savant_core::types::ChatMessage {
                        role: savant_core::types::ChatRole::System,
                        content: format!("Insight: {}", learning.content),
                        sender: Some("ALD".to_string()),
                        recipient: None,
                        agent_id: None,
                        session_id: Some(savant_core::types::SessionId("learnings".to_string())),
                        channel: savant_core::types::AgentOutputChannel::Telemetry,
                    };
                    let _ =
                        crate::persistence::GatewayPersistence::persist_chat(&storage_clone, &msg)
                            .await;
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
            while let Some(Ok(Message::Text(text))) = receiver.next().await {
                if let Ok(frame) = serde_json::from_str::<RequestFrame>(&text) {
                    if frame.session_id == session_id {
                        // Route message through proper handler
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
        }
    });

    // 8. Wait for connection closure
    tokio::select! {
        _ = (&mut lane_fwd_task) => {},
        _ = (&mut telemetry_task) => {},
        _ = (&mut send_task) => {},
        _ = (&mut recv_task) => {},
    }

    // 9. Cleanup
    lane_fwd_task.abort();
    telemetry_task.abort();
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
                    .unwrap()
            });
    }

    let name_lower = name.to_lowercase();

    // 1. Check Cache
    {
        let mut cache = state.avatar_cache.lock().await;
        if let Some((content, content_type)) = cache.get(&name_lower) {
            return Response::builder()
                .header(header::CONTENT_TYPE, content_type)
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(axum::body::Body::from(content.clone()))
                .unwrap_or_else(|_| {
                    tracing::error!("Failed to build cached image response for {}", name_lower);
                    Response::builder()
                        .status(500)
                        .body(axum::body::Body::empty())
                        .unwrap()
                });
        }
    }

    let workspaces_dir = std::path::PathBuf::from(&state.config.system.agents_path);
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
                    .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .header(header::CACHE_CONTROL, "public, max-age=3600")
                    .body(axum::body::Body::from(content))
                    .unwrap_or_else(|_| {
                        tracing::error!("Failed to build image response for {}", name_lower);
                        Response::builder()
                            .status(500)
                            .body(axum::body::Body::empty())
                            .unwrap()
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
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(axum::body::Body::from(svg))
        .unwrap_or_else(|_| {
            tracing::error!("Failed to build SVG response for {}", name);
            Response::builder()
                .status(500)
                .body(axum::body::Body::empty())
                .unwrap()
        })
}

#[cfg(test)]
mod tests {
    // tests
}
