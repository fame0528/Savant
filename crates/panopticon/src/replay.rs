//! Conversation Replay — structured event logging for agent reasoning steps.
//!
//! Records agent decisions, tool calls, observations, and thoughts as
//! structured events that can be replayed in a timeline visualization.
//!
//! # Event Types
//! - `Thought` — Agent's internal reasoning
//! - `ToolCall` — Agent invokes a tool
//! - `Observation` — Tool returns a result
//! - `Decision` — Agent makes a decision based on observation
//! - `Error` — Something went wrong

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// A single step in the agent's reasoning chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayEvent {
    /// Unique event ID.
    pub id: String,
    /// Agent that produced this event.
    pub agent_id: String,
    /// Timestamp (millis since epoch).
    pub timestamp: i64,
    /// Type of event.
    pub event_type: ReplayEventType,
    /// The event content.
    pub content: String,
    /// Optional metadata (tool name, error code, etc.)
    pub metadata: Option<serde_json::Value>,
}

/// Types of events in the replay timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplayEventType {
    /// Agent's internal reasoning.
    Thought,
    /// Agent invokes a tool.
    ToolCall,
    /// Tool returns an observation.
    Observation,
    /// Agent makes a decision.
    Decision,
    /// Error occurred.
    Error,
    /// Agent receives user input.
    UserInput,
    /// Agent produces output.
    AgentOutput,
}

impl std::fmt::Display for ReplayEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayEventType::Thought => write!(f, "THOUGHT"),
            ReplayEventType::ToolCall => write!(f, "TOOL_CALL"),
            ReplayEventType::Observation => write!(f, "OBSERVATION"),
            ReplayEventType::Decision => write!(f, "DECISION"),
            ReplayEventType::Error => write!(f, "ERROR"),
            ReplayEventType::UserInput => write!(f, "USER_INPUT"),
            ReplayEventType::AgentOutput => write!(f, "AGENT_OUTPUT"),
        }
    }
}

/// Replay event recorder. Thread-safe, append-only log.
pub struct ReplayRecorder {
    events: Arc<Mutex<Vec<ReplayEvent>>>,
    max_events: usize,
}

impl ReplayRecorder {
    /// Creates a new recorder with the given max event capacity.
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::with_capacity(max_events))),
            max_events,
        }
    }

    /// Records a new event.
    pub async fn record(&self, event: ReplayEvent) {
        let mut events = self.events.lock().await;
        if events.len() >= self.max_events {
            events.remove(0); // FIFO eviction
        }
        events.push(event);
    }

    /// Returns all events for an agent, optionally filtered by type.
    pub async fn get_events(
        &self,
        agent_id: &str,
        event_type: Option<ReplayEventType>,
        limit: usize,
    ) -> Vec<ReplayEvent> {
        let events = self.events.lock().await;
        events
            .iter()
            .filter(|e| e.agent_id == agent_id)
            .filter(|e| event_type.as_ref().map_or(true, |t| &e.event_type == t))
            .rev()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Returns the total number of events.
    pub async fn count(&self) -> usize {
        self.events.lock().await.len()
    }

    /// Clears all events.
    pub async fn clear(&self) {
        self.events.lock().await.clear();
    }
}

