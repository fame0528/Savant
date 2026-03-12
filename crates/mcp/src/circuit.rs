/// The core state sequence points underlying a circuit breaker
pub enum BreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Provides localized fault isolation capabilities when wrapping unreliable clients.
pub struct CircuitBreaker {
    pub state: BreakerState,
}

impl CircuitBreaker {
    #[must_use]
    pub fn new() -> Self {
        Self { state: BreakerState::Closed }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod benches {
    // criterion benchmark stub
}
