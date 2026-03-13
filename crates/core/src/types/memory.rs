// use bytecheck::CheckBytes; removed.
use rkyv::{Archive, Deserialize, Serialize};

/// High-performance, zero-copy message structure for LSM-tree storage.
/// Optimized for back-traversal and instant context window assembly.
#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub struct AgentMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub sender: Option<String>,
    pub recipient: Option<String>,
    pub tool_use_id: Option<String>,
    pub timestamp: i64,
}

impl AgentMessage {
    pub fn from_chat(msg: &crate::types::ChatMessage, session_id: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: msg.role.to_string(),
            content: msg.content.clone(),
            sender: msg.sender.clone(),
            recipient: msg.recipient.clone(),
            tool_use_id: None, // Will be populated by compaction/logic if needed
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn to_chat(&self) -> crate::types::ChatMessage {
        use std::str::FromStr;
        crate::types::ChatMessage {
            role: crate::types::ChatRole::from_str(&self.role)
                .unwrap_or(crate::types::ChatRole::User),
            content: self.content.clone(),
            sender: self.sender.clone(),
            recipient: self.recipient.clone(),
            agent_id: Some(self.session_id.clone()), // session_id is typically the agent_id in current Savant
        }
    }
}