/// Helper: create a timestamp for now.
pub fn now_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_display() {
        assert_eq!(ReplayEventType::Thought.to_string(), "THOUGHT");
        assert_eq!(ReplayEventType::ToolCall.to_string(), "TOOL_CALL");
        assert_eq!(ReplayEventType::Error.to_string(), "ERROR");
    }

    #[test]
    fn test_event_serialization() {
        let event = ReplayEvent {
            id: "test-1".to_string(),
            agent_id: "agent-alpha".to_string(),
            timestamp: 1234567890,
            event_type: ReplayEventType::Thought,
            content: "Thinking about the problem".to_string(),
            metadata: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("thought"));
        assert!(json.contains("agent-alpha"));

        let deserialized: ReplayEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, "Thinking about the problem");
    }

    #[tokio::test]
    async fn test_recorder_record_and_retrieve() {
        let recorder = ReplayRecorder::new(100);

        recorder
            .record(ReplayEvent {
                id: "1".to_string(),
                agent_id: "alpha".to_string(),
                timestamp: 1000,
                event_type: ReplayEventType::Thought,
                content: "First thought".to_string(),
                metadata: None,
            })
            .await;

        recorder
            .record(ReplayEvent {
                id: "2".to_string(),
                agent_id: "alpha".to_string(),
                timestamp: 2000,
                event_type: ReplayEventType::ToolCall,
                content: "Calling tool".to_string(),
                metadata: None,
            })
            .await;

        let events = recorder.get_events("alpha", None, 10).await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].content, "First thought");
        assert_eq!(events[1].content, "Calling tool");
    }

    #[tokio::test]
    async fn test_recorder_filter_by_type() {
        let recorder = ReplayRecorder::new(100);

        recorder
            .record(ReplayEvent {
                id: "1".to_string(),
                agent_id: "alpha".to_string(),
                timestamp: 1000,
                event_type: ReplayEventType::Thought,
                content: "thinking".to_string(),
                metadata: None,
            })
            .await;

        recorder
            .record(ReplayEvent {
                id: "2".to_string(),
                agent_id: "alpha".to_string(),
                timestamp: 2000,
                event_type: ReplayEventType::ToolCall,
                content: "calling tool".to_string(),
                metadata: None,
            })
            .await;

        let thoughts = recorder
            .get_events("alpha", Some(ReplayEventType::Thought), 10)
            .await;
        assert_eq!(thoughts.len(), 1);
        assert_eq!(thoughts[0].content, "thinking");
    }

    #[tokio::test]
    async fn test_recorder_eviction() {
        let recorder = ReplayRecorder::new(3);

        for i in 0..5 {
            recorder
                .record(ReplayEvent {
                    id: i.to_string(),
                    agent_id: "alpha".to_string(),
                    timestamp: i as i64,
                    event_type: ReplayEventType::Thought,
                    content: format!("event {}", i),
                    metadata: None,
                })
                .await;
        }

        let events = recorder.get_events("alpha", None, 10).await;
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].content, "event 2");
        assert_eq!(events[2].content, "event 4");
    }

    #[tokio::test]
    async fn test_recorder_isolation() {
        let recorder = ReplayRecorder::new(100);

        recorder
            .record(ReplayEvent {
                id: "1".to_string(),
                agent_id: "alpha".to_string(),
                timestamp: 1000,
                event_type: ReplayEventType::Thought,
                content: "alpha thought".to_string(),
                metadata: None,
            })
            .await;

        recorder
            .record(ReplayEvent {
                id: "2".to_string(),
                agent_id: "beta".to_string(),
                timestamp: 2000,
                event_type: ReplayEventType::Thought,
                content: "beta thought".to_string(),
                metadata: None,
            })
            .await;

        let alpha_events = recorder.get_events("alpha", None, 10).await;
        assert_eq!(alpha_events.len(), 1);
        assert_eq!(alpha_events[0].agent_id, "alpha");

        let beta_events = recorder.get_events("beta", None, 10).await;
        assert_eq!(beta_events.len(), 1);
        assert_eq!(beta_events[0].agent_id, "beta");
    }

    #[tokio::test]
    async fn test_recorder_count() {
        let recorder = ReplayRecorder::new(100);
        assert_eq!(recorder.count().await, 0);

        recorder
            .record(ReplayEvent {
                id: "1".to_string(),
                agent_id: "a".to_string(),
                timestamp: 0,
                event_type: ReplayEventType::Thought,
                content: "test".to_string(),
                metadata: None,
            })
            .await;

        assert_eq!(recorder.count().await, 1);
    }
}
