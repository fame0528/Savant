use ed25519_dalek::{Signer, Verifier, Signature, SigningKey, VerifyingKey};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use crate::token::{AgentToken, CapabilityPayload};

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Invalid Ed25519 signature")]
    InvalidSignature,
    #[error("Token has expired")]
    TokenExpired,
    #[error("Unauthorized: {0} for resource {1}")]
    UnauthorizedAction(String, String),
    #[error("Zero-copy memory validation failed.")]
    MemoryCorruption,
}

pub struct SecurityEnclave {
    /// The master public key of the Gateway/Orchestrator that issues tokens
    pub root_authority: VerifyingKey,
}

impl SecurityEnclave {
    pub fn new(root_authority: VerifyingKey) -> Self {
        Self { root_authority }
    }

    /// Helper to get current UNIX time securely
    fn current_time() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Mints a new token for a subagent. (Executed by the Orchestrator)
    pub fn mint_token(
        signer: &SigningKey,
        assignee_hash: u64,
        resource_uri: &str,
        permitted_action: &str,
        ttl_seconds: u64,
    ) -> Result<AgentToken, SecurityError> {
        let payload = CapabilityPayload {
            assignee_hash,
            resource_uri: resource_uri.to_string(),
            permitted_action: permitted_action.to_string(),
            expires_at: Self::current_time() + ttl_seconds,
        };

        // Serialize the payload to bytes for signing
        let payload_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&payload)
            .map_err(|_| SecurityError::MemoryCorruption)?;

        // Cryptographically sign the bytes using Ed25519
        let signature = signer.sign(&payload_bytes);

        Ok(AgentToken {
            payload,
            signature_bytes: signature.to_bytes(),
        })
    }

    /// Mathematically verifies a token presented by a subagent.
    /// Executed by the Wassette sandbox BEFORE running any tool.
    pub fn verify_token_and_action(
        &self,
        token: &AgentToken,
        agent_id: u64,
        requested_resource: &str,
        requested_action: &str,
    ) -> Result<(), SecurityError> {
        // 1. Time-to-Live Check
        if Self::current_time() > token.payload.expires_at {
            return Err(SecurityError::TokenExpired);
        }

        // 2. Identity Check (Prevent token theft)
        if token.payload.assignee_hash != agent_id {
            return Err(SecurityError::UnauthorizedAction(
                "Identity Mismatch".into(),
                "Token belongs to another agent".into(),
            ));
        }

        // 3. Action / Resource Scope Check (Fixes OpenClaw Issue #11102)
        if token.payload.permitted_action != requested_action 
            || !requested_resource.starts_with(&token.payload.resource_uri) 
        {
            return Err(SecurityError::UnauthorizedAction(
                requested_action.to_string(),
                token.payload.resource_uri.clone(),
            ));
        }

        // 4. Cryptographic Integrity Check
        let payload_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&token.payload)
            .map_err(|_| SecurityError::MemoryCorruption)?;
            
        let signature = Signature::from_bytes(&token.signature_bytes);

        self.root_authority
            .verify(&payload_bytes, &signature)
            .map_err(|_| SecurityError::InvalidSignature)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::thread_rng;

    #[test]
    fn test_token_minting_and_verification() {
        let mut rng = thread_rng();
        let signing_key = SigningKey::generate(&mut rng);
        let enclave = SecurityEnclave::new(signing_key.verifying_key());

        let token = SecurityEnclave::mint_token(
            &signing_key,
            12345,
            "/workspace/data/",
            "read",
            3600,
        ).expect("Failed to mint token");

        assert!(enclave.verify_token_and_action(&token, 12345, "/workspace/data/file.txt", "read").is_ok());
    }

    #[test]
    fn test_token_expiration() {
        let mut rng = thread_rng();
        let signing_key = SigningKey::generate(&mut rng);
        let enclave = SecurityEnclave::new(signing_key.verifying_key());

        let token = SecurityEnclave::mint_token(
            &signing_key,
            12345,
            "/",
            "read",
            0, // Expire immediately
        ).unwrap();

        // Small delay to ensure expiration
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        assert!(matches!(
            enclave.verify_token_and_action(&token, 12345, "/file", "read"),
            Err(SecurityError::TokenExpired)
        ));
    }

    #[test]
    fn test_identity_mismatch() {
        let mut rng = thread_rng();
        let signing_key = SigningKey::generate(&mut rng);
        let enclave = SecurityEnclave::new(signing_key.verifying_key());

        let token = SecurityEnclave::mint_token(&signing_key, 111, "/", "read", 100).unwrap();
        
        assert!(enclave.verify_token_and_action(&token, 222, "/file", "read").is_err());
    }

    #[test]
    fn test_signature_forgery() {
        let mut rng = thread_rng();
        let signing_key = SigningKey::generate(&mut rng);
        let enclave = SecurityEnclave::new(signing_key.verifying_key());

        let mut token = SecurityEnclave::mint_token(&signing_key, 123, "/", "read", 100).unwrap();
        
        // Tamper with payload action
        token.payload.permitted_action = "write".to_string();
        
        assert!(matches!(
            enclave.verify_token_and_action(&token, 123, "/file", "write"),
            Err(SecurityError::InvalidSignature)
        ));
    }
}
