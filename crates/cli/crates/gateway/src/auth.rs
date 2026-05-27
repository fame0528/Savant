//! Ed25519 authentication for Savant Gateway
//!
//! Matches the existing pairing handshake in `handlers/pairing.rs`:
//! 1. Client generates Ed25519 keypair
//! 2. Client signs timestamped challenge
//! 3. Client sends signed frame to gateway
//! 4. Gateway validates signature and returns session token

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use savant_core::types::{RequestFrame, RequestPayload, SessionId};
use serde::{Deserialize, Serialize};

/// Ed25519 authentication credentials
#[derive(Clone)]
pub struct Ed25519Auth {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    public_key_base64: String,
}

impl Ed25519Auth {
    /// Generate a new Ed25519 keypair
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let public_key_base64 = STANDARD.encode(verifying_key.to_bytes());

        Self {
            signing_key,
            verifying_key,
            public_key_base64,
        }
    }

    /// Load from existing keypair bytes
    pub fn from_bytes(secret_key_bytes: [u8; 32]) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(&secret_key_bytes);
        let verifying_key = signing_key.verifying_key();
        let public_key_base64 = STANDARD.encode(verifying_key.to_bytes());

        Ok(Self {
            signing_key,
            verifying_key,
            public_key_base64,
        })
    }

    /// Get the public key as base64
    pub fn public_key(&self) -> &str {
        &self.public_key_base64
    }

    /// Create an auth frame for the initial handshake
    pub fn create_auth_frame(&self) -> Result<RequestFrame> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let challenge = format!("auth:{}:{}", self.public_key_base64, timestamp);

        // Sign the challenge
        let signature = self.signing_key.sign(challenge.as_bytes());
        let signature_base64 = STANDARD.encode(signature.to_bytes());

        // Create auth payload
        let auth_payload = AuthPayload {
            public_key: self.public_key_base64.clone(),
            timestamp: timestamp.to_string(),
            signature: signature_base64,
        };

        let payload_json = serde_json::to_string(&auth_payload)?;

        Ok(RequestFrame {
            request_id: uuid::Uuid::new_v4().to_string(),
            session_id: SessionId("cli-auth".to_string()),
            payload: RequestPayload::Auth(payload_json),
            signature: None,
            timestamp: Some(timestamp),
        })
    }

    /// Sign arbitrary data
    pub fn sign(&self, data: &[u8]) -> [u8; 64] {
        self.signing_key.sign(data).to_bytes()
    }

    /// Get the verifying key
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }
}

/// Authentication payload sent to gateway
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthPayload {
    pub public_key: String,
    pub timestamp: String,
    pub signature: String,
}
