use savant_core::error::SavantError;
use savant_core::types::{SessionId, RequestFrame};
use savant_core::config::GatewayConfig;
use axum::{
    Router, 
    routing::get,
    extract::{WebSocketUpgrade, State, Path, ws::{WebSocket, Message}},
    response::{IntoResponse, Response},
    http::header,
};
use std::net::SocketAddr;
use std::sync::Arc;
use savant_core::db::Storage;
use dashmap::DashMap;
use futures::{StreamExt, SinkExt};
use crate::auth;
use crate::lanes::SessionLane;

use savant_core::bus::NexusBridge;

/// Shared state for the gateway server.
pub struct GatewayState {
    pub config: GatewayConfig,
    pub sessions: DashMap<SessionId, Arc<SessionLane>>,
    pub nexus: Arc<NexusBridge>,
    pub storage: Arc<Storage>,
}

/// Starts the axum gateway server.
pub async fn start_gateway(config: GatewayConfig, nexus: Arc<NexusBridge>) -> Result<(), SavantError> {
    let addr = format!("{}:{}", config.host, config.port)
        .parse::<SocketAddr>()
        .map_err(|e| SavantError::Unknown(format!("Invalid address: {}", e)))?;

    let state = Arc::new(GatewayState {
        config: config.clone(),
        sessions: DashMap::new(),
        nexus,
        storage: Arc::new(Storage::new(std::path::PathBuf::from("./savant.db"))),
    });

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .route("/api/agents/:name/image", get(agent_image_handler))
        .route("/live", get(|| async { "OK" }))
        .route("/ready", get(|| async { "OK" }))
        .with_state(state);

    tracing::info!("Gateway server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
        .map_err(|e| SavantError::IoError(e))?;

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
        Some(Ok(Message::Text(text))) => {
            match serde_json::from_str::<RequestFrame>(&text) {
                Ok(frame) => frame,
                Err(e) => {
                    tracing::error!("Failed to parse auth frame: {}", e);
                    return;
                }
            }
        }
        _ => return,
    };

    let session_context = match auth::authenticate(&auth_frame).await {
        Ok(ctx) => ctx,
        Err(e) => {
            tracing::error!("Authentication failed: {}", e);
            let _ = sender.send(Message::Text(format!("Auth failed: {}", e))).await;
            return;
        }
    };

    let session_id = session_context.session_id.clone();
    tracing::info!("Session authenticated: {}", session_id.0);

    // 2. Initial State Synchronization: Send current agents if discovered
    let initial_agents = {
        let memory = state.nexus.shared_memory.lock().await;
        memory.get("system.agents").cloned()
    };
    
    if let Some(agents_json) = initial_agents {
        // Send as an EVENT:agents.discovered to match the real-time structure
        let event = savant_core::types::EventFrame {
            event_type: "agents.discovered".to_string(),
            payload: agents_json,
        };
        let msg = serde_json::to_string(&event).expect("Event serializable");
        let _ = sender.send(Message::Text(format!("EVENT:{}", msg))).await;
    }

    // 3. Outgoing Message Hub
    // We use a central MPSC to funnel both Lane responses and Swarm telemetry
    let (outgoing_tx, mut outgoing_rx) = tokio::sync::mpsc::channel::<Message>(100);

    // 3. Session Setup
    let (lane, lane_rx, mut res_rx, limit) = SessionLane::new(
        state.config.lane_capacity,
        state.config.max_lane_concurrency
    );
    let lane = Arc::new(lane);
    
    state.sessions.insert(session_id.clone(), lane.clone());
    SessionLane::spawn_consumer(lane_rx, lane.response_tx.clone(), limit, state.nexus.clone());

    // 4. Task 1: Forward Lane Responses to Outgoing Hub
    let out_tx = outgoing_tx.clone();
    let mut lane_fwd_task = tokio::spawn(async move {
        while let Some(response) = res_rx.recv().await {
            let msg = serde_json::to_string(&response).expect("Response serializable");
            let _ = out_tx.send(Message::Text(msg)).await;
        }
    });

    // 5. Consolidated Swarm Telemetry Task
    let out_tx = outgoing_tx.clone();
    let storage_clone = state.storage.clone();
    let mut event_rx = state.nexus.event_bus.subscribe();
    
    let mut telemetry_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            match event.event_type.as_str() {
                "chat.response" => {
                    if let Ok(msg) = serde_json::from_str::<savant_core::types::ChatMessage>(&event.payload) {
                        // Persist agent response
                        let _ = storage_clone.append_chat("global", &msg);
                        // Send raw JSON for consistency with dashboard expectation
                        let _ = out_tx.send(Message::Text(event.payload.clone())).await;
                    }
                }
                "chat.chunk" => {
                    let _ = out_tx.send(Message::Text(format!("CHUNK:{}", event.payload))).await;
                }
                _ => {
                    // Forward all other significant swarm events
                    let msg = serde_json::to_string(&event).expect("Event serializable");
                    let _ = out_tx.send(Message::Text(format!("EVENT:{}", msg))).await;
                }
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
    let out_tx = outgoing_tx.clone();
    let storage = state.storage.clone();
    let nexus_inner = state.nexus.clone();
    let mut recv_task = tokio::spawn({
        let session_id = session_id.clone();
        let session_context_clone = session_context.clone();
        async move {
            while let Some(Ok(Message::Text(text))) = receiver.next().await {
                // Handle special HISTORY_REQUEST
                if text == "HISTORY_REQUEST" {
                    if let Ok(history) = storage.get_history("global", 50) {
                    if let Ok(history_json) = serde_json::to_string(&history) {
                        let _ = out_tx.send(Message::Text(format!("HISTORY:{}", history_json))).await;
                    }
                    }
                    continue;
                }

                if let Ok(frame) = serde_json::from_str::<RequestFrame>(&text) {
                    if frame.session_id == session_id {
                        // Route message through proper handler
                        crate::handlers::handle_message(
                            frame, 
                            session_context_clone.clone(), 
                            axum::extract::State(crate::handlers::AppState { 
                                nexus: nexus_inner.clone(),
                                storage: storage.clone(),
                            })
                        ).await;
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
    Path(name): Path<String>,
) -> impl IntoResponse {
    let name_lower = name.to_lowercase();
    let workspaces_dir = std::path::PathBuf::from("./workspaces");
    
    // Look for workspace-{name}
    let workspace_path = workspaces_dir.join(format!("workspace-{}", name_lower));
    
    // Potential image filenames
    let candidates = ["avatar.png", "avatar.jpg", "avatar.jpeg", "agentimg.png"];
    
    for filename in candidates {
        let file_path = workspace_path.join(filename);
        if file_path.exists() {
            if let Ok(content) = std::fs::read(&file_path) {
                let content_type = if filename.ends_with(".png") {
                    "image/png"
                } else {
                    "image/jpeg"
                };
                
                return Response::builder()
                    .header(header::CONTENT_TYPE, content_type)
                    .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .header(header::CACHE_CONTROL, "no-cache, no-store")
                    .body(axum::body::Body::from(content))
                    .expect("Failed to build image response");
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
        .expect("Failed to build static SVG response")
}

#[cfg(test)]
mod tests {
    // tests
}
