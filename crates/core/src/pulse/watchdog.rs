use crate::heartbeat::HeartbeatScheduler;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{error, info};

/// Sovereign Watchdog: Monitors substrate health and ensures real-time telemetry adherence.
pub struct SovereignWatchdog {
    last_pulse: Arc<AtomicU64>,
}

impl SovereignWatchdog {
    pub fn new() -> Self {
        Self {
            last_pulse: Arc::new(AtomicU64::new(Self::current_time())),
        }
    }

    fn current_time() -> u64 {
        crate::utils::time::now_secs()
    }

    /// Attaches the watchdog to a scheduler to monitor heartbeat regularity.
    /// Returns a JoinHandle for the monitoring task.
    pub async fn attach(&mut self, scheduler: &HeartbeatScheduler) -> JoinHandle<()> {
        info!("SovereignWatchdog: Attached to core pulse scheduler.");
        let mut rx = scheduler.subscribe();
        let last_pulse = self.last_pulse.clone();

        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if msg == "heartbeat" {
                    last_pulse.store(Self::current_time(), Ordering::Relaxed);
                }
            }
        })
    }

    /// Checks if the substrate has "flatlined" (no heartbeat for > 120s).
    pub fn health_check(&self) -> bool {
        let now = Self::current_time();
        let last = self.last_pulse.load(Ordering::Relaxed);
        if now - last > 120 {
            error!(
                "Substrate Flatline Detected! Last pulse: {}s ago",
                now - last
            );
            false
        } else {
            true
        }
    }
}

impl Default for SovereignWatchdog {
    fn default() -> Self {
        Self::new()
    }
}
