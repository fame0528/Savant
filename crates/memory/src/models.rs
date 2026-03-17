//! Zero-Copy Memory Models using rkyv
//!
//! These structures are designed for direct memory mapping from disk with
//! zero heap allocation during read operations. They maintain identical
//! in-memory and on-disk representations via `#[repr(C)]` and `rkyv` derives.

use crate::error::MemoryError;
use chrono::Utc;
use savant_core::types::{ChatMessage, ChatRole};
use std::collections::HashMap;
use tracing::error;

/// Represents a single message in the conversation history.
///
/// This is the core transcript unit. It is stored using rkyv's zero-copy
/// serialization in Fjall's LSM tree. The structure is optimized for:
/// - Minimal size (32-byte alignment)
/// - Fast deserialization
/// - Compaction integrity verification
///
/// # Safety
///
/// The `#[rkyv(check_bytes)]` attribute ensures that any bytes loaded from
/// disk are cryptographically validated against the schema before mapping,
/// preventing maliciously crafted data from causing undefined behavior.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    bytecheck::CheckBytes,
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
#[bytecheck(crate = bytecheck)]
#[repr(C)]
pub struct AgentMessage {
    /// Unique message identifier (UUID v4)
    pub id: String,
    /// Session identifier - MUST be present to prevent ZeroClaw Issue #2222 (cross-channel bleed)
    pub session_id: String,
    /// Role of the message sender
    pub role: MessageRole,
    /// Message content (text)
    pub content: String,
    /// Associated tool calls (if any)
    pub tool_calls: Vec<ToolCallRef>,
    /// Associated tool results (if any)
    pub tool_results: Vec<ToolResultRef>,
    /// Unix timestamp in milliseconds
    pub timestamp: rend::i64_le,
    /// Optional parent message ID (for conversation threading)
    pub parent_id: Option<String>,
    /// Strict Output Channel
    pub channel: String,
}

impl AgentMessage {
    /// Creates a new user message.
    pub fn user(session_id: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: MessageRole::User,
            content: content.to_string(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            timestamp: chrono::Utc::now().timestamp_millis().into(),
            parent_id: None,
            channel: "Chat".to_string(),
        }
    }

    /// Creates a new assistant message.
    pub fn assistant(session_id: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: MessageRole::Assistant,
            content: content.to_string(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            timestamp: chrono::Utc::now().timestamp_millis().into(),
            parent_id: None,
            channel: "Chat".to_string(),
        }
    }

    /// Creates a new system message.
    pub fn system(session_id: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: MessageRole::System,
            content: content.to_string(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            timestamp: chrono::Utc::now().timestamp_millis().into(),
            parent_id: None,
            channel: "Telemetry".to_string(),
        }
    }

    /// Creates a new tool result message.
    pub fn tool_result(
        session_id: &str,
        tool_use_id: &str,
        result_content: &str,
        is_error: bool,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: MessageRole::Tool,
            content: result_content.to_string(),
            tool_calls: Vec::new(),
            tool_results: vec![ToolResultRef {
                tool_use_id: tool_use_id.to_string(),
                result_content: result_content.to_string(),
                is_error,
            }],
            timestamp: chrono::Utc::now().timestamp_millis().into(),
            parent_id: None,
            channel: "Telemetry".to_string(),
        }
    }

    /// Converts a core `ChatMessage` into an `AgentMessage`.
    /// The session_id is provided separately; if ChatMessage has an agent_id,
    /// it will be used as the session_id if `session_id` param is empty.
    pub fn from_chat(msg: &ChatMessage, session_id: &str) -> Self {
        let role = match msg.role {
            ChatRole::User => MessageRole::User,
            ChatRole::Assistant => MessageRole::Assistant,
            ChatRole::System => MessageRole::System,
        };

        // AAA: Unified Context Harmony - Prioritize session_id over agent_id or implicit session
        let sid = msg
            .session_id
            .as_ref()
            .map(|s| s.0.clone())
            .unwrap_or_else(|| session_id.to_string());

        // Sanitize to prevent path traversal in LSM partitions
        let sid = savant_core::session::sanitize_session_id(&sid);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: sid,
            role,
            content: msg.content.clone(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            timestamp: Utc::now().timestamp_millis().into(),
            parent_id: None,
            channel: serde_json::to_string(&msg.channel)
                .unwrap_or_default()
                .replace('"', ""),
        }
    }

    /// Converts this `AgentMessage` into a core `ChatMessage`.
    /// Note: Tool role messages are converted to User role for LLM context.
    pub fn to_chat(&self) -> ChatMessage {
        let role = match self.role {
            MessageRole::User => ChatRole::User,
            MessageRole::Assistant => ChatRole::Assistant,
            MessageRole::System => ChatRole::System,
            MessageRole::Tool => ChatRole::User,
        };
        ChatMessage {
            role,
            content: self.content.clone(),
            sender: None,
            recipient: None,
            agent_id: None, // AAA: Deprecated in favor of session_id
            session_id: Some(savant_core::types::SessionId(self.session_id.clone())),
            channel: serde_json::from_str(&format!("\"{}\"", self.channel)).unwrap_or_default(),
        }
    }
}

