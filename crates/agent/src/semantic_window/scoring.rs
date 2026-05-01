//! Context Scoring — Evaluates message turns for semantic importance.

use savant_core::types::{ChatMessage, ChatRole};

/// Score for a single message turn.
#[derive(Debug, Clone)]
pub struct ContextScore {
    /// Index in the original message list.
    pub index: usize,
    /// The message.
    pub message: ChatMessage,
    /// Relevance score [0.0, 1.0].
    pub relevance: f32,
    /// Whether this message is pinned (never evicted).
    pub pinned: bool,
}

/// Scores message turns for semantic importance.
///
/// Scoring factors:
/// - Role weight: System > User > Assistant
/// - Recency: newer messages score higher
/// - Pin status: system prompts and SOUL.md are pinned
pub fn score_messages(messages: &[ChatMessage], current_query: &str) -> Vec<ContextScore> {
    let total = messages.len();
    let mut scores = Vec::with_capacity(total);

    for (i, msg) in messages.iter().enumerate() {
        let role_weight = match msg.role {
            ChatRole::System => 1.0,    // System messages are always important
            ChatRole::User => 0.7,      // User messages are usually important
            ChatRole::Assistant => 0.4, // Assistant responses are less critical
            _ => 0.3,
        };

        // Recency: normalize index to [0, 1] where 1 = most recent
        let recency = if total > 1 {
            i as f32 / (total - 1) as f32
        } else {
            1.0
        };

        // Keyword relevance: how much does this message overlap with current query
        let keyword_relevance = if current_query.is_empty() {
            0.5
        } else {
            simple_keyword_overlap(&msg.content, current_query)
        };

        // Pin status: system messages and messages containing SOUL.md references are pinned
        let pinned = msg.role == ChatRole::System
            || msg.content.contains("SOUL.md")
            || msg.content.contains("PERSONA (SOUL)")
            || msg.content.contains("SUBSTRATE OPERATIONAL DIRECTIVE");

        let relevance = if pinned {
            1.0 // Pinned content always has max relevance
        } else {
            (role_weight * 0.3 + recency * 0.4 + keyword_relevance * 0.3).clamp(0.0, 1.0)
        };

        scores.push(ContextScore {
            index: i,
            message: msg.clone(),
            relevance,
            pinned,
        });
    }

    scores
}

/// Simple keyword overlap between two strings.
fn simple_keyword_overlap(a: &str, b: &str) -> f32 {
    let words_a: std::collections::HashSet<&str> = a
        .split_whitespace()
        .filter(|w| w.len() > 3) // Skip short words
        .collect();
    let words_b: std::collections::HashSet<&str> =
        b.split_whitespace().filter(|w| w.len() > 3).collect();

    if words_a.is_empty() || words_b.is_empty() {
        return 0.0;
    }

    let intersection = words_a.intersection(&words_b).count();
    let min_size = words_a.len().min(words_b.len());

    (intersection as f32 / min_size as f32).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(role: ChatRole, content: &str) -> ChatMessage {
        ChatMessage {
            is_telemetry: false,
            role,
            content: content.to_string(),
            sender: None,
            recipient: None,
            agent_id: None,
            session_id: None,
            channel: savant_core::types::AgentOutputChannel::Chat,
        }
    }

    #[test]
    fn test_system_messages_pinned() {
        let messages = vec![
            make_msg(ChatRole::System, "You are an assistant"),
            make_msg(ChatRole::User, "hello"),
        ];
        let scores = score_messages(&messages, "");
        assert!(scores[0].pinned);
        assert!(!scores[1].pinned);
    }

    #[test]
    fn test_soul_reference_pinned() {
        let messages = vec![make_msg(ChatRole::User, "PERSONA (SOUL): You are loyal")];
        let scores = score_messages(&messages, "");
        assert!(scores[0].pinned);
    }

    #[test]
    fn test_relevance_ordering() {
        let messages = vec![
            make_msg(ChatRole::System, "system prompt"),
            make_msg(ChatRole::User, "first question"),
            make_msg(ChatRole::Assistant, "first answer"),
            make_msg(ChatRole::User, "second question"),
        ];
        let scores = score_messages(&messages, "second question");
        // The most recent user message with keyword match should score high
        assert!(scores[3].relevance > scores[2].relevance);
    }

    #[test]
    fn test_keyword_overlap() {
        let overlap =
            simple_keyword_overlap("the build failed with errors", "build errors detected");
        assert!(overlap > 0.0);
    }
}
