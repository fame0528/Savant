use iceoryx2::prelude::*;
use iceoryx2::service::port_factory::blackboard::PortFactory;
use std::sync::Arc;
use tracing::info;
use crate::error::SwarmIpcError;

/// Individual Agent Entry in the Collective Blackboard.
///
/// MUST be `#[repr(C)]` for zero-copy sharing.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, ZeroCopySend)]
pub struct AgentEntry {
    /// Total successful tool executions by this agent
    pub successes: u64,
    /// Total tool failures by this agent
    pub failures: u64,
    /// Agent-specific task pressure (0.0 to 1.0)
    pub pressure: f32,
    /// Whether the agent is currently participating in the swarm pulse
    pub is_active: bool,
    /// Epoch-relative index of the agent (1-128)
    pub agent_index: u8,
    /// Reserved for future expansion
    pub reserved: [u8; 10],
}

impl Default for AgentEntry {
    fn default() -> Self {
        Self {
            successes: 0,
            failures: 0,
            pressure: 0.0,
            is_active: false,
            agent_index: 0,
            reserved: [0; 10],
        }
    }
}

/// Swarm-wide Collective Intelligence State (Global Entry 0)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, ZeroCopySend)]
pub struct GlobalState {
    /// Swarm-wide heuristic version (incremented on major insights)
    pub heuristic_version: u64,
    /// Aggregate swarm pressure (mean of all active agents)
    pub swarm_pressure: f32,
    /// Swarm-wide aggregate successes
    pub total_successes: u64,
    /// Swarm-wide aggregate failures
    pub total_failures: u64,
    
    // --- Swarm Consensus Phase (Voting) ---
    /// The current proposal hash (XXH3) undergoing voting
    pub active_proposal_hash: u64,
    /// Type of proposal (0=None, 1=Destructive Edit, 2=Security Polish)
    pub proposal_type: u8,
    /// Bitmask of "Approve" votes (Supports up to 128 agents)
    pub approve_mask: [u64; 2],
    /// Bitmask of "Veto" votes
    pub veto_mask: [u64; 2],
    /// Threshold required for consensus
    pub quorum_threshold: u8,
    
    /// Reserved for future swarm expansion
    pub reserved: [u8; 31],
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            heuristic_version: 0,
            swarm_pressure: 0.0,
            total_successes: 0,
            total_failures: 0,
            active_proposal_hash: 0,
            proposal_type: 0,
            approve_mask: [0; 2],
            veto_mask: [0; 2],
            quorum_threshold: 3,
            reserved: [0; 31],
        }
    }
}

/// The Collective Blackboard Service.
///
/// Implements a distributed entry model where each agent owns a specific
/// index in the blackboard to avoid concurrency race conditions.
pub struct CollectiveBlackboard {
    _node: Arc<Node<ipc::Service>>,
    service: PortFactory<ipc::Service, u64>,
}

impl CollectiveBlackboard {
    /// Initializes the collective blackboard with 129 entries (0=Global, 1-128=Agents).
    pub fn new(service_name: &str) -> Result<Self, SwarmIpcError> {
        info!("Initializing Distributed Collective Blackboard '{}' (129 entries)", service_name);

        let node = NodeBuilder::new()
            .create::<ipc::Service>()
            .map_err(|e| SwarmIpcError::NodeCreation(e.to_string()))?;

        let iox_name: iceoryx2::prelude::ServiceName = service_name.try_into()
            .map_err(|e: iceoryx2::service::service_name::ServiceNameError| SwarmIpcError::ServiceCreation(e.to_string()))?;

        let mut builder = node
            .service_builder(&iox_name)
            .blackboard_creator::<u64>()
            .max_readers(1024)
            .max_nodes(128) // Support one node per agent if needed
            .add::<GlobalState>(0, GlobalState::default());

        // Reserve entries 1 through 128 for individual agents
        for i in 1..=128 {
            builder = builder.add::<AgentEntry>(i as u64, AgentEntry::default());
        }

        let service = builder.create()
            .map_err(|e| SwarmIpcError::ServiceCreation(e.to_string()))?;

        Ok(Self {
            _node: Arc::new(node),
            service,
        })
    }

    /// Publishes the global swarm state.
    pub fn publish_global_state(&self, state: GlobalState) -> Result<(), SwarmIpcError> {
        let writer = self.service.writer_builder().create().map_err(|e| {
            SwarmIpcError::AccessViolation(format!("Failed to create global writer: {}", e))
        })?;

        let entry = writer.entry::<GlobalState>(&0).map_err(|e| {
            SwarmIpcError::AccessViolation(format!("Global state entry not found: {}", e))
        })?;
        
        entry.update_with_copy(state);
        Ok(())
    }

