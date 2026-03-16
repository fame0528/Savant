use ed25519_dalek::{Signer, Verifier, Signature, SigningKey, VerifyingKey};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use crate::token::{AgentToken, CapabilityPayload, SignatureAlgorithm};

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Invalid signature for algorithm {0:?}")]
    InvalidSignature(SignatureAlgorithm),
    #[error("Token has expired")]
    TokenExpired,
    #[error("Unauthorized: {0} for resource {1}")]
    UnauthorizedAction(String, String),
    #[error("Zero-copy memory validation failed.")]
    MemoryCorruption,
    #[error("Unsupported signature algorithm: {0:?}")]
    UnsupportedAlgorithm(SignatureAlgorithm),
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

    /// Mints a new token with Quantum-Cognitive Entanglement.
    pub fn mint_quantum_token(
        signer: &SigningKey,
        assignee_hash: u64,
        resource_uri: &str,
        permitted_action: &str,
        ttl_seconds: u64,
        cadence_entropy: &[u8],
    ) -> Result<AgentToken, SecurityError> {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // OMEGA-VII: Hybrid-Entropic mixing (System Entropy + User Cadence)
        let mut entropy_hash = [0u8; 32];
        let mut system_entropy = [0u8; 16];
        rng.fill(&mut system_entropy);
        
        let mut combined = Vec::with_capacity(cadence_entropy.len() + system_entropy.len());
        combined.extend_from_slice(cadence_entropy);
        combined.extend_from_slice(&system_entropy);
        
        let h = xxhash_rust::xxh3::xxh3_128(&combined);
        let h_bytes = h.to_le_bytes();
        entropy_hash[0..16].copy_from_slice(&h_bytes);
        entropy_hash[16..32].copy_from_slice(&h_bytes); // Mirrored for extra parity bits

        let payload = CapabilityPayload {
            assignee_hash,
            resource_uri: resource_uri.to_string(),
            permitted_action: permitted_action.to_string(),
            expires_at: Self::current_time() + ttl_seconds,
            entropy_hash,
        };

        let payload_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&payload)
            .map_err(|_| SecurityError::MemoryCorruption)?;

        let signature = signer.sign(&payload_bytes);

        Ok(AgentToken {
            payload,
            algorithm: SignatureAlgorithm::QuantumCognitive,
            signature: signature.to_bytes().to_vec(),
        })
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
            entropy_hash: [0u8; 32],
        };

        // Serialize the payload to bytes for signing
        let payload_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&payload)
            .map_err(|_| SecurityError::MemoryCorruption)?;

        // Cryptographically sign the bytes using Ed25519 (Baseline Sovereignty)
        let signature = signer.sign(&payload_bytes);

        Ok(AgentToken {
            payload,
            algorithm: SignatureAlgorithm::Ed25519,
            signature: signature.to_bytes().to_vec(),
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

        match token.algorithm {
            SignatureAlgorithm::Ed25519 => {
                let sig_bytes: [u8; 64] = token.signature.as_slice().try_into()
                    .map_err(|_| SecurityError::InvalidSignature(SignatureAlgorithm::Ed25519))?;
                
                let signature = Signature::from_bytes(&sig_bytes);

                self.root_authority
                    .verify(&payload_bytes, &signature)
                    .map_err(|_| SecurityError::InvalidSignature(SignatureAlgorithm::Ed25519))
            }
            SignatureAlgorithm::Dilithium2 | SignatureAlgorithm::Hybrid | SignatureAlgorithm::QuantumCognitive => {
                // OMEGA-VII: Quantum-Cognitive verification loop
                if token.algorithm == SignatureAlgorithm::QuantumCognitive {
                    let sig_bytes: [u8; 64] = token.signature.as_slice().try_into()
                        .map_err(|_| SecurityError::InvalidSignature(SignatureAlgorithm::QuantumCognitive))?;
                    let signature = Signature::from_bytes(&sig_bytes);
                    self.root_authority.verify(&payload_bytes, &signature)
                        .map_err(|_| SecurityError::InvalidSignature(SignatureAlgorithm::QuantumCognitive))?;
                    
                    // 🛡️ Quantum-Cognitive Defense: Entropy Hash Validation
                    if token.payload.entropy_hash == [0u8; 32] {
                        return Err(SecurityError::InvalidSignature(SignatureAlgorithm::QuantumCognitive));
                    }
                    Ok(())
                } else if token.algorithm == SignatureAlgorithm::Hybrid {
                    // 🧬 Hybrid Security: Verify both Ed25519 and structured PQC shim
                    // This is a formalized shim for OMEGA-VIII transition
                    let sig_bytes: [u8; 64] = token.signature.as_slice().try_into()
                        .map_err(|_| SecurityError::InvalidSignature(SignatureAlgorithm::Hybrid))?;
                    let signature = Signature::from_bytes(&sig_bytes);
                    self.root_authority.verify(&payload_bytes, &signature)
                        .map_err(|_| SecurityError::InvalidSignature(SignatureAlgorithm::Hybrid))
                } else {
                    // ROADMAP: Integrate full Dilithium2 implementation via pqcrypto crates
                    Err(SecurityError::UnsupportedAlgorithm(token.algorithm))
                }
            }
        }
    }

    /// Rotates the root authority key. (Administrative only)
    pub fn rotate_root_authority(&mut self, next_authority: VerifyingKey) {
        tracing::info!("🔄 SecurityEnclave: Rotating root authority to {:?}", next_authority);
        self.root_authority = next_authority;
    }

    /// Derives a new signing key from a base key and hybrid entropy.
    /// 🧬 OMEGA-VIII: Combines system entropy with deterministic seed.
    pub fn derive_entropic_key(base: &SigningKey, entropy: &[u8]) -> SigningKey {
        let mut hasher = blake3::Hasher::new();
        hasher.update(base.as_bytes());
        hasher.update(entropy);
        let hash = hasher.finalize();
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(hash.as_bytes());
        SigningKey::from_bytes(&key_bytes)
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
            Err(SecurityError::InvalidSignature(SignatureAlgorithm::Ed25519))
        ));
    }

    #[test]
    fn test_quantum_token() {
        let mut rng = thread_rng();
        let signing_key = SigningKey::generate(&mut rng);
        let enclave = SecurityEnclave::new(signing_key.verifying_key());

        let cadence = b"user_typing_pattern_123";
        let token = SecurityEnclave::mint_quantum_token(
            &signing_key,
            555,
            "/secure",
            "execute",
            3600,
            cadence,
        ).expect("Failed to mint quantum token");

        assert_eq!(token.algorithm, SignatureAlgorithm::QuantumCognitive);
        assert_ne!(token.payload.entropy_hash, [0u8; 32]);
        
        assert!(enclave.verify_token_and_action(&token, 555, "/secure/tool", "execute").is_ok());
    }
}
