use crate::types::SessionId;

/// 🌀 SessionMapper: Platform-Agnostic Context Anchoring
/// 
/// Translates platform-specific identifiers into unified, platform-prefixed 
/// session anchors to ensure high-fidelity context persistence.
pub struct SessionMapper;

impl SessionMapper {
    /// Maps a platform and ID into a sanitized OMEGA anchor.
    /// Prefixes prevent collisions across Discord, Matrix, and WebUI.
    pub fn map(platform: &str, id: &str) -> SessionId {
        let sanitized = Self::sanitize(id);
        SessionId(format!("{}:{}", platform.to_lowercase(), sanitized))
    }

    /// 🛡️ Sanitizes a session ID to prevent path traversal or keyspace corruption.
    pub fn sanitize(id: &str) -> String {
        id.chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect()
    }

    /// Verifies if a session ID is well-formed within the UCH context.
    pub fn is_valid(session: &SessionId) -> bool {
        let s = &session.0;
        s.contains(':') && s.chars().all(|c| c.is_alphanumeric() || c == ':' || c == '-' || c == '_')
    }
}

pub fn sanitize_session_id(id: &str) -> String {
    SessionMapper::sanitize(id)
}
