use serde::{Deserialize, Serialize};

/// Session ID type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

/// Device ID type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DeviceId(pub String);

/// Request Frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestFrame {
    pub session_id: SessionId,
    pub payload: String,
    pub signature: Option<String>,
    pub timestamp: Option<i64>,
}

/// Response Frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFrame {
    pub request_id: String,
    pub payload: String,
}

/// Event Frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFrame {
    pub event_type: String,
    pub payload: String,
}

/// Chat Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEvent {
    pub message: MessageContent,
}

/// Message Content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub text: String,
}

/// Chat roles for LLM interaction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// A standardized chat message for LLM context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub sender: Option<String>,
    pub recipient: Option<String>, // None = Broadcast
    pub agent_id: Option<String>,  // Stable ID for tracking
}

/// A streaming chunk of a chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    pub agent_name: String,
    pub agent_id: String,
    pub content: String,
    pub is_final: bool,
}

/// Model Provider Enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelProvider {
    OpenAi,
    Anthropic,
    Ollama,
    OpenRouter,
    LmStudio,
    Groq,
    Perplexity,
    Local,
}

/// Agent Identity containing personality and metadata (OpenClaw compatible)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentIdentity {
    pub name: String,
    pub soul: String,              // SOUL.md: Persona & Tone
    pub instructions: Option<String>, // AGENTS.md: Rules & Operating instructions
    pub user_context: Option<String>, // USER.md: Who the user is
    pub metadata: Option<String>,     // IDENTITY.md: Name, vibe, emoji
    pub mission: Option<String>,
    pub expertise: Vec<String>,
    pub ethics: Option<String>,
    pub image: Option<String>, // Base64 or URL to agentimg.png
}

/// Agent Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_id: String,
    pub agent_name: String,
    pub model_provider: ModelProvider,
    pub api_key: Option<String>,
    pub env_vars: std::collections::HashMap<String, String>,
    pub system_prompt: String,
    pub model: Option<String>,
    pub heartbeat_interval: u64,
    pub allowed_skills: Vec<String>,
    pub workspace_path: std::path::PathBuf,
    pub identity: Option<AgentIdentity>,
    pub parent_id: Option<String>,
}


/// Memory Category for specialized retrieval
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryCategory {
    Fact,
    Procedure,
    Correction,
    Preference,
    Observation,
    Reflection,
}

/// Memory Entry with Elite metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: i64,
    pub timestamp: i64,
    pub content: String,
    pub category: MemoryCategory,
    pub importance: u8,          // 1-10 ranking for consolidation
    pub associations: Vec<String>, // Tags or linked concept IDs
    pub embedding: Option<Vec<f32>>,
}

/// Agent Reflection for self-improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReflection {
    pub task_id: String,
    pub success: bool,
    pub critique: String,
    pub learning: String,
    pub action_items: Vec<String>,
    pub importance: u8,
}

/// Heartbeat Task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatTask {
    pub id: String,
    pub schedule: String,
    pub command: String,
    pub last_run: Option<i64>,
    pub next_run: Option<i64>,
}

