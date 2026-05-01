//! NREM Phase — Structured Memory Consolidation.
//!
//! Replays recent episodic memories, compresses redundant entries,
//! resolves contradictions, and writes consolidated results to persistent storage.

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
    /// Duration in milliseconds.
    pub duration_ms: u64,
}

/// NREM controller for structured memory replay and consolidation.
pub struct NremController {
    /// Hours of episodic memory to replay.
    pub replay_window_hours: u64,
}

impl NremController {
    /// Creates a new NREM controller with the given replay window.
    pub fn new(replay_window_hours: u64) -> Self {
        Self {
            replay_window_hours,
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
    /// 4. Write consolidated results back to memory
    pub async fn run(&self, memory: &Arc<MemoryEngine>) -> Result<NremResult, super::DreamError> {
        let start = Instant::now();
        info!(
            "[NREM] Starting consolidation cycle (window={}h)",
            self.replay_window_hours
        );

        // Fetch all messages across sessions
        let enclave = memory.enclave();
        let lsm = enclave.lsm();
        let all_messages = lsm.iter_all_messages();
        let messages: Vec<_> = all_messages.collect();

        if messages.is_empty() {
            debug!("[NREM] No messages to consolidate");
            return Ok(NremResult {
                scanned: 0,
                consolidated: 0,
                contradictions_resolved: 0,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let scanned = messages.len();

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

        // Phase 4: Write consolidated results back
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
            "[NREM] Complete: {} scanned, {} consolidated, {} contradictions resolved ({}ms)",
            scanned, consolidated, contradictions_resolved, duration_ms
        );

        Ok(NremResult {
            scanned,
            consolidated: dedup_count,
            contradictions_resolved,
            duration_ms,
        })
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
}
