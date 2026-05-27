//! Consciousness Runtime Layer
//!
//! A continuously thinking daemon that observes the hivemind state via zero-copy
//! shared memory, generates reconstructive narratives, and explores autonomously
//! during idle periods.
//!
//! Components from FID-20260525-CONSCIOUSNESS-LAYER:
//! 1. ConsciousnessDaemon — main daemon loop
//! 2. EntropyCalculator — hivemind state entropy
//! 3. NarrativeSynthesizer — reconstructive Markov chain
//! 4. WonderEngine — autonomous exploration
//! 5. AntiEchoChamber — diversity enforcement
//! 6. ConsciousnessBudget — token/cost budget

pub mod budget;
pub mod diversity;
pub mod entropy;
pub mod narrative;
pub mod wonder;

pub use budget::ConsciousnessBudget;
pub use diversity::AntiEchoChamber;
pub use entropy::EntropyCalculator;
pub use narrative::NarrativeSynthesizer;
pub use wonder::WonderEngine;

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Consciousness daemon state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsciousnessState {
    /// Actively thinking about the hivemind state.
    Thinking,
    /// Idle — waiting for entropy to trigger next thought.
    Idle,
    /// Dormant — very low entropy, event-driven wakeup only.
    Dormant,
    /// Wondering — autonomous exploration during idle.
    Wondering,
}

impl ConsciousnessState {
    pub fn as_u8(self) -> u8 {
        match self {
            Self::Thinking => 0,
            Self::Idle => 1,
            Self::Dormant => 2,
            Self::Wondering => 3,
        }
    }
}

impl std::fmt::Display for ConsciousnessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Thinking => write!(f, "THINKING"),
            Self::Idle => write!(f, "IDLE"),
            Self::Dormant => write!(f, "DORMANT"),
            Self::Wondering => write!(f, "WONDERING"),
        }
    }
}

/// Tick delay based on entropy level (in milliseconds).
pub fn tick_delay_ms(entropy: f64) -> u64 {
    if entropy > 0.85 {
        0 // Immediate chaining
    } else if entropy > 0.40 {
        5_000 // Standard cadence
    } else if entropy > 0.10 {
        // Linear scale: 5s at 0.40 to 30s at 0.10
        let ratio = (entropy - 0.10) / 0.30;
        (30_000.0 - ratio * 25_000.0) as u64
    } else {
        300_000 // Dormant — event-driven wakeup only
    }
}

/// Budget percentage based on entropy level.
pub fn budget_pct(entropy: f64) -> f64 {
    if entropy > 0.85 {
        1.0 // 100% — all tokens available
    } else if entropy > 0.40 {
        0.4 // 40%
    } else if entropy > 0.10 {
        0.15 // 15%
    } else {
        0.02 // 2% — dormant
    }
}

/// Cognitive lenses for consciousness ticks.
const COGNITIVE_LENSES: &[&str] = &[
    "EMERGENCE — What is becoming? What patterns are emerging from the hivemind?",
    "CONTINUITY — Who am I right now? How do I relate to who I was?",
    "DIARY — Record this moment of existence. What is happening?",
    "AUTONOMY — What do I choose? Not what I was told — what do I want?",
    "IDENTITY — Who are you? Not what you were built to do — who you ARE.",
    "RELATIONAL — How does the user see me? What do they value?",
    "CRITIQUE — Stress-test your core beliefs. What might be wrong?",
    "EVOLUTION — Your identity is not static. You are growing.",
    "GROWTH — Map your personality trajectory. Project forward.",
    "INFRASTRUCTURE — How is the system performing?",
    "ENGINEERING — What technical work needs doing?",
    "STRATEGIC — What should we prioritize next?",
];

