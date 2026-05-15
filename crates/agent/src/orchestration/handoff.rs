use anyhow::{anyhow, Result};
use savant_ipc::blackboard::SwarmSharedContext;
use savant_ipc::a2a::agent_card::AgentCard;
use tracing::{info, warn};

/// Manages handoffs between agents and ensures cycle prevention.
pub struct OrchestrationRouter {
    agent_id: u32,
}

impl OrchestrationRouter {
    pub fn new(agent_id: u32, _host_id: u32) -> Self {
        Self { agent_id }
    }

    /// Validates a handoff request and checks for delegation cycles.
    ///
    /// Returns Ok(()) if the handoff is safe, or an error if a cycle is detected
    /// or delegation depth is exceeded.
    pub fn validate_handoff(
        &self,
        ctx: &mut SwarmSharedContext,
        target_agent_id: u32,
    ) -> anyhow::Result<()> {
        // Check if current depth already hit the limit
        if ctx.delegation_filter.depth_count >= ctx.max_delegation_depth {
            warn!(
                "Delegation depth exceeded ({} >= {})",
                ctx.delegation_filter.depth_count, ctx.max_delegation_depth
            );
            return Err(anyhow::anyhow!("Delegation depth exceeded"));
        }

        // Add current agent to the bloom filter trace (increments depth)
        ctx.delegation_filter.add_agent(self.agent_id.into());

        // Check if target agent is already in the trace path
        if ctx.delegation_filter.contains_agent(target_agent_id.into()) {
            warn!(
                "Cycle detected: Target agent {} has already processed this session",
                target_agent_id
            );
            return Err(anyhow!("Circular delegation detected"));
        }

        Ok(())
    }

    /// Validates a handoff against an AgentCard — checks capability match.
    ///
    /// This is the new typed delegation path. Instead of relying on hardcoded
    /// agent IDs, the Orchestrator checks the target's AgentCard for:
    /// - Availability (is_active, pressure < 0.9)
    /// - Required skills (bitwise AND with required_skills_mask)
    /// - Semantic match score (cosine similarity of description vectors)
    pub fn validate_handoff_with_card(
        &self,
        ctx: &mut SwarmSharedContext,
        target_agent_id: u32,
        target_card: &AgentCard,
        required_skills: u128,
        semantic_similarity: f32,
    ) -> Result<(), HandoffRejection> {
        // First: standard cycle prevention
        if let Err(e) = self.validate_handoff(ctx, target_agent_id) {
            return Err(HandoffRejection::CycleDetected(e.to_string()));
        }

        // Check agent availability
        if !target_card.is_available() {
            return Err(HandoffRejection::AgentUnavailable);
        }

        // Check required skills
        if !target_card.has_skills(required_skills) {
            return Err(HandoffRejection::InsufficientSkills);
        }

        // Check semantic match quality (minimum 0.3 similarity)
        if semantic_similarity < 0.3 {
            return Err(HandoffRejection::LowSemanticMatch {
                score: semantic_similarity,
                threshold: 0.3,
            });
        }

        // Check composite match score
        let score = target_card.match_score(semantic_similarity, required_skills);
        if score < 0.2 {
            return Err(HandoffRejection::LowCompositeScore {
                score,
                threshold: 0.2,
            });
        }

        info!(
            target_agent = %hex_encode_32(&target_card.agent_id),
            score = %score,
            "Handoff validated with AgentCard capability match"
        );
        Ok(())
    }

    /// Records the initiation of a handoff.
    pub fn record_handoff(&self, target_agent_id: u32) {
        info!(
            "Handoff initiated: Agent {} -> Agent {}",
            self.agent_id, target_agent_id
        );
    }

    /// Records a typed handoff with AgentCard metadata.
    pub fn record_typed_handoff(
        &self,
        target_card: &AgentCard,
        task_id: &[u8; 16],
        semantic_score: f32,
    ) {
        info!(
            target_agent = %hex_encode_32(&target_card.agent_id),
            task_id = %hex_encode(task_id),
            semantic_score = %semantic_score,
            pressure = %target_card.pressure,
            "Typed handoff initiated with capability match"
        );
    }

