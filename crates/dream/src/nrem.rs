//! NREM Phase — Structured Memory Consolidation.
//!
//! Replays recent episodic memories, compresses redundant entries,
//! resolves contradictions, and writes consolidated results to persistent storage.
//!
//! # Relevance-Conditioned Logarithmic Decay
//! Memory weights decrease logarithmically from last access:
//! `w(t) = w0 * log(e + t) * spike_factor(access_count)`
//! Below threshold → cold storage eligible. Spikes on re-access or re-linking.

use std::sync::Arc;
use std::time::Instant;

use savant_memory::MemoryEngine;
use tracing::{debug, info, warn};

/// Result of an NREM consolidation cycle.
#[derive(Debug, Clone)]
pub struct NremResult {
    /// Number of memories scanned.
    pub scanned: usize,
    /// Number of memories consolidated (deduplicated + compressed).
    pub consolidated: usize,
    /// Number of contradictions resolved.
    pub contradictions_resolved: usize,
    /// IDs of memories marked for cold storage (below decay threshold).
    pub cold_storage_eligible: Vec<u64>,
    /// Duration in milliseconds.
    pub duration_ms: u64,
}

/// Consolidation event emitted to the vault outbox after NREM Phase 3.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConsolidationEvent {
    /// Unique event identifier.
    pub event_id: String,
    /// IDs of memories that were consolidated (deduplicated/compressed).
    pub consolidated_ids: Vec<u64>,
    /// IDs of memories archived to cold storage (below decay threshold).
    pub archived_ids: Vec<u64>,
    /// New synthesis generated from consolidation (if any).
    pub new_synthesis: Option<String>,
    /// Timestamp of the consolidation event.
    pub timestamp: i64,
}

/// Relevance-conditioned logarithmic decay function.
///
/// Weight decreases logarithmically from last access time.
/// `w(t) = w0 * ln(e + t_hours) * spike_factor(access_count)`
///
/// The spike factor resets weight toward original on re-access or re-linking:
/// `spike_factor(n) = 1.0 + 0.2 * ln(1 + n)` where n = access_count
///
/// Below threshold → cold storage eligible.
pub fn compute_decay_weight(
    initial_weight: f32,
    age_hours: f32,
    access_count: u32,
    referenced_by_others: bool,
) -> f32 {
    // Logarithmic decay: weight decreases slowly over time
    let decay_factor = (1.0 + age_hours).ln().max(0.01);

    // Spike factor: re-access or re-linking pushes weight back up
    let spike = 1.0 + 0.2 * (1.0 + access_count as f32).ln();

    // Reference bonus: memories linked by others are more relevant
    let reference_bonus = if referenced_by_others { 1.3 } else { 1.0 };

    // Normalized: divide by decay, multiply by spike and reference
    let weight = initial_weight / decay_factor.max(1.0) * spike * reference_bonus;

    weight.clamp(0.01, 1.0)
}

/// Determines if a memory weight is below the cold storage threshold.
pub fn is_cold_storage_eligible(weight: f32, threshold: f32) -> bool {
    weight < threshold
}

/// Default decay threshold for cold storage eligibility.
pub const DEFAULT_DECAY_THRESHOLD: f32 = 0.15;

/// NREM controller for structured memory replay and consolidation.
pub struct NremController {
    /// Hours of episodic memory to replay.
    pub replay_window_hours: u64,
    /// Decay threshold below which memories are cold-storage eligible.
    pub decay_threshold: f32,
}

impl NremController {
    /// Creates a new NREM controller with the given replay window.
    pub fn new(replay_window_hours: u64) -> Self {
        Self {
            replay_window_hours,
            decay_threshold: DEFAULT_DECAY_THRESHOLD,
        }
    }

    /// Creates a NREM controller with custom decay threshold.
    pub fn with_decay_threshold(replay_window_hours: u64, decay_threshold: f32) -> Self {
        Self {
            replay_window_hours,
            decay_threshold,
        }
    }

    /// Creates a default NREM controller (24 hour replay window).
    pub fn default_controller() -> Self {
        Self::new(24)
    }

