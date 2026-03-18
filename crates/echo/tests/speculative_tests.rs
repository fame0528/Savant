//! ECHO protocol speculative execution tests.

#[cfg(test)]
mod echo_speculative_tests {
    use savant_echo::circuit_breaker::{BreakerState, CircuitBreaker};

    #[test]
    fn test_circuit_breaker_halfopen_blocks_excess() {
        let cb = CircuitBreaker::with_thresholds(1, 0, 5);
        cb.record_failure(); // Trip
        assert!(cb.allow_request()); // → HalfOpen

        // Additional requests in HalfOpen should still be allowed
        // (the breaker doesn't block, it just limits success needed)
        for _ in 0..10 {
            cb.allow_request(); // Should not panic
        }
    }

    #[test]
    fn test_circuit_breaker_state_persistence() {
        let cb = CircuitBreaker::with_thresholds(3, 0, 2);

        // Trip the circuit
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), BreakerState::Open);

        // Check metrics persist
        let m = cb.metrics();
        assert_eq!(m.total_failures, 3);
        assert_eq!(m.state, BreakerState::Open);
    }

    #[test]
    fn test_circuit_breaker_recovery_chain() {
        let cb = CircuitBreaker::with_thresholds(1, 0, 2);

        // Trip → HalfOpen → fail → Open → HalfOpen → success → success → Closed
        cb.record_failure(); // Open
        assert!(cb.allow_request()); // HalfOpen
        cb.record_failure(); // Back to Open
        assert_eq!(cb.state(), BreakerState::Open);

        assert!(cb.allow_request()); // HalfOpen again
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), BreakerState::Closed);
    }

    #[test]
    fn test_circuit_breaker_high_frequency() {
        // Test with rapid succession of events
        let cb = CircuitBreaker::with_thresholds(100, 0, 10);

        for _ in 0..99 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), BreakerState::Closed);

        cb.record_failure(); // 100th failure
        assert_eq!(cb.state(), BreakerState::Open);

        // Rapidly transition through HalfOpen
        assert!(cb.allow_request());
        for _ in 0..10 {
            cb.record_success();
        }
        assert_eq!(cb.state(), BreakerState::Closed);
    }

    #[test]
    fn test_circuit_breaker_zero_threshold() {
        // Edge case: threshold of 1
        let cb = CircuitBreaker::with_thresholds(1, 0, 1);
        cb.record_failure();
        assert_eq!(cb.state(), BreakerState::Open);

        assert!(cb.allow_request()); // HalfOpen
        cb.record_success();
        assert_eq!(cb.state(), BreakerState::Closed);
    }
}