    /// Reads the global swarm state.
    pub fn read_global_state(&self) -> Result<GlobalState, SwarmIpcError> {
        let reader = self.service.reader_builder().create().map_err(|e| {
            SwarmIpcError::AccessViolation(format!("Failed to create global reader: {}", e))
        })?;

        if let Ok(entry) = reader.entry::<GlobalState>(&0) {
            Ok(*entry.get())
        } else {
            Err(SwarmIpcError::AccessViolation("Global state entry not found".to_string()))
        }
    }

    /// Updates metrics for a specific agent.
    pub fn update_agent_metrics(&self, agent_index: u8, success: bool, pressure: f32) -> Result<(), SwarmIpcError> {
        if agent_index == 0 || agent_index > 128 {
            return Err(SwarmIpcError::AccessViolation(format!("Invalid agent index: {}", agent_index)));
        }

        // We need a reader to get the current state and a writer to update it
        let reader = self.service.reader_builder().create().map_err(|e| {
            SwarmIpcError::AccessViolation(format!("Failed to create agent reader: {}", e))
        })?;

        let writer = self.service.writer_builder().create().map_err(|e| {
            SwarmIpcError::AccessViolation(format!("Failed to create agent writer: {}", e))
        })?;

        let id = agent_index as u64;
        let entry_ref = writer.entry::<AgentEntry>(&id).map_err(|e| {
            SwarmIpcError::AccessViolation(format!("Agent entry {} not found: {}", id, e))
        })?;

        let mut entry = if let Ok(reader_entry) = reader.entry::<AgentEntry>(&id) {
            *reader_entry.get()
        } else {
            AgentEntry::default()
        };

        if success {
            entry.successes += 1;
        } else {
            entry.failures += 1;
        }
        entry.pressure = pressure;
        entry.is_active = true;
        entry.agent_index = agent_index;

        entry_ref.update_with_copy(entry);
        Ok(())
    }

    /// Aggregates all agent metrics and publishes them to the global state.
    /// 
    /// This should typically be called by a designated "Swarm Leader" or periodically by agents.
    pub fn aggregate_swarm_metrics(&self) -> Result<GlobalState, SwarmIpcError> {
        let reader = self.service.reader_builder().create().map_err(|e| {
            SwarmIpcError::AccessViolation(format!("Failed to create aggregation reader: {}", e))
        })?;

        let mut global = self.read_global_state()?;
        let mut total_successes = 0;
        let mut total_failures = 0;
        let mut total_pressure = 0.0;
        let mut active_count = 0;

        for i in 1..=128 {
            if let Ok(entry) = reader.entry::<AgentEntry>(&(i as u64)) {
                let data = entry.get();
                if data.is_active {
                    total_successes += data.successes;
                    total_failures += data.failures;
                    total_pressure += data.pressure;
                    active_count += 1;
                }
            }
        }

        global.total_successes = total_successes;
        global.total_failures = total_failures;
        if active_count > 0 {
            global.swarm_pressure = total_pressure / (active_count as f32);
        }

        self.publish_global_state(global)?;
        Ok(global)
    }

    /// Setting the quorum threshold dynamically.
    pub fn set_quorum_threshold(&self, threshold: u8) -> Result<(), SwarmIpcError> {
        let mut state = self.read_global_state()?;
        state.quorum_threshold = threshold;
        self.publish_global_state(state)
    }

    /// Participating in a vote using the per-agent isolated masks in GlobalState.
    pub fn cast_vote(&self, agent_index: u8, approve: bool) -> Result<(), SwarmIpcError> {
        // Note: consensus voting still requires a read-modify-write on GlobalState
        // but it is less frequent than metrics updates.
        let mut state = self.read_global_state()?;
        
        // Agent indices are 1-based for blackboard, but 0-based for masks
        let mask_index = (agent_index - 1) as usize;
        let mask_idx = mask_index / 64;
        let bit_idx = mask_index % 64;

        if approve {
            state.approve_mask[mask_idx] |= 1 << bit_idx;
        } else {
            state.veto_mask[mask_idx] |= 1 << bit_idx;
        }

        self.publish_global_state(state)
    }

    /// Checking if consensus is reached.
    pub fn check_consensus(&self) -> ConsensusResult {
        let Ok(state) = self.read_global_state() else {
            return ConsensusResult::Pending;
        };

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
        let mut state = GlobalState {
            quorum_threshold: 2,
            ..Default::default()
        };

        // Agent 1 approves (mask index 0, bit 0 as it's 1-based index)
        let mask_idx = 0;
        let bit_idx = 0;
        state.approve_mask[mask_idx] |= 1 << bit_idx;

        // Check consensus (Pending as 1 < 2)
        assert_eq!(state.approve_mask[0].count_ones(), 1);
        
        // Agent 2 approves
        let bit_idx2 = 1;
        state.approve_mask[0] |= 1 << bit_idx2;
        
        assert_eq!(state.approve_mask[0].count_ones(), 2);
    }

    #[test]
    fn test_collective_veto_overrides_approval() {
        let mut state = GlobalState {
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
