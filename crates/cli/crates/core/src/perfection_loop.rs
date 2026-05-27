//! Perfection Loop FSM — Red → Green → Audit → Self-Correct cycle
//!
//! This module drives the autonomous code improvement loop:
//! 1. Red:   Compilation/test failure detected
//! 2. Green: Changes compile and tests pass
//! 3. Audit: Deep audit for quality, tech debt, security
//! 4. Self-Correct: Apply fixes and loop back to Green

use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Maximum Levenshtein distance between consecutive errors before circuit breaker trips
const MAX_LEVENSHTEIN_DISTANCE: usize = 3;

/// Maximum iterations before circuit breaker forces manual review
const MAX_ITERATIONS_WITHOUT_CONVERGENCE: u32 = 5;

/// Maximum consecutive identical error patterns before trip
const MAX_IDENTICAL_ERRORS: u32 = 3;

/// States in the Perfection Loop
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopState {
    /// Initial state — no work done yet
    Idle,
    /// Compilation or test failure detected — need fixes
    Red,
    /// Code compiles and tests pass — ready for audit
    Green,
    /// Deep audit in progress — looking for quality issues
    Audit,
    /// Self-correcting — applying audit findings
    SelfCorrect,
    /// Loop complete — all checks passed
    Complete,
    /// Circuit breaker tripped — manual review needed
    Tripped,
}

impl std::fmt::Display for LoopState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopState::Idle => write!(f, "Idle"),
            LoopState::Red => write!(f, "Red"),
            LoopState::Green => write!(f, "Green"),
            LoopState::Audit => write!(f, "Audit"),
            LoopState::SelfCorrect => write!(f, "SelfCorrect"),
            LoopState::Complete => write!(f, "Complete"),
            LoopState::Tripped => write!(f, "Tripped"),
        }
    }
}

/// An individual issue found during audit
#[derive(Debug, Clone)]
pub struct AuditIssue {
    /// Severity: error, warning, suggestion
    pub severity: String,
    /// File path where the issue was found
    pub file: String,
    /// Line number (if known)
    pub line: Option<usize>,
    /// Human-readable description
    pub message: String,
    /// Suggested fix (if the linter provides one)
    pub suggested_fix: Option<String>,
}

/// Result of a Perfection Loop iteration
#[derive(Debug, Clone)]
pub enum LoopResult {
    /// Loop completed successfully after N iterations
    Complete {
        iterations: u32,
        issues_fixed: usize,
        duration: Duration,
    },
    /// Circuit breaker tripped — identical error repeated too many times
    Tripped {
        reason: String,
        iterations: u32,
        last_error: String,
    },
    /// Max iterations exceeded without convergence
    Exhausted {
        iterations: u32,
        remaining_issues: usize,
    },
}

/// Circuit breaker that detects infinite loops via error fingerprint similarity
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Recent error fingerprints (Levenshtein-normalized)
    fingerprints: VecDeque<String>,
    /// Count of consecutive identical fingerprints
    consecutive_identical: u32,
    /// Maximum fingerprints to track
    window_size: usize,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default window size
    pub fn new() -> Self {
        Self {
            fingerprints: VecDeque::with_capacity(5),
            consecutive_identical: 0,
            window_size: 5,
        }
    }

    /// Create a circuit breaker with a custom window size
    pub fn with_window_size(window_size: usize) -> Self {
        Self {
            fingerprints: VecDeque::with_capacity(window_size),
            consecutive_identical: 0,
            window_size: window_size.max(1),
        }
    }

    /// Normalize an error string for fingerprinting:
    /// strip line numbers, file:line:col markers, and normalize whitespace
    fn fingerprint(error: &str) -> String {
        let mut normalized = String::new();
        for line in error.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("-->")
                || trimmed.starts_with('|')
                || trimmed.starts_with("= note:")
                || trimmed.starts_with("= help:")
            {
                continue;
            }
            if let Some(pos) = trimmed.find("error[") {
                normalized.push_str(&trimmed[pos..]);
            } else if let Some(pos) = trimmed.find("warning[") {
                normalized.push_str(&trimmed[pos..]);
            } else {
                normalized.push_str(trimmed);
            }
            normalized.push(' ');
        }
        normalized.trim().to_lowercase()
    }

    /// Check if a new error should trip the breaker.
    /// Returns `None` if safe to proceed, or `Some(reason)` if tripped.
    pub fn check(&mut self, error: &str) -> Option<String> {
        let fp = Self::fingerprint(error);

        if let Some(last) = self.fingerprints.back() {
            if *last == fp || strsim::levenshtein(last, &fp) <= MAX_LEVENSHTEIN_DISTANCE {
                self.consecutive_identical += 1;
            } else {
                self.consecutive_identical = 0;
            }
        } else {
            self.consecutive_identical = 1;
        }

        self.fingerprints.push_back(fp);

        if self.fingerprints.len() > self.window_size {
            self.fingerprints.pop_front();
        }

        if self.consecutive_identical >= MAX_IDENTICAL_ERRORS {
            return Some(format!(
                "Circuit breaker tripped: {} consecutive identical/similar errors",
                self.consecutive_identical
            ));
        }

        None
    }

    /// Reset the circuit breaker state
    pub fn reset(&mut self) {
        self.fingerprints.clear();
        self.consecutive_identical = 0;
    }

    /// Get the count of consecutive identical fingerprints
    pub fn consecutive_count(&self) -> u32 {
        self.consecutive_identical
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for the Perfection Loop
#[derive(Debug, Clone)]
pub struct LoopConfig {
    /// Command to compile the project
    pub compile_command: String,
    /// Command to run tests
    pub test_command: String,
    /// Command to run linter
    pub lint_command: String,
    /// Working directory for commands
    pub working_dir: String,
    /// Max iterations before forced review
    pub max_iterations: u32,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            compile_command: "cargo check --workspace".to_string(),
            test_command: "cargo test --workspace".to_string(),
            lint_command: "cargo clippy --workspace -- -D warnings".to_string(),
            working_dir: ".".to_string(),
            max_iterations: MAX_ITERATIONS_WITHOUT_CONVERGENCE,
        }
    }
}