    /// Runs the NREM consolidation cycle.
    ///
    /// # Process
    /// 1. Fetch recent messages from all sessions (last N hours)
    /// 2. Deduplicate consecutive identical messages
    /// 3. Detect and resolve contradictions (keep newer + higher importance)
    /// 4. Apply relevance-conditioned logarithmic decay to memory weights
    /// 5. Mark below-threshold memories as cold-storage eligible
    /// 6. Write consolidated results back to memory
    /// 7. Emit ConsolidationEvent to outbox for vault projection
    pub async fn run(&self, memory: &Arc<MemoryEngine>) -> Result<(NremResult, Option<ConsolidationEvent>), super::DreamError> {
        let start = Instant::now();
        info!(
            "[NREM] Starting consolidation cycle (window={}h, decay_threshold={:.2})",
            self.replay_window_hours, self.decay_threshold
        );

        // Fetch all messages across sessions
        let enclave = memory.enclave();
        let lsm = enclave.lsm();
        let all_messages = lsm.iter_all_messages(5000);
        let messages: Vec<_> = all_messages.collect();

        if messages.is_empty() {
            debug!("[NREM] No messages to consolidate");
            return Ok((NremResult {
                scanned: 0,
                consolidated: 0,
                contradictions_resolved: 0,
                cold_storage_eligible: Vec::new(),
                duration_ms: start.elapsed().as_millis() as u64,
            }, None));
        }

        let scanned = messages.len();
        let now_ms = chrono::Utc::now().timestamp_millis();

        // Phase 1: Deduplicate consecutive identical messages
        let mut deduped = Vec::with_capacity(messages.len());
        let mut dedup_count = 0usize;

        for msg in &messages {
            if let Some(last) = deduped.last() {
                let last_msg: &savant_memory::AgentMessage = last;
                if last_msg.content == msg.content && last_msg.role == msg.role {
                    dedup_count += 1;
                    continue;
                }
            }
            deduped.push(msg.clone());
        }

        // Phase 2: Detect contradictions (simplified: messages with conflicting keywords)
        let contradictions = detect_contradictions(&deduped);

        // Phase 3: Resolve contradictions — keep the newer message
        let resolved = resolve_contradictions(deduped, &contradictions);
        let consolidated = resolved.len();
        let contradictions_resolved = contradictions.len();

        // Phase 4: Apply relevance-conditioned logarithmic decay
        let mut cold_storage_ids = Vec::new();
        let mut consolidated_ids = Vec::new();

        for msg in &resolved {
            let age_hours = (now_ms - i64::from(msg.timestamp)) as f32 / 3_600_000.0;

            // For AgentMessage, we use content length as a proxy for importance
            // and tool_calls count as a proxy for access/references
            let content_importance = (msg.content.len().min(1000) as f32) / 1000.0;
            let access_count = msg.tool_calls.len() as u32;
            let referenced = !msg.tool_calls.is_empty() || !msg.tool_results.is_empty();

            let weight = compute_decay_weight(
                content_importance,
                age_hours,
                access_count,
                referenced,
            );

            if is_cold_storage_eligible(weight, self.decay_threshold) {
                if let Ok(id_val) = msg.id.parse::<u64>() {
                    cold_storage_ids.push(id_val);
                }
            }

            if !msg.tool_calls.is_empty() {
                if let Ok(id_val) = msg.id.parse::<u64>() {
                    consolidated_ids.push(id_val);
                }
            }
        }

        if !cold_storage_ids.is_empty() {
            info!(
                "[NREM] {} memories below decay threshold ({:.2}) — cold storage eligible",
                cold_storage_ids.len(),
                self.decay_threshold
            );
        }

        // Phase 5: Write consolidated results back
        // Group by session and compact each session
        let mut sessions: std::collections::HashMap<String, Vec<savant_memory::AgentMessage>> =
            std::collections::HashMap::new();
        for msg in resolved {
            sessions
                .entry(msg.session_id.clone())
                .or_default()
                .push(msg);
        }

        for (session_id, session_messages) in &sessions {
            if let Err(e) = memory
                .enclave()
                .atomic_compact(session_id, session_messages.clone())
                .await
            {
                warn!("[NREM] Failed to compact session {}: {}", session_id, e);
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        info!(
            "[NREM] Complete: {} scanned, {} consolidated, {} contradictions resolved, {} cold-storage eligible ({}ms)",
            scanned, consolidated, contradictions_resolved, cold_storage_ids.len(), duration_ms
        );

        // Phase 6: Emit ConsolidationEvent to outbox for vault projection
        let consolidation_event = if !consolidated_ids.is_empty() || !cold_storage_ids.is_empty() {
            let event = ConsolidationEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                consolidated_ids: consolidated_ids.clone(),
                archived_ids: cold_storage_ids.clone(),
                new_synthesis: if contradictions_resolved > 0 {
                    Some(format!(
                        "Resolved {} contradictions across {} sessions",
                        contradictions_resolved,
                        sessions.len()
                    ))
                } else {
                    None
                },
                timestamp: chrono::Utc::now().timestamp(),
            };
            info!(
                "[NREM] Emitting ConsolidationEvent: {} consolidated, {} archived",
                event.consolidated_ids.len(),
                event.archived_ids.len()
            );
            Some(event)
        } else {
            None
        };

        Ok((NremResult {
            scanned,
            consolidated: dedup_count,
            contradictions_resolved,
            cold_storage_eligible: cold_storage_ids,
            duration_ms,
        }, consolidation_event))
    }
}

/// Detects contradictions in a list of messages.
/// Returns indices of contradictory message pairs.
fn detect_contradictions(messages: &[savant_memory::AgentMessage]) -> Vec<(usize, usize)> {
    let mut contradictions = Vec::new();

    let negation_patterns = [
        ("is", "is not"),
        ("was", "was not"),
        ("can", "cannot"),
        ("will", "will not"),
        ("should", "should not"),
        ("true", "false"),
        ("yes", "no"),
        ("enabled", "disabled"),
        ("active", "inactive"),
        ("passing", "failing"),
        ("success", "failure"),
    ];

    for i in 0..messages.len() {
        for j in (i + 1)..messages.len() {
            let a = messages[i].content.to_lowercase();
            let b = messages[j].content.to_lowercase();

            // Check if messages are about similar topics but with negation
            for (pos, neg) in &negation_patterns {
                if (a.contains(pos) && b.contains(neg)) || (a.contains(neg) && b.contains(pos)) {
                    // Verify they share enough context to be about the same topic
                    let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
                    let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();
                    let shared = words_a.intersection(&words_b).count();

                    if shared >= 3 {
                        contradictions.push((i, j));
                        break;
                    }
                }
            }
        }
    }

    contradictions
}

/// Resolves contradictions by keeping the newer message (higher index = newer).
fn resolve_contradictions(
    mut messages: Vec<savant_memory::AgentMessage>,
    contradictions: &[(usize, usize)],
) -> Vec<savant_memory::AgentMessage> {
    let mut to_remove = std::collections::HashSet::new();

    for &(i, j) in contradictions {
        // Keep the newer one (higher index), remove the older one
        to_remove.insert(i.min(j));
    }

    // Remove in reverse order to preserve indices
    let mut remove_indices: Vec<usize> = to_remove.into_iter().collect();
    remove_indices.sort_unstable();
    remove_indices.reverse();

    for idx in remove_indices {
        if idx < messages.len() {
            messages.remove(idx);
        }
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_contradictions() {
        use savant_memory::AgentMessage;

        let messages = vec![
            AgentMessage::user("s1", "The build is passing"),
            AgentMessage::user("s1", "The build is failing"),
        ];

        let contradictions = detect_contradictions(&messages);
        assert!(
            !contradictions.is_empty(),
            "Should detect build pass/fail contradiction"
        );
    }

    #[test]
    fn test_resolve_contradictions_keeps_newer() {
        use savant_memory::AgentMessage;

        let messages = vec![
            AgentMessage::user("s1", "The service is enabled"),
            AgentMessage::user("s1", "The service is disabled"),
        ];

        let contradictions = vec![(0, 1)];
        let resolved = resolve_contradictions(messages, &contradictions);

        assert_eq!(resolved.len(), 1);
        assert!(
            resolved[0].content.contains("disabled"),
            "Should keep the newer message"
        );
    }

    #[test]
    fn test_nrem_controller_default() {
        let controller = NremController::default_controller();
        assert_eq!(controller.replay_window_hours, 24);
    }

    #[test]
    fn test_decay_weight_no_decay() {
        // New memory (age=0), no accesses → should have high weight
        let weight = compute_decay_weight(1.0, 0.0, 0, false);
        assert!(weight > 0.5, "Fresh memory should have high weight, got {}", weight);
    }

    #[test]
    fn test_decay_weight_old_memory() {
        // Very old memory (1 year), no accesses → should have low weight
        let weight = compute_decay_weight(1.0, 8760.0, 0, false);
        assert!(weight < 0.3, "Old memory should have low weight, got {}", weight);
    }

    #[test]
    fn test_decay_weight_spike_on_access() {
        // Old memory but frequently accessed → spike factor kicks in
        let weight_no_access = compute_decay_weight(1.0, 100.0, 0, false);
        let weight_with_access = compute_decay_weight(1.0, 100.0, 10, false);
        assert!(
            weight_with_access > weight_no_access,
            "Frequently accessed memory should spike: {} > {}",
            weight_with_access,
            weight_no_access
        );
    }

    #[test]
    fn test_decay_weight_referenced_bonus() {
        // Referenced memory should have higher weight
        let weight_unreferenced = compute_decay_weight(1.0, 50.0, 0, false);
        let weight_referenced = compute_decay_weight(1.0, 50.0, 0, true);
        assert!(
            weight_referenced > weight_unreferenced,
            "Referenced memory should have bonus: {} > {}",
            weight_referenced,
            weight_unreferenced
        );
    }

    #[test]
    fn test_cold_storage_eligible() {
        assert!(is_cold_storage_eligible(0.1, 0.15));
        assert!(!is_cold_storage_eligible(0.5, 0.15));
        assert!(!is_cold_storage_eligible(0.15, 0.15)); // At threshold = not eligible
    }

    #[test]
    fn test_controller_custom_decay_threshold() {
        let controller = NremController::with_decay_threshold(48, 0.25);
        assert_eq!(controller.replay_window_hours, 48);
        assert!((controller.decay_threshold - 0.25).abs() < f32::EPSILON);
    }
}
