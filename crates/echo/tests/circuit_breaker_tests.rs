//! ECHO protocol tests - circuit breaker state transitions.

#[cfg(test)]
mod echo_tests {
    use savant_echo::circuit_breaker::{BreakerState, CircuitBreaker};

    #[test]
    fn test_circuit_breaker_full_lifecycle() {
        let cb = CircuitBreaker::with_thresholds(3, 0, 2);

        // Start closed
        assert_eq!(cb.state(), BreakerState::Closed);

        // Accumulate failures
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), BreakerState::Closed);

        // Third failure trips the circuit
        cb.record_failure();
        assert_eq!(cb.state(), BreakerState::Open);

        // Open blocks requests until timeout
        // recovery_timeout=0 means immediate transition to HalfOpen
        assert!(cb.allow_request());
        assert_eq!(cb.state(), BreakerState::HalfOpen);

        // HalfOpen: success count accumulates
        cb.record_success();
        assert_eq!(cb.state(), BreakerState::HalfOpen);

        // Second success closes the circuit
        cb.record_success();
        assert_eq!(cb.state(), BreakerState::Closed);
    }

    #[test]
    fn test_halfopen_failure_reopens() {
        let cb = CircuitBreaker::with_thresholds(1, 0, 3);
        cb.record_failure(); // Trip to Open
        assert!(cb.allow_request()); // → HalfOpen (timeout=0)

        // Failure in HalfOpen immediately re-opens
        cb.record_failure();
        assert_eq!(cb.state(), BreakerState::Open);
    }

    #[test]
    fn test_reset_clears_state() {
        let cb = CircuitBreaker::with_thresholds(1, 60, 1);
        cb.record_failure();
        assert_eq!(cb.state(), BreakerState::Open);

        cb.reset();
        assert_eq!(cb.state(), BreakerState::Closed);
        assert!(cb.allow_request());
    }

    #[test]
    fn test_concurrent_operations() {
        use std::sync::Arc;
        use std::thread;

        let cb = Arc::new(CircuitBreaker::with_thresholds(50, 60, 10));
        let mut handles = vec![];

        // Spawn 100 threads recording failures
        for _ in 0..100 {
            let cb_clone = cb.clone();
            handles.push(thread::spawn(move || {
                cb_clone.record_failure();
            }));
        }

        // Spawn 50 threads recording successes
        for _ in 0..50 {
            let cb_clone = cb.clone();
            handles.push(thread::spawn(move || {
                cb_clone.record_success();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // With 50+ failures and threshold=50, circuit should be open
        assert_eq!(cb.state(), BreakerState::Open);
    }

    #[test]
    fn test_metrics_consistency() {
        let cb = CircuitBreaker::with_thresholds(10, 0, 5);

        for _ in 0..7 {
            cb.record_failure();
        }

        for _ in 0..3 {
            cb.record_success();
        }

        let metrics = cb.metrics();
        assert_eq!(metrics.total_failures, 7);
        assert_eq!(metrics.total_successes, 3);
        assert_eq!(metrics.trip_count, 0); // threshold=10, only 7 failures
    }
}
