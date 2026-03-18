//! Integration tests for MCP server authentication and circuit breaker.

#[cfg(test)]
mod mcp_tests {
    use savant_mcp::circuit::CircuitBreaker;

    #[test]
    fn test_circuit_starts_closed() {
        let cb = CircuitBreaker::new();
        assert!(cb.allow_request());
    }

    #[test]
    fn test_circuit_opens_after_failures() {
        let cb = CircuitBreaker::with_thresholds(3, 10, 2);
        assert!(cb.allow_request());
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), savant_mcp::circuit::BreakerState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), savant_mcp::circuit::BreakerState::Open);
        assert!(!cb.allow_request());
    }

    #[test]
    fn test_circuit_halfopen_after_timeout() {
        let cb = CircuitBreaker::with_thresholds(1, 0, 1);
        cb.record_failure();
        assert_eq!(cb.state(), savant_mcp::circuit::BreakerState::Open);
        // recovery_timeout = 0 means immediate transition
        assert!(cb.allow_request());
        assert_eq!(cb.state(), savant_mcp::circuit::BreakerState::HalfOpen);
    }

    #[test]
    fn test_circuit_closes_after_successes() {
        let cb = CircuitBreaker::with_thresholds(1, 0, 2);
        cb.record_failure();
        assert!(cb.allow_request()); // → HalfOpen
        cb.record_success();
        assert_eq!(cb.state(), savant_mcp::circuit::BreakerState::HalfOpen);
        cb.record_success();
        assert_eq!(cb.state(), savant_mcp::circuit::BreakerState::Closed);
    }

    #[test]
    fn test_circuit_resets() {
        let cb = CircuitBreaker::with_thresholds(1, 0, 1);
        cb.record_failure();
        assert_eq!(cb.state(), savant_mcp::circuit::BreakerState::Open);
        cb.reset();
        assert_eq!(cb.state(), savant_mcp::circuit::BreakerState::Closed);
        assert!(cb.allow_request());
    }

    #[test]
    fn test_concurrent_failures() {
        use std::sync::Arc;
        use std::thread;

        let cb = Arc::new(CircuitBreaker::with_thresholds(10, 60, 5));
        let mut handles = vec![];

        for _ in 0..20 {
            let cb_clone = cb.clone();
            handles.push(thread::spawn(move || {
                cb_clone.record_failure();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // With 20 failures and threshold=10, circuit should be open
        assert_eq!(cb.state(), savant_mcp::circuit::BreakerState::Open);
    }
}
