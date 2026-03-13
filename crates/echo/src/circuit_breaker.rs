//! Statistical Circuit Breaker for Autonomous Upgrades
//!
//! Monitors the failure rate of newly hot-swapped components and triggers
//! rollbacks if mathematical thresholds are exceeded.

use std::sync::atomic::{AtomicU64, Ordering};

/// Metrics for a specific component or epoch.
pub struct ComponentMetrics {
    total_invocations: AtomicU64,
    failed_invocations: AtomicU64,
    /// Threshold error rate (e.g., 5% = 0.05)
    error_threshold: f64,
    /// Minimum invocations before the breaker is allowed to trip
    min_sample_size: u64,
}

impl ComponentMetrics {
    /// Creates a new metrics tracker with internal thresholding.
    pub fn new(error_threshold: f64, min_sample_size: u64) -> Self {
        Self {
            total_invocations: AtomicU64::new(0),
            failed_invocations: AtomicU64::new(0),
            error_threshold,
            min_sample_size,
        }
    }

    /// Records an execution outcome. 
    ///
    /// Returns `true` if the circuit breaker should trip.
    pub fn record_outcome(&self, success: bool) -> bool {
        let total = self.total_invocations.fetch_add(1, Ordering::Relaxed) + 1;
        
        let failed = if !success {
            self.failed_invocations.fetch_add(1, Ordering::Relaxed) + 1
        } else {
            self.failed_invocations.load(Ordering::Relaxed)
        };

        // Do not calculate statistics until we have a statistically significant sample
        if total < self.min_sample_size {
            return false;
        }

        // Calculate current error rate
        let current_error_rate = (failed as f64) / (total as f64);

        // Trip the breaker if the error rate exceeds the threshold
        current_error_rate > self.error_threshold
    }

    /// Returns current failure count.
    pub fn failure_count(&self) -> u64 {
        self.failed_invocations.load(Ordering::Relaxed)
    }

    /// Returns current total count.
    pub fn total_count(&self) -> u64 {
        self.total_invocations.load(Ordering::Relaxed)
    }
}
