//! Zero-Copy Inter-Process Communication using iceoryx2 Blackboard pattern.
//!
//! This crate provides O(1) context sharing for massive agent swarms,
//! eliminating JSON serialization overhead and enabling sub-microsecond
//! state propagation across thousands of concurrent agents.

pub mod a2a;
pub mod blackboard;
pub mod collective;
mod error;

pub use a2a::{
    agent_card::{AgentCard, input_modes, output_modes},
    context::ContextPackage,
    protocol::{A2AMessageType, A2AEnvelope, Artifact, ArtifactPart, ArtifactPartType, DelegationTask, TaskState},
    queues::{AgentTaskQueue, TaskQueueError, DEFAULT_QUEUE_CAPACITY, MAX_QUEUE_RETRIES, QUEUE_FULL_BACKOFF_MS},
    result_router::{DelegationResult, RejectionReason, ResultRouter, ResultRouterError, TaskStatusUpdate},
};
pub use blackboard::{hash_session_id, CapabilityRegistry, SwarmBlackboard, SwarmSharedContext};
pub use collective::{CollectiveBlackboard, GlobalState, AgentEntry, ConsensusResult};
pub use error::SwarmIpcError;
