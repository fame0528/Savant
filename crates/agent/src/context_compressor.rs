use savant_core::types::ChatMessage;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct ContextCompressor {
    enabled: bool,
    trigger_threshold: f64,
    preserve_head_turns: usize,
    preserve_tail_turns: usize,
    max_summary_tokens: usize,
    cooldown: Duration,
    last_compression: Mutex<Option<Instant>>,
}

impl ContextCompressor {
    pub fn new(
        enabled: bool,
        trigger_threshold: f64,
        preserve_head_turns: usize,
        preserve_tail_turns: usize,
        max_summary_tokens: usize,
        cooldown_seconds: u64,
    ) -> Self {
        ContextCompressor {
            enabled,
            trigger_threshold,
            preserve_head_turns,
            preserve_tail_turns,
            max_summary_tokens,
            cooldown: Duration::from_secs(cooldown_seconds),
            last_compression: Mutex::new(None),
        }
    }

    pub async fn should_compress(
        &self,
        messages: &[ChatMessage],
        current_token_count: usize,
        max_tokens: usize,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        let threshold = (max_tokens as f64 * self.trigger_threshold) as usize;
        if current_token_count < threshold {
            return false;
        }
        if messages.len() <= self.preserve_head_turns + self.preserve_tail_turns {
            return false;
        }
        let mut last = self.last_compression.lock().await;
        if let Some(prev) = *last {
            if prev.elapsed() < self.cooldown {
                return false;
            }
        }
        *last = Some(Instant::now());
        true
    }

    pub fn partition<'a>(
        &self,
        messages: &'a [ChatMessage],
    ) -> (Vec<&'a ChatMessage>, Vec<&'a ChatMessage>, Vec<&'a ChatMessage>) {
        let head: Vec<&ChatMessage> = messages
            .iter()
            .take(self.preserve_head_turns)
            .collect();
        let tail: Vec<&ChatMessage> = messages
            .iter()
            .rev()
            .take(self.preserve_tail_turns)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let middle: Vec<&ChatMessage> = messages
            .iter()
            .skip(self.preserve_head_turns)
            .take(messages.len().saturating_sub(self.preserve_head_turns + self.preserve_tail_turns))
            .collect();
        (head, middle, tail)
    }

    pub fn build_compression_prompt(middle_messages: &[&ChatMessage]) -> String {
        let conversation: String = middle_messages
            .iter()
            .map(|m| {
                format!(
                    "[{}] {}",
                    match m.role {
                        savant_core::types::ChatRole::User => "USER",
                        savant_core::types::ChatRole::Assistant => "ASSISTANT",
                        _ => "SYSTEM",
                    },
                    m.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "Summarize this conversation segment concisely. Output ONLY the summary, no preamble.\n\
            Format:\n\
            [Resolved:]\n- question answered: resolution\n\
            [Pending:]\n- open question\n\
            [Key Decisions:]\n- decision made\n\
            [Context:]\n- important context for future turns\n\n\
            Conversation:\n{conversation}"
        )
    }

    pub fn estimate_tokens(text: &str) -> usize {
        text.len() / 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        let text = "This is a test of token estimation.";
        let tokens = ContextCompressor::estimate_tokens(text);
        assert!(tokens > 0);
        assert!(tokens < text.len());
    }

    #[test]
    fn test_partition() {
        let compressor = ContextCompressor::new(true, 0.8, 2, 3, 2000, 600);
        let messages: Vec<ChatMessage> = (0..10)
            .map(|i| ChatMessage {
                is_telemetry: false,
                role: savant_core::types::ChatRole::User,
                content: format!("message {i}"),
                sender: None,
                recipient: None,
                agent_id: None,
                session_id: None,
                channel: savant_core::types::AgentOutputChannel::Chat,
            })
            .collect();
        let (head, middle, tail) = compressor.partition(&messages);
        assert_eq!(head.len(), 2);
        assert_eq!(tail.len(), 3);
        assert_eq!(middle.len(), 5);
    }
}