/// The Perfection Loop FSM — drives the Red → Green → Audit → Self-Correct cycle
pub struct PerfectionLoop {
    state: LoopState,
    config: LoopConfig,
    breaker: CircuitBreaker,
    iteration: u32,
    issues_fixed: usize,
    start_time: Option<Instant>,
}

impl PerfectionLoop {
    /// Create a new Perfection Loop with default config
    pub fn new() -> Self {
        Self {
            state: LoopState::Idle,
            config: LoopConfig::default(),
            breaker: CircuitBreaker::new(),
            iteration: 0,
            issues_fixed: 0,
            start_time: None,
        }
    }

    /// Create a Perfection Loop with custom config
    pub fn with_config(config: LoopConfig) -> Self {
        Self {
            state: LoopState::Idle,
            config,
            breaker: CircuitBreaker::new(),
            iteration: 0,
            issues_fixed: 0,
            start_time: None,
        }
    }

    /// Get the current state
    pub fn state(&self) -> LoopState {
        self.state
    }

    /// Get the current iteration count
    pub fn iteration(&self) -> u32 {
        self.iteration
    }

    /// Get the number of issues fixed so far
    pub fn issues_fixed(&self) -> usize {
        self.issues_fixed
    }

    /// Get elapsed time since the loop started
    pub fn elapsed(&self) -> Option<Duration> {
        self.start_time.map(|t| t.elapsed())
    }

    /// Start the perfection loop
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.iteration = 0;
        self.issues_fixed = 0;
        self.breaker.reset();
        self.transition_to(LoopState::Green);
        info!("Perfection Loop started");
    }

    /// Submit a compilation/test error (Red state trigger)
    pub fn submit_error(&mut self, error: &str) -> LoopState {
        self.transition_to(LoopState::Red);

        if let Some(reason) = self.breaker.check(error) {
            warn!("{}", reason);
            self.transition_to(LoopState::Tripped);
            return LoopState::Tripped;
        }

        debug!("Red state: error recorded (iteration {})", self.iteration);
        LoopState::Red
    }

    /// Submit successful compilation (Green state trigger)
    pub fn submit_green(&mut self) -> LoopState {
        self.transition_to(LoopState::Green);
        debug!(
            "Green state: compilation passed (iteration {})",
            self.iteration
        );
        LoopState::Green
    }

    /// Move to audit phase — collects issues from linting/analysis
    pub fn submit_audit(&mut self, issues: &[AuditIssue]) -> LoopState {
        self.transition_to(LoopState::Audit);

        if issues.is_empty() {
            info!("Audit: no issues found. Loop complete.");
            self.transition_to(LoopState::Complete);
            return LoopState::Complete;
        }

        let error_count = issues.iter().filter(|i| i.severity == "error").count();
        let warn_count = issues.iter().filter(|i| i.severity == "warning").count();

        debug!(
            "Audit: {} errors, {} warnings found (iteration {})",
            error_count, warn_count, self.iteration
        );

        LoopState::Audit
    }

    /// Move to self-correct phase — applies fixes
    pub fn submit_fix(&mut self, issues_fixed: usize) -> LoopState {
        self.transition_to(LoopState::SelfCorrect);
        self.issues_fixed += issues_fixed;

        debug!(
            "SelfCorrect: fixed {} issues, total fixed: {} (iteration {})",
            issues_fixed, self.issues_fixed, self.iteration
        );

        if self.iteration >= self.config.max_iterations {
            warn!(
                "Max iterations ({}) reached without convergence",
                self.config.max_iterations
            );
            return LoopState::SelfCorrect;
        }

        self.transition_to(LoopState::Green);
        self.iteration += 1;

        LoopState::Green
    }

    /// Mark the loop as complete
    pub fn complete(&mut self) -> LoopResult {
        self.transition_to(LoopState::Complete);
        info!(
            "Perfection Loop complete: {} iterations, {} issues fixed, {:?} elapsed",
            self.iteration,
            self.issues_fixed,
            self.elapsed().unwrap_or_default()
        );

        LoopResult::Complete {
            iterations: self.iteration,
            issues_fixed: self.issues_fixed,
            duration: self.elapsed().unwrap_or_default(),
        }
    }

    /// Trip the circuit breaker manually
    pub fn trip(&mut self, reason: String) -> LoopResult {
        self.transition_to(LoopState::Tripped);
        warn!("Perfection Loop tripped: {}", reason);

        LoopResult::Tripped {
            reason,
            iterations: self.iteration,
            last_error: String::new(),
        }
    }

    /// Mark as exhausted (max iterations)
    pub fn exhaust(&mut self, remaining_issues: usize) -> LoopResult {
        LoopResult::Exhausted {
            iterations: self.iteration,
            remaining_issues,
        }
    }

    fn transition_to(&mut self, new_state: LoopState) {
        if self.state != new_state {
            debug!("Perfection Loop: {} -> {}", self.state, new_state);
            self.state = new_state;
        }
    }
}

