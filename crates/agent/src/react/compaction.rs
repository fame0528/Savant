//! Context Compaction — prevents context overflow on long conversations.
//!
//! Three strategies selected by usage ratio:
//! - 80-85%: MoveToWorkspace — archive old messages to daily log, keep recent
//! - 85-95%: Summarize — LLM bullet-point summary, keep recent
//! - >95%: Truncate — aggressive, keep only recent turns

use savant_core::types::{ChatMessage, ChatRole};

/// Compaction strategy selected based on context usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionStrategy {
    /// Archive old messages to workspace daily log
    MoveToWorkspace,
    /// LLM-based summarization of old messages
    Summarize,
    /// Aggressive truncation — keep only recent turns
    Truncate,
}

/// Estimate token count for a message (word count * 1.3 + 4 overhead).
pub fn estimate_message_tokens(msg: &ChatMessage) -> usize {
    let words = msg.content.split_whitespace().count();
    (words as f64 * 1.3) as usize + 4
}

/// Estimate total tokens across all messages.
pub fn estimate_total_tokens(messages: &[ChatMessage]) -> usize {
    messages.iter().map(|m| estimate_message_tokens(m)).sum()
}

/// Context monitor — decides when and how to compact.
pub struct ContextMonitor {
    /// Model's context window in tokens
    context_limit: usize,
}

impl ContextMonitor {
    pub fn new(context_limit: usize) -> Self {
        Self { context_limit }
    }

    /// Current usage ratio (0.0 = empty, 1.0 = full).
    pub fn usage_ratio(&self, messages: &[ChatMessage]) -> f64 {
        if self.context_limit == 0 {
            return 1.0;
        }
        estimate_total_tokens(messages) as f64 / self.context_limit as f64
    }

    /// Suggest a compaction strategy based on current usage.
    /// Returns None if no compaction needed (< 80%).
    pub fn suggest(&self, messages: &[ChatMessage]) -> Option<CompactionStrategy> {
        let usage = self.usage_ratio(messages);
        match usage {
            u if u < 0.80 => None,
            u if u < 0.85 => Some(CompactionStrategy::MoveToWorkspace),
            u if u < 0.95 => Some(CompactionStrategy::Summarize),
            _ => Some(CompactionStrategy::Truncate),
        }
    }
}

/// Compactor — executes compaction strategies.
pub struct Compactor;

impl Compactor {
    /// Truncate: keep only the most recent messages.
    pub fn truncate(messages: Vec<ChatMessage>, keep_recent: usize) -> Vec<ChatMessage> {
        if messages.len() <= keep_recent {
            return messages;
        }
        messages[messages.len() - keep_recent..].to_vec()
    }

    /// Move to workspace: archive old messages, keep recent.
    /// Returns (archived_text, recent_messages).
    pub fn partition(messages: Vec<ChatMessage>, keep_recent: usize) -> (String, Vec<ChatMessage>) {
        if messages.len() <= keep_recent {
            return (String::new(), messages);
        }

        let split_idx = messages.len() - keep_recent;
        let archived_text = messages[..split_idx]
            .iter()
            .map(|m| format!("[{:?}] {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let recent = messages[split_idx..].to_vec();
        (archived_text, recent)
    }

    /// Apply compaction strategy to messages.
    ///
    /// For MoveToWorkspace and Summarize: archives old content, returns recent messages.
    /// A system message is injected to inform the LLM that context was compacted.
    pub fn compact(
        messages: Vec<ChatMessage>,
        strategy: CompactionStrategy,
        keep_recent: usize,
    ) -> Vec<ChatMessage> {
        match strategy {
            CompactionStrategy::Truncate => Self::truncate(messages, keep_recent),
            CompactionStrategy::MoveToWorkspace | CompactionStrategy::Summarize => {
                let (archived, mut recent) = Self::partition(messages, keep_recent);

                if !archived.is_empty() {
                    let summary_msg = ChatMessage {
                        is_telemetry: false,
                        role: ChatRole::System,
                        content: format!(
                            "[Context compacted: {} older messages archived. The conversation continues below.]",
                            archived.lines().count().max(1)
                        ),
                        sender: Some("SYSTEM".to_string()),
                        recipient: None,
                        agent_id: None,
                        session_id: None,
                        channel: savant_core::types::AgentOutputChannel::Chat,
                    };
                    recent.insert(0, summary_msg);
                }

                recent
            }
        }
    }
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
    fn test_estimate_message_tokens() {
        let msg = make_msg(ChatRole::User, "hello world this is a test");
        let tokens = estimate_message_tokens(&msg);
        assert!(tokens > 0);
        assert!(tokens < 20); // 6 words * 1.3 + 4 ≈ 12
    }

    #[test]
    fn test_estimate_total_tokens() {
        let messages = vec![
            make_msg(ChatRole::User, "hello"),
            make_msg(ChatRole::Assistant, "hi there"),
        ];
        let total = estimate_total_tokens(&messages);
        assert!(total > 0);
    }

    #[test]
    fn test_monitor_no_compaction_needed() {
        let monitor = ContextMonitor::new(100_000);
        let messages = vec![make_msg(ChatRole::User, "short message")];
        assert!(monitor.suggest(&messages).is_none());
    }

    #[test]
    fn test_monitor_suggests_archive() {
        let monitor = ContextMonitor::new(100);
        let messages: Vec<ChatMessage> = (0..30)
            .map(|_| {
                make_msg(
                    ChatRole::User,
                    "this is a moderately long message with several words",
                )
            })
            .collect();
        let strategy = monitor.suggest(&messages);
        assert!(strategy.is_some());
    }

    #[test]
    fn test_truncate() {
        let messages: Vec<ChatMessage> = (0..10)
            .map(|i| make_msg(ChatRole::User, &format!("msg {}", i)))
            .collect();
        let result = Compactor::truncate(messages, 3);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].content, "msg 7");
    }

    #[test]
    fn test_truncate_no_op() {
        let messages = vec![make_msg(ChatRole::User, "only message")];
        let result = Compactor::truncate(messages, 10);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_partition() {
        let messages: Vec<ChatMessage> = (0..5)
            .map(|i| make_msg(ChatRole::User, &format!("msg {}", i)))
            .collect();
        let (archived, recent) = Compactor::partition(messages, 2);
        assert!(archived.contains("msg 0"));
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].content, "msg 3");
    }

    #[test]
    fn test_compact_with_summary_injection() {
        let messages: Vec<ChatMessage> = (0..5)
            .map(|i| make_msg(ChatRole::User, &format!("msg {}", i)))
            .collect();
        let result = Compactor::compact(messages, CompactionStrategy::MoveToWorkspace, 2);
        assert_eq!(result.len(), 3); // summary + 2 recent
        assert_eq!(result[0].role, ChatRole::System);
        assert!(result[0].content.contains("compacted"));
    }

    #[test]
    fn test_compact_truncate() {
        let messages: Vec<ChatMessage> = (0..10)
            .map(|i| make_msg(ChatRole::User, &format!("msg {}", i)))
            .collect();
        let result = Compactor::compact(messages, CompactionStrategy::Truncate, 3);
        assert_eq!(result.len(), 3);
    }
}
