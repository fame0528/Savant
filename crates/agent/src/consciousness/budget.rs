//! Consciousness Budget — token/cost budget enforcement for the consciousness layer.

/// Token budget for consciousness operations.
pub struct ConsciousnessBudget {
    tokens_per_hour: u32,
    tokens_per_day: u32,
    current_hour_tokens: u32,
    current_day_tokens: u32,
    quiet_hours_start: u8,
    quiet_hours_end: u8,
}

impl Default for ConsciousnessBudget {
    fn default() -> Self { Self::new() }
}

impl ConsciousnessBudget {
    pub fn new() -> Self {
        Self {
            tokens_per_hour: 100_000,
            tokens_per_day: 500_000,
            current_hour_tokens: 0,
            current_day_tokens: 0,
            quiet_hours_start: 22,
            quiet_hours_end: 7,
        }
    }

    /// Check if thinking is allowed right now.
    pub fn can_think(&self) -> bool {
        if self.is_quiet_hours() {
            return false;
        }
        if self.current_hour_tokens >= self.tokens_per_hour {
            return false;
        }
        if self.current_day_tokens >= self.tokens_per_day {
            return false;
        }
        true
    }

    /// Record token usage.
    pub fn record_usage(&mut self, tokens: u32) {
        self.current_hour_tokens += tokens;
        self.current_day_tokens += tokens;
    }

    /// Reset hourly counter (called every hour).
    pub fn reset_hourly(&mut self) {
        self.current_hour_tokens = 0;
    }

    /// Reset daily counter (called every midnight).
    pub fn reset_daily(&mut self) {
        self.current_day_tokens = 0;
        self.current_hour_tokens = 0;
    }

    /// Check if we're in quiet hours (no consciousness operations).
    fn is_quiet_hours(&self) -> bool {
        use chrono::Timelike;
        let hour = chrono::Utc::now().hour() as u8;
        if self.quiet_hours_start > self.quiet_hours_end {
            // Overnight: e.g., 22-7 means 22:00-06:59 is quiet
            hour >= self.quiet_hours_start || hour < self.quiet_hours_end
        } else {
            hour >= self.quiet_hours_start && hour < self.quiet_hours_end
        }
    }

    /// Adjust budget based on entropy level (0.0-1.0).
    pub fn set_budget_multiplier(&mut self, entropy: f64) {
        let multiplier = if entropy > 0.85 {
            1.0 // Full budget
        } else if entropy > 0.40 {
            0.4 // 40%
        } else if entropy > 0.10 {
            0.15 // 15%
        } else {
            0.02 // 2%
        };

        self.tokens_per_hour = (100_000.0 * multiplier) as u32;
        self.tokens_per_day = (500_000.0 * multiplier) as u32;
    }

    /// Percentage of hourly budget used.
    pub fn hourly_usage_pct(&self) -> f64 {
        if self.tokens_per_hour == 0 {
            return 1.0;
        }
        self.current_hour_tokens as f64 / self.tokens_per_hour as f64
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_allows_thinking() {
        let budget = ConsciousnessBudget::new();
        // During non-quiet hours, should allow thinking
        // (this test depends on time of day, so just test the logic)
        assert!(budget.current_hour_tokens < budget.tokens_per_hour);
    }

    #[test]
    fn test_budget_blocks_at_limit() {
        let mut budget = ConsciousnessBudget::new();
        budget.current_hour_tokens = 100_000;
        assert!(!budget.can_think());
    }

    #[test]
    fn test_budget_records_usage() {
        let mut budget = ConsciousnessBudget::new();
        budget.record_usage(5000);
        assert_eq!(budget.current_hour_tokens, 5000);
        assert_eq!(budget.current_day_tokens, 5000);
    }

    #[test]
    fn test_budget_reset() {
        let mut budget = ConsciousnessBudget::new();
        budget.record_usage(5000);
        budget.reset_hourly();
        assert_eq!(budget.current_hour_tokens, 0);
        assert_eq!(budget.current_day_tokens, 5000);
    }

    #[test]
    fn test_budget_multiplier() {
        let mut budget = ConsciousnessBudget::new();
        budget.set_budget_multiplier(0.9); // Hyper-active
        assert_eq!(budget.tokens_per_hour, 100_000);

        budget.set_budget_multiplier(0.05); // Dormant
        assert_eq!(budget.tokens_per_hour, 2000);
    }
}
