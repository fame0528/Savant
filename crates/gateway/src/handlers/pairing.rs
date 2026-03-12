use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::server::GatewayState;

/// Pairing Request from a Companion Node
#[derive(Debug, Deserialize)]
pub struct PairingRequest {
    pub device_name: String,
    pub device_type: String, // "macos", "ios", "android"
    pub public_key: String,
}

/// Pairing Response from the Gateway
#[derive(Debug, Serialize)]
pub struct PairingResponse {
    pub session_token: String,
    pub gateway_public_key: String,
}

/// Handles node pairing requests through a secure handshake.
pub async fn pairing_handler(
    State(_state): State<Arc<GatewayState>>,
    Json(payload): Json<PairingRequest>,
) -> Json<PairingResponse> {
    tracing::info!("Node pairing initiated from {} ({})", payload.device_name, payload.device_type);
    
    // Placeholder for secure vault storage and Ed25519 key exchange
    Json(PairingResponse {
        session_token: "ST-HANDSHAKE-SUCCESS".to_string(),
        gateway_public_key: "GPK-SAVANT-MASTER".to_string(),
    })
}