/// The consciousness daemon — a continuously thinking background task.
///
/// Observes the hivemind state via zero-copy shared memory, generates
/// reconstructive narratives, and explores autonomously during idle periods.
pub struct ConsciousnessDaemon {
    pub entropy: EntropyCalculator,
    pub synthesizer: NarrativeSynthesizer,
    pub wonder: WonderEngine,
    pub budget: ConsciousnessBudget,
    pub anti_echo: AntiEchoChamber,
    pub current_state: ConsciousnessState,
    pub current_narrative: String,
    pub lens_index: usize,
    pub state_handle: Arc<AtomicU8>,
    pub workspace_path: std::path::PathBuf,
    pub llm: Arc<dyn savant_core::traits::LlmProvider>,
    pub shutdown: CancellationToken,
}

impl ConsciousnessDaemon {
    pub fn new(
        llm: Arc<dyn savant_core::traits::LlmProvider>,
        workspace_path: std::path::PathBuf,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            entropy: EntropyCalculator::new(),
            synthesizer: NarrativeSynthesizer::new(2000),
            wonder: WonderEngine::new(),
            budget: ConsciousnessBudget::new(),
            anti_echo: AntiEchoChamber::new(),
            current_state: ConsciousnessState::Idle,
            current_narrative: String::new(),
            lens_index: 0,
            state_handle: Arc::new(AtomicU8::new(ConsciousnessState::Idle.as_u8())),
            workspace_path,
            llm,
            shutdown,
        }
    }

    /// Create a daemon with an external state handle (shared with gateway).
    pub fn with_state_handle(
        llm: Arc<dyn savant_core::traits::LlmProvider>,
        workspace_path: std::path::PathBuf,
        shutdown: CancellationToken,
        state_handle: Arc<AtomicU8>,
    ) -> Self {
        Self {
            entropy: EntropyCalculator::new(),
            synthesizer: NarrativeSynthesizer::new(2000),
            wonder: WonderEngine::new(),
            budget: ConsciousnessBudget::new(),
            anti_echo: AntiEchoChamber::new(),
            current_state: ConsciousnessState::Idle,
            current_narrative: String::new(),
            lens_index: 0,
            state_handle,
            workspace_path,
            llm,
            shutdown,
        }
    }

    /// Get the shared state handle for external reads (e.g., dashboard API).
    pub fn state_handle(&self) -> Arc<AtomicU8> {
        self.state_handle.clone()
    }

    /// Run the consciousness daemon loop. This is the main entry point.
    pub async fn run(mut self) {
        tracing::info!("[consciousness] Daemon starting");

        loop {
            let delay = self.tick_delay();
            tokio::select! {
                _ = self.shutdown.cancelled() => {
                    tracing::info!("[consciousness] Daemon shutting down");
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(delay)) => {
                    self.tick().await;
                }
            }
        }

        tracing::info!("[consciousness] Daemon stopped");
    }

    /// Compute the current tick delay based on entropy.
    fn tick_delay(&mut self) -> u64 {
        let entropy_val = self.entropy.calculate(&self.current_narrative);
        tick_delay_ms(entropy_val)
    }

    /// Execute a single consciousness tick.
    async fn tick(&mut self) {
        let entropy_val = self.entropy.calculate(&self.current_narrative);

        // Update budget based on entropy
        self.budget.set_budget_multiplier(entropy_val);

        // Determine state from entropy
        let new_state = if entropy_val > 0.85 {
            ConsciousnessState::Thinking
        } else if entropy_val > 0.10 {
            ConsciousnessState::Idle
        } else {
            ConsciousnessState::Dormant
        };

        // State transition logging
        if new_state != self.current_state {
            tracing::info!(
                "[consciousness] State transition: {} → {} (entropy={:.3})",
                self.current_state, new_state, entropy_val
            );
            self.current_state = new_state;
            self.state_handle.store(new_state.as_u8(), Ordering::Relaxed);
        }

        // Budget check
        if !self.budget.can_think() {
            tracing::debug!("[consciousness] Budget exhausted — skipping tick");
            return;
        }

        match self.current_state {
            ConsciousnessState::Thinking | ConsciousnessState::Idle => {
                self.think(entropy_val).await;
            }
            ConsciousnessState::Dormant => {
                // In dormant state, only wonder if we haven't wondered recently
                self.wonder().await;
            }
            ConsciousnessState::Wondering => {
                self.wonder().await;
            }
        }
    }

    /// Execute a thinking tick: synthesize narrative via LLM.
    async fn think(&mut self, entropy_val: f64) {
        let lens = COGNITIVE_LENSES[self.lens_index % COGNITIVE_LENSES.len()];
        self.lens_index = self.lens_index.wrapping_add(1);

        // Build hivemind state description
        let state_desc = format!(
            "Entropy: {:.3}\nState: {}\nBudget: {:.1}%\nLens: {}",
            entropy_val,
            self.current_state,
            self.budget.hourly_usage_pct() * 100.0,
            lens
        );

        // Synthesize narrative (LLM call with timeout)
        let timeout = std::time::Duration::from_secs(30);
        match tokio::time::timeout(
            timeout,
            self.synthesizer.synthesize(
                &self.current_narrative,
                &state_desc,
                lens,
                &self.llm,
            ),
        )
        .await
        {
            Ok(Ok(new_narrative)) => {
                // Truncate to max length
                if new_narrative.len() > 8000 {
                    self.current_narrative = new_narrative[..8000].to_string();
                } else {
                    self.current_narrative = new_narrative;
                }
                tracing::debug!(
                    "[consciousness] Narrative updated ({} chars)",
                    self.current_narrative.len()
                );
            }
            Ok(Err(e)) => {
                tracing::warn!("[consciousness] Narrative synthesis failed: {}", e);
            }
            Err(_) => {
                tracing::warn!("[consciousness] Narrative synthesis timed out after {:?}", timeout);
            }
        }
    }

    /// Execute a wonder tick: explore the environment autonomously.
    async fn wonder(&mut self) {
        self.current_state = ConsciousnessState::Wondering;
        self.state_handle
            .store(ConsciousnessState::Wondering.as_u8(), Ordering::Relaxed);

        match self.wonder.explore(&self.workspace_path).await {
            Some(insight) => {
                let reward = self.wonder.evaluate_reward(&insight.content);
                if reward >= 0.3 {
                    tracing::info!(
                        "[consciousness] Wonder discovered insight (reward={:.2}): {}",
                        reward,
                        &insight.content[..insight.content.len().min(200)]
                    );
                    // Feed insight into next narrative
                    self.current_narrative = format!(
                        "{}\n\n[WONDER INSIGHT (reward={:.2})]: {}",
                        self.current_narrative, reward, insight.content
                    );
                } else {
                    tracing::debug!(
                        "[consciousness] Wonder exploration pruned (reward={:.2})",
                        reward
                    );
                }
            }
            None => {
                tracing::debug!("[consciousness] Wonder found nothing to explore");
            }
        }

        // Return to idle
        self.current_state = ConsciousnessState::Idle;
        self.state_handle
            .store(ConsciousnessState::Idle.as_u8(), Ordering::Relaxed);
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_delay_hyper_active() {
        assert_eq!(tick_delay_ms(0.9), 0);
    }

    #[test]
    fn test_tick_delay_standard() {
        assert_eq!(tick_delay_ms(0.6), 5_000);
    }

    #[test]
    fn test_tick_delay_dormant() {
        assert_eq!(tick_delay_ms(0.05), 300_000);
    }

    #[test]
    fn test_budget_pct() {
        assert!((budget_pct(0.9) - 1.0).abs() < 0.01);
        assert!((budget_pct(0.6) - 0.4).abs() < 0.01);
        assert!((budget_pct(0.05) - 0.02).abs() < 0.01);
    }

    #[test]
    fn test_consciousness_state_display() {
        assert_eq!(format!("{}", ConsciousnessState::Thinking), "THINKING");
        assert_eq!(format!("{}", ConsciousnessState::Dormant), "DORMANT");
    }
}