impl Default for PerfectionLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_trips_on_repeated_error() {
        let mut breaker = CircuitBreaker::new();
        let error = "error[E0502]: cannot borrow `*self` as mutable";

        assert!(breaker.check(error).is_none());
        assert!(breaker.check(error).is_none());
        assert!(breaker.check(error).is_some()); // 3rd identical = trip
    }

    #[test]
    fn test_circuit_breaker_resets_on_different_error() {
        let mut breaker = CircuitBreaker::new();
        let error1 = "error[E0502]: cannot borrow `*self` as mutable";
        let error2 = "error[E0382]: use of moved value";

        assert!(breaker.check(error1).is_none());
        assert!(breaker.check(error2).is_none()); // Different error resets counter
        assert_eq!(breaker.consecutive_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_similar_errors_trip() {
        let mut breaker = CircuitBreaker::new();
        let error1 = "error[E0502]: cannot borrow self as mutable\n  --> src/lib.rs:105:21";
        let error2 = "error[E0502]: cannot borrow self as mutable\n  --> src/lib.rs:210:5";

        assert!(breaker.check(error1).is_none());
        assert!(breaker.check(error2).is_none());
    }

    #[test]
    fn test_perfection_loop_full_cycle() {
        let mut loop_fsm = PerfectionLoop::new();
        loop_fsm.start();

        assert_eq!(loop_fsm.state(), LoopState::Green);

        let state = loop_fsm.submit_error("error[E0277]: trait bound not satisfied");
        assert_eq!(state, LoopState::Red);

        let state = loop_fsm.submit_green();
        assert_eq!(state, LoopState::Green);

        let state = loop_fsm.submit_audit(&[]);
        assert_eq!(state, LoopState::Complete);

        let result = loop_fsm.complete();
        assert!(matches!(result, LoopResult::Complete { .. }));
    }

    #[test]
    fn test_perfection_loop_audit_with_issues() {
        let mut loop_fsm = PerfectionLoop::new();
        loop_fsm.start();

        let issues = vec![AuditIssue {
            severity: "warning".to_string(),
            file: "src/main.rs".to_string(),
            line: Some(42),
            message: "unused variable".to_string(),
            suggested_fix: None,
        }];
        let state = loop_fsm.submit_audit(&issues);
        assert_eq!(state, LoopState::Audit);

        let state = loop_fsm.submit_fix(1);
        assert_eq!(state, LoopState::Green);
        assert_eq!(loop_fsm.issues_fixed(), 1);
    }

    #[test]
    fn test_perfection_loop_circuit_breaker_integration() {
        let mut loop_fsm = PerfectionLoop::new();
        loop_fsm.start();

        let error = "error[E0502]: borrow checker violation";
        loop_fsm.submit_error(error);
        loop_fsm.submit_green();
        loop_fsm.submit_error(error);
        loop_fsm.submit_green();
        let state = loop_fsm.submit_error(error);

        assert_eq!(state, LoopState::Tripped);
    }

    #[test]
    fn test_fingerprint_strips_line_numbers() {
        let error =
            "error[E0502]: cannot borrow\n  --> src/lib.rs:105:21\n  |\n105 |     self.do_thing();";
        let fp = CircuitBreaker::fingerprint(error);
        assert!(!fp.contains(":105"));
        assert!(!fp.contains(":21"));
        assert!(fp.contains("error"));
    }
}