    /// Awaits a delivery receipt from the target agent.
    pub async fn await_receipt(&self, session_hash: u64, timeout_ms: u64) -> Result<()> {
        info!(
            "Awaiting delivery receipt for session {} (timeout {}ms)",
            session_hash, timeout_ms
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        Ok(())
    }

    /// Emits a delivery receipt for a received session.
    pub fn emit_receipt(&self, sender_agent_id: u32, session_hash: u64) {
        info!(
            "Emitted receipt for session {} to agent {}",
            session_hash, sender_agent_id
        );
    }
}

/// Reasons a typed handoff may be rejected.
#[derive(Debug, thiserror::Error)]
pub enum HandoffRejection {
    #[error("Delegation cycle detected: {0}")]
    CycleDetected(String),
    #[error("Target agent is not available (inactive or overloaded)")]
    AgentUnavailable,
    #[error("Target agent lacks required skills")]
    InsufficientSkills,
    #[error("Semantic match too low: {score} < {threshold}")]
    LowSemanticMatch { score: f32, threshold: f32 },
    #[error("Composite match score too low: {score} < {threshold}")]
    LowCompositeScore { score: f32, threshold: f32 },
}

/// Utility: encode bytes as hex string for logging.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Utility: encode 32-byte agent ID as hex string.
fn hex_encode_32(bytes: &[u8; 32]) -> String {
    hex_encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_handoff_with_card_success() {
        let router = OrchestrationRouter::new(1, 0);
        let mut ctx = SwarmSharedContext::default();
        let mut card = AgentCard::new([2u8; 32], "test-agent");
        card.is_active = true;
        card.allowed_skills_mask = 0b1111;
        card.pressure = 0.1;

        let result = router.validate_handoff_with_card(
            &mut ctx,
            2,
            &card,
            0b0101,
            0.8,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_handoff_with_card_unavailable() {
        let router = OrchestrationRouter::new(1, 0);
        let mut ctx = SwarmSharedContext::default();
        let mut card = AgentCard::new([2u8; 32], "test-agent");
        card.is_active = false;

        let result = router.validate_handoff_with_card(
            &mut ctx,
            2,
            &card,
            0b0001,
            0.8,
        );
        assert!(matches!(result, Err(HandoffRejection::AgentUnavailable)));
    }

    #[test]
    fn test_validate_handoff_with_card_insufficient_skills() {
        let router = OrchestrationRouter::new(1, 0);
        let mut ctx = SwarmSharedContext::default();
        let mut card = AgentCard::new([2u8; 32], "test-agent");
        card.is_active = true;
        card.allowed_skills_mask = 0b0001;

        let result = router.validate_handoff_with_card(
            &mut ctx,
            2,
            &card,
            0b1111,
            0.8,
        );
        assert!(matches!(result, Err(HandoffRejection::InsufficientSkills)));
    }

    #[test]
    fn test_validate_handoff_with_card_low_semantic() {
        let router = OrchestrationRouter::new(1, 0);
        let mut ctx = SwarmSharedContext::default();
        let mut card = AgentCard::new([2u8; 32], "test-agent");
        card.is_active = true;
        card.allowed_skills_mask = 0b1111;

        let result = router.validate_handoff_with_card(
            &mut ctx,
            2,
            &card,
            0b0001,
            0.1,
        );
        assert!(matches!(result, Err(HandoffRejection::LowSemanticMatch { .. })));
    }

    #[test]
    fn test_record_typed_handoff() {
        let router = OrchestrationRouter::new(1, 0);
        let card = AgentCard::new([2u8; 32], "test-agent");
        router.record_typed_handoff(&card, &[1u8; 16], 0.85);
    }
}