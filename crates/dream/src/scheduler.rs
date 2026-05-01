//! Dream Scheduler — Manages NREM/REM cycle timing.
//!
//! Triggers dream cycles during idle periods, yields immediately
//! when environment becomes active. Uses atomic `IS_DREAMING` flag
//! to coordinate with heartbeat pulse.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use savant_memory::MemoryEngine;
use tokio::sync::watch;
use tracing::{debug, info, warn};

use super::{DreamConfig, DreamCycleResult, IS_DREAMING};

/// Dream scheduler that manages NREM/REM cycle timing.
pub struct DreamScheduler {
    config: DreamConfig,
    memory: Arc<MemoryEngine>,
    /// Receiver for delta score updates (from heartbeat).
    delta_rx: watch::Receiver<f32>,
}

impl DreamScheduler {
    /// Creates a new dream scheduler.
    pub fn new(
        config: DreamConfig,
        memory: Arc<MemoryEngine>,
        delta_rx: watch::Receiver<f32>,
    ) -> Self {
        Self {
            config,
            memory,
            delta_rx,
        }
    }

    /// Runs the dream scheduling loop.
    ///
    /// Monitors environment delta and triggers NREM/REM cycles
    /// during idle periods. Yields immediately when activity detected.
    pub async fn run(self) {
        if !self.config.enabled {
            info!("[DreamScheduler] Dream engine disabled");
            return;
        }

        info!(
            "[DreamScheduler] Online (idle_threshold={}, idle_min={}, nrem={}s, rem={}s)",
            self.config.idle_threshold,
            self.config.idle_minutes,
            self.config.nrem_duration_secs,
            self.config.rem_duration_secs,
        );

        let mut idle_start: Option<Instant> = None;
        let check_interval = Duration::from_secs(30);

        loop {
            tokio::time::sleep(check_interval).await;

            let current_delta = *self.delta_rx.borrow();

            if current_delta < self.config.idle_threshold {
                // Environment is idle
                if idle_start.is_none() {
                    idle_start = Some(Instant::now());
                    debug!(
                        "[DreamScheduler] Idle detected (delta={:.2})",
                        current_delta
                    );
                }

                let idle_duration = idle_start.map(|s| s.elapsed().as_secs() / 60).unwrap_or(0);

                if idle_duration >= self.config.idle_minutes {
                    // Sufficient idle time — trigger dream cycle
                    info!(
                        "[DreamScheduler] Triggering dream cycle (idle={}min, delta={:.2})",
                        idle_duration, current_delta
                    );

                    match self.run_dream_cycle().await {
                        Ok(result) => {
                            info!(
                                "[DreamScheduler] Cycle complete: consolidated={}, associations={}, vendi={:.2}, interrupted={}",
                                result.nrem_consolidated,
                                result.rem_associations,
                                result.rem_vendi_score,
                                result.interrupted,
                            );
                        }
                        Err(e) => {
                            warn!("[DreamScheduler] Cycle failed: {}", e);
                        }
                    }

                    // Reset idle timer after cycle
                    idle_start = None;
                }
            } else {
                // Activity detected — reset idle tracking
                if idle_start.is_some() {
                    debug!(
                        "[DreamScheduler] Activity detected (delta={:.2}), resetting idle timer",
                        current_delta
                    );
                }
                idle_start = None;
            }
        }
    }

    /// Runs a single dream cycle (NREM + REM).
    async fn run_dream_cycle(&self) -> Result<DreamCycleResult, super::DreamError> {
        let cycle_start = Instant::now();
        let cycle_id = uuid::Uuid::new_v4().to_string();

        // Set dreaming flag — heartbeat pulse will skip while this is true
        IS_DREAMING.store(true, Ordering::SeqCst);
        info!("[DreamScheduler] Dream cycle {} started", cycle_id);

        // NREM Phase
        let nrem_controller = super::nrem::NremController::new(24);
        let nrem_result = match tokio::time::timeout(
            Duration::from_secs(self.config.nrem_duration_secs),
            nrem_controller.run(&self.memory),
        )
        .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => {
                IS_DREAMING.store(false, Ordering::SeqCst);
                return Err(e);
            }
            Err(_) => {
                warn!("[DreamScheduler] NREM phase timed out");
                IS_DREAMING.store(false, Ordering::SeqCst);
                return Err(super::DreamError::Interrupted);
            }
        };

        // Check if environment became active during NREM
        if *self.delta_rx.borrow() > self.config.idle_threshold * 3.0 {
            warn!("[DreamScheduler] Environment active during NREM, aborting REM");
            IS_DREAMING.store(false, Ordering::SeqCst);
            return Ok(DreamCycleResult {
                cycle_id,
                nrem_consolidated: nrem_result.consolidated,
                rem_associations: 0,
                rem_vendi_score: 0.0,
                duration_ms: cycle_start.elapsed().as_millis() as u64,
                interrupted: true,
            });
        }

        // REM Phase
        let rem_controller = super::rem::RemController::default_controller();
        let rem_result = match tokio::time::timeout(
            Duration::from_secs(self.config.rem_duration_secs),
            rem_controller.run(&self.memory, self.config.vendi_threshold),
        )
        .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => {
                IS_DREAMING.store(false, Ordering::SeqCst);
                return Err(e);
            }
            Err(_) => {
                warn!("[DreamScheduler] REM phase timed out");
                IS_DREAMING.store(false, Ordering::SeqCst);
                return Err(super::DreamError::Interrupted);
            }
        };

        // Clear dreaming flag
        IS_DREAMING.store(false, Ordering::SeqCst);

        Ok(DreamCycleResult {
            cycle_id,
            nrem_consolidated: nrem_result.consolidated,
            rem_associations: if rem_result.passed_filter {
                rem_result.associations.len()
            } else {
                0
            },
            rem_vendi_score: rem_result.vendi_score,
            duration_ms: cycle_start.elapsed().as_millis() as u64,
            interrupted: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dreaming_flag_operations() {
        IS_DREAMING.store(false, Ordering::SeqCst);
        assert!(!IS_DREAMING.load(Ordering::SeqCst));

        IS_DREAMING.store(true, Ordering::SeqCst);
        assert!(IS_DREAMING.load(Ordering::SeqCst));

        IS_DREAMING.store(false, Ordering::SeqCst);
        assert!(!IS_DREAMING.load(Ordering::SeqCst));
    }

    #[test]
    fn test_dream_config_defaults() {
        let config = DreamConfig::default();
        assert!(config.enabled);
        assert_eq!(config.nrem_duration_secs, 300);
        assert_eq!(config.rem_duration_secs, 180);
        assert!((config.idle_threshold - 0.1).abs() < f32::EPSILON);
        assert_eq!(config.idle_minutes, 10);
        assert!((config.vendi_threshold - 0.3).abs() < f32::EPSILON);
    }
}