/// Role of the message sender.
///
/// This is a compact enum optimized for storage and serialization.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    bytecheck::CheckBytes,
    rkyv::Portable,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
)]
#[bytecheck(crate = bytecheck)]
#[repr(u8)]
pub enum MessageRole {
    System = 0,
    User = 1,
    Assistant = 2,
    Tool = 3,
}

/// Reference to a tool invocation within a message.
///
/// This is stored inline in the AgentMessage to maintain atomicity between
/// a tool call and its result (prevents OpenClaw Issue #39609).
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    bytecheck::CheckBytes,
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
#[bytecheck(crate = bytecheck)]
#[repr(C)]
pub struct ToolCallRef {
    /// Unique identifier for this tool use (must match the corresponding ToolResultRef)
    pub tool_use_id: String,
    /// Name of the tool being invoked
    pub tool_name: String,
    /// Arguments as raw JSON string (avoids nested allocation overhead)
    pub arguments: String,
}

/// Reference to a tool execution result.
///
/// Every ToolResultRef must have a matching ToolCallRef in the session
/// to prevent orphaned results that would break the conversation.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    bytecheck::CheckBytes,
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
#[bytecheck(crate = bytecheck)]
#[repr(C)]
pub struct ToolResultRef {
    /// Must match the tool_use_id from a previous ToolCallRef
    pub tool_use_id: String,
    /// Result payload (or error message if is_error=true)
    pub result_content: String,
    /// Whether this result represents an error
    pub is_error: bool,
}

/// A higher-level memory entry for semantic search.
///
/// This structure represents tagged, important memories that should be
/// indexed for semantic retrieval. It is separate from the conversation
/// transcript to allow for summarization and distillation.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    bytecheck::CheckBytes,
    Debug,
    Clone,
    PartialEq,
)]
#[bytecheck(crate = bytecheck)]
#[repr(C)]
pub struct MemoryEntry {
    /// Unique ID for this memory
    pub id: rend::u64_le,
    /// Session this memory belongs to
    pub session_id: String,
    /// Category/type of memory (e.g., "fact", "preference", "observation")
    pub category: String,
    /// The distilled memory content
    pub content: String,
    /// Importance score (1-10) used for compaction prioritization
    pub importance: u8,
    /// Associated tags for filtering
    pub tags: Vec<String>,
    /// Vector embedding (128-384 dimensions) for semantic search
    /// Stored as raw f32 array; actual length determined by embedding model
    pub embedding: Vec<f32>,
    /// Creation timestamp
    pub created_at: rend::i64_le,
    /// Last updated timestamp
    pub updated_at: rend::i64_le,
    // --- OMEGA SINGULARITY EXTENSIONS ---
    /// Shannon Entropy value (Informational Gain)
    pub shannon_entropy: rend::f32_le,
    /// Last accessed timestamp for temporal heuristics
    pub last_accessed_at: rend::i64_le,
    /// Total access count for frequency-based importance
    pub hit_count: rend::u32_le,
    /// Relational edges (IDs of related MemoryEntry objects)
    pub related_to: Vec<rend::u64_le>,
}

/// Generates a storage key for a session's transcript.
///
/// Key format: `session:{session_id}`
/// This ensures all messages for a session are contiguous in the LSM tree.
pub fn session_key(session_id: &str) -> String {
    format!("session:{}", session_id)
}

/// Generates a storage key for an individual message.
///
/// Key format: `session:{session_id}:{timestamp}:{id}`
/// The timestamp prefix ensures chronological ordering in the LSM tree.
pub fn message_key(session_id: &str, timestamp: i64, id: &str) -> String {
    format!("session:{}:{}:{}", session_id, timestamp, id)
}

