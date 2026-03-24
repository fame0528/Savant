/// Reliable time utilities — loud failure on clock errors.
///
/// The system clock should NEVER return a time before Unix epoch.
/// If it does, the system is misconfigured and we fail immediately
/// rather than silently treating the error as epoch 0.
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current time as seconds since Unix epoch.
/// Fails loudly if the system clock is before Unix epoch.
/// This replaces all `unwrap_or_default()` patterns that silently return 0.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock error: time is before Unix epoch. System is misconfigured.")
        .as_secs()
}

/// Returns the current time as milliseconds since Unix epoch.
pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock error: time is before Unix epoch. System is misconfigured.")
        .as_millis() as u64
}

/// Returns the current time as nanoseconds since Unix epoch.
pub fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock error: time is before Unix epoch. System is misconfigured.")
        .as_nanos()
}
