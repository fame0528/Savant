//! Environmental Delta — Threshold-Based Activation
//!
//! Replaces the fixed 60-second heartbeat clock with a change-detection system.
//! The LLM is only invoked when the environment has changed meaningfully.
//! This eliminates forced emergence on static environments.

use std::time::Instant;

/// Environmental change detector. Computes a weighted score from
/// environmental signals. If the score exceeds the threshold, the
/// agent should invoke the LLM. If below threshold, skip the pulse.
pub struct EnvironmentalDelta {
    /// Lines changed in git diff since last pulse
    pub git_lines_changed: usize,
    /// Files modified in filesystem since last pulse
    pub files_modified: usize,
    /// New messages received since last pulse
    pub new_messages: usize,
    /// Tool errors since last pulse
    pub tool_errors: usize,
    /// Minutes since last LLM invocation
    pub minutes_since_last_pulse: u64,
}

impl EnvironmentalDelta {
    /// Computes environmental change score [0.0, 1.0].
    ///
    /// Weights:
    /// - git changes: 25% (most meaningful signal)
    /// - new messages: 20% (user/agent interaction)
    /// - filesystem changes: 15% (file modifications)
    /// - time decay: 35% (forces pulse at ~8.5 minutes to prevent permanent dormancy)
    /// - tool errors: 5% (error-driven reflection)
    ///
    /// Forced pulse: at 8.57 minutes, time component alone reaches 0.3 (threshold).
    pub fn score(&self) -> f32 {
        let git = (self.git_lines_changed as f32 / 100.0).min(1.0) * 0.25;
        let fs = (self.files_modified as f32 / 10.0).min(1.0) * 0.15;
        let msgs = (self.new_messages as f32 / 5.0).min(1.0) * 0.20;
        let errors = (self.tool_errors as f32 / 3.0).min(1.0) * 0.05;
        let time = (self.minutes_since_last_pulse as f32 / 10.0).min(1.0) * 0.35;
        git + fs + msgs + errors + time
    }

    /// Returns true if the agent should invoke the LLM.
    pub fn should_activate(&self, threshold: f32) -> bool {
        self.score() >= threshold
    }
}

/// Tracks state between pulses to compute deltas.
pub struct DeltaTracker {
    last_pulse_time: Instant,
    last_git_hash: u64,
    last_fs_snapshot: Vec<(String, u64)>,
    new_messages_count: usize,
    tool_errors_count: usize,
}

impl DeltaTracker {
    pub fn new() -> Self {
        Self {
            last_pulse_time: Instant::now(),
            last_git_hash: 0,
            last_fs_snapshot: Vec::new(),
            new_messages_count: 0,
            tool_errors_count: 0,
        }
    }

    /// Record a new message received since last pulse.
    pub fn record_message(&mut self) {
        self.new_messages_count += 1;
    }

    /// Record a tool error since last pulse.
    pub fn record_tool_error(&mut self) {
        self.tool_errors_count += 1;
    }

    /// Compute the current environmental delta and reset counters.
    pub fn compute_and_reset(
        &mut self,
        git_lines_changed: usize,
        files_modified: usize,
    ) -> EnvironmentalDelta {
        let minutes = self.last_pulse_time.elapsed().as_secs() / 60;

        let delta = EnvironmentalDelta {
            git_lines_changed,
            files_modified,
            new_messages: self.new_messages_count,
            tool_errors: self.tool_errors_count,
            minutes_since_last_pulse: minutes,
        };

        // Reset counters
        self.last_pulse_time = Instant::now();
        self.new_messages_count = 0;
        self.tool_errors_count = 0;

        delta
    }
}