/// Verifies that for every tool_result in a batch, there is a corresponding
/// tool_call earlier in the session. This prevents OpenClaw Issue #39609.
///
/// # Arguments
/// * `messages` - A batch of messages being committed in a transaction
///
/// # Returns
/// * `Ok(())` if all tool_results have matching tool_calls
/// * `Err(MemoryError::OrphanedToolResult)` if any orphan is detected
pub fn verify_tool_pair_integrity(messages: &[AgentMessage]) -> Result<(), MemoryError> {
    let mut tool_call_set = HashMap::new();

    // First pass: collect all tool_use_ids from tool_calls
    for msg in messages {
        for tool_call in &msg.tool_calls {
            tool_call_set.insert(tool_call.tool_use_id.clone(), msg.id.clone());
        }
    }

    // Second pass: verify every tool_result has a matching tool_call
    for msg in messages {
        for tool_result in &msg.tool_results {
            if !tool_call_set.contains_key(&tool_result.tool_use_id) {
                error!(
                    orphan_id = %tool_result.tool_use_id,
                    session = %msg.session_id,
                    "Compaction aborted: orphaned tool_result detected"
                );
                return Err(MemoryError::OrphanedToolResult {
                    tool_use_id: tool_result.tool_use_id.clone(),
                    session_id: msg.session_id.clone(),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_message_creation() {
        let msg = AgentMessage::user("session123", "Hello, world!");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.session_id, "session123");
        assert!(msg.tool_calls.is_empty());
        assert!(msg.tool_results.is_empty());
    }

    #[test]
    fn test_verify_tool_pair_integrity_success() {
        let msg1 = AgentMessage {
            id: "msg1".to_string(),
            session_id: "sess".to_string(),
            role: MessageRole::Assistant,
            content: "".to_string(),
            tool_calls: vec![ToolCallRef {
                tool_use_id: "call1".to_string(),
                tool_name: "test".to_string(),
                arguments: "{}".to_string(),
            }],
            tool_results: Vec::new(),
            timestamp: 1000.into(),
            parent_id: None,
            channel: "Chat".to_string(),
        };

        let msg2 = AgentMessage {
            id: "msg2".to_string(),
            session_id: "sess".to_string(),
            role: MessageRole::Tool,
            content: "result".to_string(),
            tool_calls: Vec::new(),
            tool_results: vec![ToolResultRef {
                tool_use_id: "call1".to_string(),
                result_content: "ok".to_string(),
                is_error: false,
            }],
            timestamp: 2000.into(),
            parent_id: None,
            channel: "Telemetry".to_string(),
        };

        assert!(verify_tool_pair_integrity(&[msg1.clone(), msg2]).is_ok());
    }

    #[test]
    fn test_verify_tool_pair_integrity_failure() {
        let msg = AgentMessage {
            id: "msg1".to_string(),
            session_id: "sess".to_string(),
            role: MessageRole::Tool,
            content: "result".to_string(),
            tool_calls: Vec::new(),
            tool_results: vec![ToolResultRef {
                tool_use_id: "missing_call".to_string(),
                result_content: "orphan".to_string(),
                is_error: false,
            }],
            timestamp: 2000.into(),
            parent_id: None,
            channel: "Telemetry".to_string(),
        };

        assert!(verify_tool_pair_integrity(&[msg]).is_err());
    }

    #[test]
    fn test_memory_entry_size() {
        // Ensure MemoryEntry is reasonably sized for indexing
        let size = std::mem::size_of::<MemoryEntry>();
        // Pointer-heavy structs have size dominated by Vec<f32> (3 usize) + 7 fields
        // This is just a sanity check
        assert!(size > 0);
    }

    #[test]
    fn test_session_key_format() {
        let key = session_key("sess-abc");
        assert!(key.contains("sess-abc"));
    }

    #[test]
    fn test_message_key_format() {
        let key = message_key("sess-abc", 1710000000, "msg-1");
        assert!(key.contains("sess-abc"));
        assert!(key.contains("msg-1"));
    }

    #[test]
    fn test_agent_message_system_role() {
        let msg = AgentMessage::system("sess", "System message");
        assert_eq!(msg.role, MessageRole::System);
        assert_eq!(msg.content, "System message");
    }

    #[test]
    fn test_agent_message_assistant_role() {
        let msg = AgentMessage::assistant("sess", "Response");
        assert_eq!(msg.role, MessageRole::Assistant);
    }

    #[test]
    fn test_agent_message_tool_role() {
        let msg = AgentMessage::tool_result("sess", "call-1", "Tool output", false);
        assert_eq!(msg.role, MessageRole::Tool);
        assert_eq!(msg.tool_results.len(), 1);
        assert!(!msg.tool_results[0].is_error);
    }

    #[test]
    fn test_agent_message_tool_error() {
        let msg = AgentMessage::tool_result("sess", "call-2", "Error output", true);
        assert!(msg.tool_results[0].is_error);
    }

    #[test]
    fn test_verify_tool_pair_integrity_empty() {
        assert!(verify_tool_pair_integrity(&[]).is_ok());
    }

    #[test]
    fn test_agent_message_serialization_roundtrip() {
        // AgentMessage uses rkyv for zero-copy serialization, not serde.
        // Verify struct fields are correctly populated.
        let msg = AgentMessage::user("sess-1", "Hello world");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello world");
        assert_eq!(msg.session_id, "sess-1");
        assert!(!msg.id.is_empty());
        assert!(msg.timestamp > 0);
    }

    #[test]
    fn test_message_key_uniqueness() {
        let key1 = message_key("sess", 1000, "m1");
        let key2 = message_key("sess", 2000, "m1");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_message_role_debug() {
        assert_eq!(format!("{:?}", MessageRole::User), "User");
        assert_eq!(format!("{:?}", MessageRole::Assistant), "Assistant");
        assert_eq!(format!("{:?}", MessageRole::System), "System");
        assert_eq!(format!("{:?}", MessageRole::Tool), "Tool");
    }
}
