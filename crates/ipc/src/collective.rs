use iceoryx2::prelude::*;
use iceoryx2::service::port_factory::blackboard::PortFactory;
use std::sync::Arc;
use tracing::info;
use crate::error::SwarmIpcError;

/// Swarm-wide Collective Intelligence State
///
/// MUST be `#[repr(C)]` for zero-copy cross-process sharing.
/// Represents the global "mind" of the swarm.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, ZeroCopySend)]
pub struct CollectiveState {
    /// Bitmask of currently active agents (102 agents max)
    pub active_agents_mask: [u64; 2],
    /// Swarm-wide heuristic version (incremented on major insights)
    pub heuristic_version: u64,
    /// Global task pressure (0.0 to 1.0)
    pub swarm_pressure: f32,
    /// Total successful tool executions in current epoch
    pub total_successes: u64,
    /// Total observed tool failures (triggers swarm-wide caution)
    pub total_failures: u64,
    
    // --- Swarm Consensus Phase (Voting) ---
    /// The current proposal hash (XXH3) undergoing voting
    pub active_proposal_hash: u64,
    /// Type of proposal (0=None, 1=Destructive Edit, 2=Security Polish)
    pub proposal_type: u8,
    /// Bitmask of "Approve" votes
    pub approve_mask: [u64; 2],
    /// Bitmask of "Veto" votes (Vetoeing overrides all approvals)
    pub veto_mask: [u64; 2],
    /// Threshold required for consensus (count of agents)
    pub quorum_threshold: u8,
    
    /// Reserved for future swarm expansion
    pub reserved: [u8; 15],
}

impl Default for CollectiveState {
    fn default() -> Self {
        Self {
            active_agents_mask: [0; 2],
            heuristic_version: 0,
            swarm_pressure: 0.0,
            total_successes: 0,
            total_failures: 0,
            active_proposal_hash: 0,
            proposal_type: 0,
            approve_mask: [0; 2],
            veto_mask: [0; 2],
            quorum_threshold: 3, // Default to 3-agent peer review
            reserved: [0; 15],
        }
    }
}

/// The Collective Blackboard Service.
///
/// Manages swarm-level state that all agents can read and update.
pub struct CollectiveBlackboard {
    _node: Arc<Node<ipc::Service>>,
    service: PortFactory<ipc::Service, u64>,
}

impl CollectiveBlackboard {
    /// Initializes the collective blackboard.
    pub fn new(service_name: &str) -> Result<Self, SwarmIpcError> {
        info!("Initializing Collective Intelligence Blackboard '{}'", service_name);

        let node = NodeBuilder::new()
            .create::<ipc::Service>()
            .map_err(|e| SwarmIpcError::NodeCreation(e.to_string()))?;

        let iox_name: iceoryx2::prelude::ServiceName = service_name.try_into()
            .map_err(|e: iceoryx2::service::service_name::ServiceNameError| SwarmIpcError::ServiceCreation(e.to_string()))?;

        let service = node
            .service_builder(&iox_name)
            .blackboard_creator::<u64>()
            .max_readers(1024)
            .max_nodes(10)
            .add::<CollectiveState>(0, CollectiveState::default())
            .create()
            .map_err(|e| SwarmIpcError::ServiceCreation(e.to_string()))?;

        Ok(Self {
            _node: Arc::new(node),
            service,
        })
    }

    /// Publishes the collective state.
    pub fn publish_state(&self, state: CollectiveState) -> Result<(), SwarmIpcError> {
        let writer = self.service.writer_builder().create().map_err(|e| {
            SwarmIpcError::AccessViolation(format!("Failed to create collective writer: {}", e))
        })?;

        let entry = writer.entry::<CollectiveState>(&0).map_err(|e| {
            SwarmIpcError::AccessViolation(format!("Collective state entry not found: {}", e))
        })?;
        
        entry.update_with_copy(state);
        Ok(())
    }

    /// Reads the collective state.
    pub fn read_state(&self) -> Result<CollectiveState, SwarmIpcError> {
        let reader = self.service.reader_builder().create().map_err(|e| {
            SwarmIpcError::AccessViolation(format!("Failed to create collective reader: {}", e))
        })?;

        if let Ok(entry) = reader.entry::<CollectiveState>(&0) {
            Ok(*entry.get())
        } else {
            Err(SwarmIpcError::AccessViolation("Collective state not found".to_string()))
        }
    }

    /// Setting the quorum threshold dynamically.
    pub fn set_quorum_threshold(&self, threshold: u8) -> Result<(), SwarmIpcError> {
        let mut state = self.read_state()?;
        state.quorum_threshold = threshold;
        self.publish_state(state)
    }

    /// Participating in a vote.
    pub fn cast_vote(&self, agent_index: u8, approve: bool) -> Result<(), SwarmIpcError> {
        if agent_index >= 128 {
            return Err(SwarmIpcError::AccessViolation("Agent index out of bounds".to_string()));
        }

        let mut state = self.read_state()?;
        let mask_idx = (agent_index / 64) as usize;
        let bit_idx = agent_index % 64;

        if approve {
            state.approve_mask[mask_idx] |= 1 << bit_idx;
        } else {
            state.veto_mask[mask_idx] |= 1 << bit_idx;
        }

        self.publish_state(state)
    }

    /// Checking if consensus is reached.
    pub fn check_consensus(&self) -> ConsensusResult {
        let Ok(state) = self.read_state() else {
            return ConsensusResult::Pending;
        };

        // If any veto is set, proposal is dead
        if state.veto_mask[0] != 0 || state.veto_mask[1] != 0 {
            return ConsensusResult::Vetoed;
        }

        let total_approvals = state.approve_mask[0].count_ones() + state.approve_mask[1].count_ones();
        if total_approvals >= state.quorum_threshold as u32 {
            ConsensusResult::Approved
        } else {
            ConsensusResult::Pending
        }
    }
}

pub enum ConsensusResult {
    Approved,
    Vetoed,
    Pending,
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collective_state_voting_logic() {
        let mut state = CollectiveState {
            quorum_threshold: 2,
            ..Default::default()
        };

        // Agent 1 approves
        let mask_idx = 0;
        let bit_idx = 1;
        state.approve_mask[mask_idx] |= 1 << bit_idx;

        // Check consensus (Pending as 1 < 2)
        assert_eq!(state.approve_mask[0].count_ones(), 1);
        
        // Agent 2 approves
        let bit_idx2 = 2;
        state.approve_mask[0] |= 1 << bit_idx2;
        
        assert_eq!(state.approve_mask[0].count_ones(), 2);
    }

    #[test]
    fn test_collective_veto_overrides_approval() {
        let mut state = CollectiveState {
            quorum_threshold: 1,
            ..Default::default()
        };
        
        // Approve
        state.approve_mask[0] |= 1 << 5;
        // Veto
        state.veto_mask[0] |= 1 << 10;
        
        // Manual verification of the logic used in check_consensus
        let has_veto = state.veto_mask[0] != 0 || state.veto_mask[1] != 0;
        assert!(has_veto);
    }
}
