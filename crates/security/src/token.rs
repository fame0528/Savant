use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

/// The payload of the token, optimized for zero-copy deserialization.
#[derive(Archive, Serialize, Deserialize, CheckBytes, Debug, Clone)]
#[bytecheck(crate = bytecheck)]
#[repr(C)]
pub struct CapabilityPayload {
    /// The unique hash of the agent this token was issued to.
    pub assignee_hash: u64,
    /// The specific resource allowed (e.g., "/workspace/data/")
    pub resource_uri: String,
    /// The specific action allowed (e.g., "read", "append", "execute")
    pub permitted_action: String,
    /// UNIX timestamp of expiration
    pub expires_at: u64,
    /// OMEGA-VII: Binding to Quantum-Cognitive Stream
    pub entropy_hash: [u8; 32],
}

/// Supported cryptographic algorithms for token signatures.
#[derive(Archive, Serialize, Deserialize, CheckBytes, rkyv::Portable, Debug, Clone, Copy, PartialEq, Eq)]
#[bytecheck(crate = bytecheck)]
#[repr(u8)]
pub enum SignatureAlgorithm {
    /// Standard Ed25519 (64 bytes)
    Ed25519 = 0,
    /// PQC-ready Dilithium2 (Future-proof)
    Dilithium2 = 1,
    /// Hybrid (Ed25519 + Dilithium2) for absolute sovereignty
    Hybrid = 2,
    /// OMEGA-VII: Quantum-Cognitive Entangled signature
    QuantumCognitive = 3,
}

/// The complete token, including cryptographic signatures.
///
/// Omega: Transitioned from fixed-size Ed25519 to a multi-algorithm hybrid structure.
#[derive(Archive, Serialize, Deserialize, CheckBytes, Debug, Clone)]
#[bytecheck(crate = bytecheck)]
#[repr(C)]
pub struct AgentToken {
    /// The capability payload being signed
    pub payload: CapabilityPayload,
    /// The algorithm used for signing
    pub algorithm: SignatureAlgorithm,
    /// The raw signature bytes (variable length to support PQC)
    pub signature: Vec<u8>,
}
