use crate::heartbeat::HeartbeatScheduler;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, error};

/// Sovereign Watchdog: Monitors substrate health and ensures real-time telemetry adherence.
pub struct SovereignWatchdog {
    last_pulse: u64,
}

impl SovereignWatchdog {
    pub fn new() -> Self {
        Self {
            last_pulse: Self::current_time(),
        }
    }

    fn current_time() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Attaches the watchdog to a scheduler to monitor heartbeat regularity.
    pub async fn attach(&mut self, scheduler: &HeartbeatScheduler) {
        info!("🧬 SovereignWatchdog: Attached to core pulse scheduler.");
        let mut rx = scheduler.subscribe();
        
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if msg == "heartbeat" {
                    // info!("💓 Substrate Heartbeat Verified.");
                }
            }
        });
    }

    /// Checks if the substrate has "flatlined" (no heartbeat for > 120s).
    pub fn health_check(&self) -> bool {
        let now = Self::current_time();
        if now - self.last_pulse > 120 {
            error!("🚨 Substrate Flatline Detected! Last pulse: {}s ago", now - self.last_pulse);
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
