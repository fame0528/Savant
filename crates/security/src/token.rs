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
}

/// The complete token, including the Ed25519 signature.
#[derive(Archive, Serialize, Deserialize, CheckBytes, Debug, Clone)]
#[bytecheck(crate = bytecheck)]
#[repr(C)]
pub struct AgentToken {
    pub payload: CapabilityPayload,
    /// 64-byte Ed25519 signature of the payload
    pub signature_bytes: [u8; 64],
}
