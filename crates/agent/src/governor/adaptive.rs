//! Adaptive Semaphore — adjusts agent concurrency based on resource pressure.

use savant_core::config::ResourceGovernorConfig;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit};

use super::monitor::ResourceMonitor;

/// Semaphore that adjusts its permit count based on system pressure.
pub struct AdaptiveSemaphore {
    inner: Arc<Semaphore>,
    monitor: Arc<ResourceMonitor>,
    config: ResourceGovernorConfig,
    current_max: AtomicUsize,
}

impl AdaptiveSemaphore {
    pub fn new(
        monitor: Arc<ResourceMonitor>,
        config: ResourceGovernorConfig,
    ) -> Self {
        let max = config.max_agents_low;
        Self {
            inner: Arc::new(Semaphore::new(max)),
            monitor,
            config,
            current_max: AtomicUsize::new(max),
        }
    }

    /// Try to acquire a spawn permit. Non-blocking.
    pub fn try_acquire(&self) -> Option<SemaphorePermit<'_>> {
        self.inner.try_acquire().ok()
    }

    /// Acquire a spawn permit (blocking).
    #[allow(clippy::disallowed_methods)]
    pub async fn acquire(&self) -> SemaphorePermit<'_> {
        self.inner.acquire().await.expect("semaphore closed")
    }

    /// Available permits.
    pub fn available_permits(&self) -> usize {
        self.inner.available_permits()
    }

    /// Adjust available permits based on current pressure.
    pub fn adjust_permits(&self) {
        let pressure = self.monitor.current_pressure();
        let target = pressure.max_agents(&self.config);
        let current = self.current_max.load(Ordering::Relaxed);

        if target == current {
            return;
        }

        if target > current {
            // Need more permits — add them
            let diff = target - current;
            self.inner.add_permits(diff);
            self.current_max.store(target, Ordering::Relaxed);
            tracing::debug!("[governor] Increased permits: {} → {}", current, target);
        } else {
            // Need fewer permits — forget excess (but don't panic)
            let current_available = self.inner.available_permits();
            if current_available > target {
                let to_forget = current_available - target;
                // Safety: only forget up to what's available minus target
                let safe_forget = to_forget.min(current_available.saturating_sub(target));
                if safe_forget > 0 {
                    self.inner.forget_permits(safe_forget);
                }
            }
            self.current_max.store(target, Ordering::Relaxed);
            tracing::debug!("[governor] Decreased permits: {} → {}", current, target);
        }
    }

    /// Get the monitor reference.
    pub fn monitor(&self) -> &Arc<ResourceMonitor> {
        &self.monitor
    }
}
