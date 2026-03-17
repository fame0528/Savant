//! Zero-Copy Inter-Process Communication using iceoryx2 Blackboard pattern.
//!
//! This crate provides O(1) context sharing for massive agent swarms,
//! eliminating JSON serialization overhead and enabling sub-microsecond
//! state propagation across thousands of concurrent agents.

pub mod blackboard;
pub mod collective;
mod error;

pub use blackboard::{hash_session_id, SwarmBlackboard, SwarmSharedContext};
pub use collective::{CollectiveBlackboard, GlobalState, AgentEntry, ConsensusResult};
pub use error::SwarmIpcError;
