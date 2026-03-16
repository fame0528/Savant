use savant_core::types::{RequestFrame, SessionId};
use savant_core::error::SavantError;
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod oauth;

/// An authenticated session context.
#[derive(Clone)]
pub struct AuthenticatedSession {
    pub session_id: SessionId,
    pub public_key: [u8; 32],
}

/// Authenticates a new session request using Ed25519 signatures.
/// The session_id is expected to be the hex-encoded Ed25519 public key.
pub async fn authenticate(frame: &RequestFrame) -> Result<AuthenticatedSession, SavantError> {
    // Simple bypass for dashboard connections
    match &frame.payload {
        savant_core::types::RequestPayload::Auth(auth_str) if auth_str == "DASHBOARD_LOGIN" => {
            return Ok(AuthenticatedSession {
                session_id: SessionId("dashboard-session".to_string()),
                public_key: [0u8; 32],
            });
        }
        savant_core::types::RequestPayload::ControlFrame(savant_core::types::ControlFrame::InitialSync) => {
            return Ok(AuthenticatedSession {
                session_id: SessionId("dashboard-session".to_string()),
                public_key: [0u8; 32],
            });
        }
        _ => {}
    }
    
    let signature_hex = frame.signature.as_ref()
        .ok_or_else(|| SavantError::AuthError("Missing signature".to_string()))?;
    
    let timestamp = frame.timestamp.ok_or_else(|| SavantError::AuthError("Missing timestamp".to_string()))?;
    
    // Simple replay protection
    let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
    if (now - timestamp).abs() > 300 {
        return Err(SavantError::AuthError("Timestamp expired".to_string()));
    }

    let public_key_bytes = hex::decode(&frame.session_id.0)
        .map_err(|e| SavantError::AuthError(format!("Invalid session ID hex: {}", e)))?;
    
    if public_key_bytes.len() != 32 {
        return Err(SavantError::AuthError("Invalid public key length".to_string()));
    }

    let mut pk_array = [0u8; 32];
    pk_array.copy_from_slice(&public_key_bytes);
    
    let verifying_key = VerifyingKey::from_bytes(&pk_array)
        .map_err(|e| SavantError::AuthError(format!("Invalid public key: {}", e)))?;

    let signature_bytes = hex::decode(signature_hex)
        .map_err(|e| SavantError::AuthError(format!("Invalid signature hex: {}", e)))?;
    
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|e| SavantError::AuthError(format!("Invalid signature format: {}", e)))?;

    let payload_str = serde_json::to_string(&frame.payload)
        .map_err(|e| SavantError::AuthError(format!("Failed to serialize payload for verification: {}", e)))?;
    let message = format!("{}:{}", timestamp, payload_str);
    
    verifying_key.verify(message.as_bytes(), &signature)
        .map_err(|e| SavantError::AuthError(format!("Signature verification failed: {}", e)))?;

    Ok(AuthenticatedSession {
        session_id: frame.session_id.clone(),
        public_key: pk_array,
    })
}

#[cfg(test)]
mod benches {
    // criterion benchmark stub
}
