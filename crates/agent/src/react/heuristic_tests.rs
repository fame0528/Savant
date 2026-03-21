#[cfg(test)]
use super::*;
use async_trait::async_trait;
use futures::stream::StreamExt;
use futures::Stream;
use savant_core::error::SavantError;
use savant_core::traits::{LlmProvider, MemoryBackend};
use savant_core::types::{AgentIdentity, AgentOutputChannel, ChatMessage};
use std::pin::Pin;
use tokio_util::sync::CancellationToken;
// use std::sync::Arc;

struct AmbiguousLlm {
    responses: Vec<String>,
}

#[async_trait]
impl LlmProvider for AmbiguousLlm {
    async fn stream_completion(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<savant_core::types::ChatChunk, SavantError>> + Send>>,
        SavantError,
    > {
        let response = self.responses[0].clone();
        let chunk = savant_core::types::ChatChunk {
            agent_name: "test".to_string(),
            agent_id: "test".to_string(),
            content: response,
            is_final: true,
            session_id: None,
            channel: AgentOutputChannel::Chat,
            logprob: None,
            is_telemetry: false,
            reasoning: None,
        };
        Ok(Box::pin(futures::stream::iter(vec![Ok(chunk)])))
    }
}

struct MockMemory;
#[async_trait]
impl MemoryBackend for MockMemory {
    async fn store(&self, _agent_id: &str, _msg: &ChatMessage) -> Result<(), SavantError> {
        Ok(())
    }
    async fn retrieve(
        &self,
        _agent_id: &str,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<ChatMessage>, SavantError> {
        Ok(vec![])
    }
    async fn consolidate(&self, _agent_id: &str) -> Result<(), SavantError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_autonomous_ambiguity_synthesis() {
    let provider = Box::new(AmbiguousLlm {
        responses: vec![
            "Thought: I should use a tool.\nAction: MockTool missing_brackets".to_string(),
        ],
    });

    let mut agent = AgentLoop::new(
        "test_agent".into(),
        provider,
        MockMemory,
        vec![], // No tools, but we want to see if it parses
        AgentIdentity::default(),
    );

    let mut stream = agent.run("start".into(), None, CancellationToken::new());
    let mut ambiguity_detected = false;
    let mut synthesized_action = false;

    while let Some(res) = stream.next().await {
        match res {
            Ok(AgentEvent::StatusUpdate(s)) if s == "HEURISTIC_AMBIGUITY_DETECTED" => {
                ambiguity_detected = true;
            }
            Ok(AgentEvent::Action { name, .. }) if name == "MalformedMockTool" => {
                synthesized_action = true;
            }
            _ => {}
        }
    }
    drop(stream);

    assert!(
        ambiguity_detected,
        "Should have detected ambiguity in malformed Action: line"
    );
    assert!(
        synthesized_action,
        "Should have synthesized the MockTool action"
    );
}

#[tokio::test]
async fn test_checkpoint_creation() {
    let provider = Box::new(AmbiguousLlm {
        responses: vec!["Action: Tool1[]".to_string()],
    });

    let mut agent = AgentLoop::new(
        "test_agent".into(),
        provider,
        MockMemory,
        vec![],
        AgentIdentity::default(),
    );

    assert!(agent.heuristic.last_stable_checkpoint.is_none());

    let mut stream = agent.run("test".into(), None, CancellationToken::new());
    // Run until action execution
    while let Some(res) = stream.next().await {
        if let Ok(AgentEvent::Action { .. }) = res {
            break;
        }
    }
    drop(stream);

    assert!(
        agent.heuristic.last_stable_checkpoint.is_some(),
        "Checkpoint should be created before actions"
    );
    assert_eq!(
        agent
            .heuristic
            .last_stable_checkpoint
            .as_ref()
            .unwrap()
            .len(),
        1,
        "Checkpoint should contain at least the user message"
    );
}
