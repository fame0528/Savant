use std::time::Duration;

/// Creates a secure `reqwest::Client` with timeout, connection pool, and redirect limits.
///
/// All production HTTP calls should use this instead of `reqwest::Client::new()`.
#[allow(clippy::disallowed_methods)]
pub fn secure_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .connect_timeout(Duration::from_secs(5))
        .pool_max_idle_per_host(4)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .expect(
            "CRITICAL: Failed to build secure HTTP client with timeout and redirect constraints",
        )
}
