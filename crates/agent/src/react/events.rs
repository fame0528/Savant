/// Enum representing distinct agent loop events.
pub enum AgentEvent {
    Thought(String),
    Action { name: String, args: String },
    Observation(String),
    FinalAnswer(String),
    FinalAnswerChunk(String),
    Reflection(String),
    StatusUpdate(String), // Internal status heartbeats
}
