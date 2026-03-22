//! Provider Chain — Error classification, cooldown, circuit breaker, response cache.
//!
//! Wraps any `LlmProvider` with 4 layers of resilience:
//! 1. **Error Classifier** — categorizes errors for intelligent retry decisions
//! 2. **Cooldown Tracker** — exponential backoff per provider (prevents thundering herd)
//! 3. **Circuit Breaker** — stops hitting dead providers after N consecutive failures
//! 4. **Response Cache** — deduplicates identical queries (saves money and latency)

use savant_core::error::SavantError;
use savant_core::traits::LlmProvider;
use savant_core::types::{ChatChunk, ChatMessage};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use futures::stream::{Stream, StreamExt};
use sha2::{Digest, Sha256};

// ============================================================================
// 1. Error Classifier
// ============================================================================

/// Categorized error types for intelligent retry decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// 401, 403 — credentials invalid or expired
    Auth,
    /// 429 — rate limit exceeded
    RateLimit,
    /// Payment/billing issues
    Billing,
    /// Request timeout or connection timeout
    Timeout,
    /// 400 — malformed request
    Format,
    /// 500, 502, 503, 504 — server overloaded
    Overloaded,
    /// Network errors, transient failures
    Transient,
}

/// Classifies a SavantError into an ErrorCategory.
pub fn classify_error(error: &SavantError) -> ErrorCategory {
    match error {
        SavantError::AuthError(msg) => {
            let lower = msg.to_lowercase();
            if lower.contains("429") || lower.contains("rate limit") || lower.contains("ratelimit")
            {
                ErrorCategory::RateLimit
            } else if lower.contains("401")
                || lower.contains("403")
                || lower.contains("unauthorized")
                || lower.contains("forbidden")
            {
                ErrorCategory::Auth
            } else if lower.contains("billing")
                || lower.contains("payment")
                || lower.contains("quota")
                || lower.contains("credit")
            {
                ErrorCategory::Billing
            } else if lower.contains("500")
                || lower.contains("502")
                || lower.contains("503")
                || lower.contains("504")
                || lower.contains("server error")
                || lower.contains("overloaded")
            {
                ErrorCategory::Overloaded
            } else if lower.contains("400")
                || lower.contains("bad request")
                || lower.contains("invalid")
            {
                ErrorCategory::Format
            } else if lower.contains("timeout") {
                ErrorCategory::Timeout
            } else {
                ErrorCategory::Transient
            }
        }
        SavantError::IoError(_) => ErrorCategory::Transient,
        SavantError::Unknown(msg) => {
            let lower = msg.to_lowercase();
            if lower.contains("timeout") {
                ErrorCategory::Timeout
            } else if lower.contains("429") {
                ErrorCategory::RateLimit
            } else if lower.contains("500") || lower.contains("502") || lower.contains("503") {
                ErrorCategory::Overloaded
            } else {
                ErrorCategory::Transient
            }
        }
        _ => ErrorCategory::Transient,
    }
}

// ============================================================================
// 2. Cooldown Tracker
// ============================================================================

/// Tracks per-key cooldown with exponential backoff.
struct CooldownState {
    failure_count: u32,
    cooldown_start: Option<Instant>,
    resume_at: Option<Instant>,
}

impl Default for CooldownState {
    fn default() -> Self {
        Self {
            failure_count: 0,
            cooldown_start: None,
            resume_at: None,
        }
    }
}

/// Helper: recover from poisoned RwLock.
macro_rules! read_lock {
    ($lock:expr) => {
        $lock.read().unwrap_or_else(|e| e.into_inner())
    };
}

/// Helper: recover from poisoned RwLock (write).
macro_rules! write_lock {
    ($lock:expr) => {
        $lock.write().unwrap_or_else(|e| e.into_inner())
    };
}

/// Per-key cooldown tracker with exponential backoff.
pub struct CooldownTracker {
    states: RwLock<HashMap<String, CooldownState>>,
}

impl CooldownTracker {
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
        }
    }

    /// Check if a key is currently on cooldown. Returns false if not.
    pub fn is_on_cooldown(&self, key: &str) -> bool {
        let states = read_lock!(self.states);
        if let Some(state) = states.get(key) {
            if let Some(resume_at) = state.resume_at {
                return Instant::now() < resume_at;
            }
        }
        false
    }

    /// Record a failure and compute the cooldown duration.
    pub fn record_failure(&self, key: &str, category: ErrorCategory) {
        let mut states = write_lock!(self.states);
        let state = states.entry(key.to_string()).or_default();
        state.failure_count += 1;
        state.cooldown_start = Some(Instant::now());

        let duration = match category {
            ErrorCategory::Billing => Self::billing_cooldown(state.failure_count),
            ErrorCategory::RateLimit => Self::standard_cooldown(state.failure_count),
            ErrorCategory::Overloaded => Self::standard_cooldown(state.failure_count),
            ErrorCategory::Auth => Duration::from_secs(300),
            _ => Duration::from_secs(30),
        };

        state.resume_at = Some(Instant::now() + duration);

        tracing::warn!(
            "Cooldown: {} for {:?} (failures={}, duration={}s)",
            key,
            category,
            state.failure_count,
            duration.as_secs()
        );
    }

    /// Record a success — resets the failure counter.
    pub fn record_success(&self, key: &str) {
        let mut states = write_lock!(self.states);
        if let Some(state) = states.get_mut(key) {
            state.failure_count = 0;
            state.cooldown_start = None;
            state.resume_at = None;
        }
    }

    /// Standard exponential backoff: min(1h, 1min * 5^min(n-1, 3))
    fn standard_cooldown(n: u32) -> Duration {
        let exponent = n.saturating_sub(1).min(3);
        let multiplier = 5u64.pow(exponent);
        let seconds = 60 * multiplier;
        Duration::from_secs(seconds.min(3600))
    }

    /// Billing exponential backoff: min(24h, 5h * 2^min(n-1, 10))
    fn billing_cooldown(n: u32) -> Duration {
        let exponent = n.saturating_sub(1).min(10);
        let multiplier = 2u64.pow(exponent);
        let seconds = 5 * 3600 * multiplier;
        Duration::from_secs(seconds.min(86400))
    }
}

// ============================================================================
// 3. Circuit Breaker
// ============================================================================

/// Circuit breaker state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker that stops hitting dead providers.
pub struct CircuitBreaker {
    state: RwLock<BreakerState>,
    failure_count: RwLock<u32>,
    failure_threshold: u32,
    open_duration: Duration,
    last_opened: RwLock<Option<Instant>>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, open_duration: Duration) -> Self {
        Self {
            state: RwLock::new(BreakerState::Closed),
            failure_count: RwLock::new(0),
            failure_threshold,
            open_duration,
            last_opened: RwLock::new(None),
        }
    }

    /// Check if a request is allowed through the breaker.
    pub fn is_allowed(&self) -> bool {
        let state = read_lock!(self.state);
        match *state {
            BreakerState::Closed => true,
            BreakerState::HalfOpen => true,
            BreakerState::Open => {
                if let Some(opened) = *read_lock!(self.last_opened) {
                    if opened.elapsed() >= self.open_duration {
                        drop(state);
                        *write_lock!(self.state) = BreakerState::HalfOpen;
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Record a successful call.
    pub fn record_success(&self) {
        let mut count = write_lock!(self.failure_count);
        *count = 0;

        let mut state = write_lock!(self.state);
        if *state == BreakerState::HalfOpen {
            tracing::info!("Circuit breaker: recovered (HalfOpen → Closed)");
        }
        *state = BreakerState::Closed;
    }

    /// Record a failed call.
    pub fn record_failure(&self) {
        let mut count = write_lock!(self.failure_count);
        *count += 1;

        let mut state = write_lock!(self.state);
        match *state {
            BreakerState::Closed => {
                if *count >= self.failure_threshold {
                    *state = BreakerState::Open;
                    *write_lock!(self.last_opened) = Some(Instant::now());
                    tracing::warn!("Circuit breaker: OPEN after {} consecutive failures", count);
                }
            }
            BreakerState::HalfOpen => {
                *state = BreakerState::Open;
                *write_lock!(self.last_opened) = Some(Instant::now());
                tracing::warn!("Circuit breaker: probe failed, reopening");
            }
            BreakerState::Open => {}
        }
    }

    /// Get current state.
    pub fn current_state(&self) -> BreakerState {
        *read_lock!(self.state)
    }
}

// ============================================================================
// 4. Response Cache
// ============================================================================

struct CacheEntry {
    chunks: Vec<ChatChunk>,
    inserted_at: Instant,
}

/// SHA-256 keyed LRU response cache.
pub struct ResponseCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
    ttl: Duration,
    max_size: usize,
}

impl ResponseCache {
    pub fn new(ttl: Duration, max_size: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::with_capacity(max_size)),
            ttl,
            max_size,
        }
    }

    /// Generate a cache key from messages (SHA-256 of content).
    fn cache_key(messages: &[ChatMessage]) -> String {
        let mut hasher = Sha256::new();
        for msg in messages {
            hasher.update(format!("{:?}:{}", msg.role, msg.content).as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    /// Try to get a cached response. Returns None on miss or expiry.
    pub fn get(&self, messages: &[ChatMessage]) -> Option<Vec<ChatChunk>> {
        let key = Self::cache_key(messages);
        let entries = read_lock!(self.entries);
        if let Some(entry) = entries.get(&key) {
            if entry.inserted_at.elapsed() < self.ttl {
                tracing::debug!("Cache hit for key {}", &key[..8.min(key.len())]);
                return Some(entry.chunks.clone());
            }
        }
        None
    }

    /// Store a response in the cache. Skips if response contains tool calls.
    pub fn put(&self, messages: &[ChatMessage], chunks: &[ChatChunk]) {
        let has_tool_calls = chunks.iter().any(|c| c.tool_calls.is_some());
        if has_tool_calls {
            return;
        }

        let key = Self::cache_key(messages);
        let mut entries = write_lock!(self.entries);

        // Evict oldest if at capacity
        if entries.len() >= self.max_size {
            if let Some(oldest_key) = entries
                .iter()
                .min_by_key(|(_, e)| e.inserted_at)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&oldest_key);
            }
        }

        entries.insert(
            key,
            CacheEntry {
                chunks: chunks.to_vec(),
                inserted_at: Instant::now(),
            },
        );
    }
}

// ============================================================================
// 5. Provider Chain — combines all 4 layers
// ============================================================================

/// Configuration for the provider chain.
pub struct ChainConfig {
    pub max_retries: u32,
    pub failure_threshold: u32,
    pub open_duration: Duration,
    pub cache_ttl: Duration,
    pub cache_max_size: usize,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            failure_threshold: 5,
            open_duration: Duration::from_secs(60),
            cache_ttl: Duration::from_secs(300),
            cache_max_size: 256,
        }
    }
}

/// Provider chain combining error classification, cooldown, circuit breaker, and response cache.
pub struct ProviderChain {
    inner: Box<dyn LlmProvider>,
    cooldown: CooldownTracker,
    breaker: CircuitBreaker,
    cache: ResponseCache,
    max_retries: u32,
    chain_key: String,
}

impl ProviderChain {
    pub fn new(inner: Box<dyn LlmProvider>, chain_key: String, config: ChainConfig) -> Self {
        Self {
            inner,
            cooldown: CooldownTracker::new(),
            breaker: CircuitBreaker::new(config.failure_threshold, config.open_duration),
            cache: ResponseCache::new(config.cache_ttl, config.cache_max_size),
            max_retries: config.max_retries,
            chain_key,
        }
    }

    fn is_retryable(category: ErrorCategory) -> bool {
        matches!(
            category,
            ErrorCategory::RateLimit
                | ErrorCategory::Overloaded
                | ErrorCategory::Timeout
                | ErrorCategory::Transient
        )
    }
}

#[async_trait::async_trait]
impl LlmProvider for ProviderChain {
    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<serde_json::Value>,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatChunk, savant_core::error::SavantError>> + Send>>,
        savant_core::error::SavantError,
    > {
        // 1. Check cache (only for non-tool requests)
        if tools.is_empty() {
            if let Some(cached) = self.cache.get(&messages) {
                tracing::debug!("[{}] Returning cached response", self.chain_key);
                return Ok(Box::pin(futures::stream::iter(cached.into_iter().map(Ok))));
            }
        }

        // 2. Check circuit breaker
        if !self.breaker.is_allowed() {
            return Err(SavantError::Unknown(format!(
                "[{}] Circuit breaker is OPEN — provider temporarily unavailable",
                self.chain_key
            )));
        }

        // 3. Check cooldown
        if self.cooldown.is_on_cooldown(&self.chain_key) {
            return Err(SavantError::Unknown(format!(
                "[{}] Provider is on cooldown — try again later",
                self.chain_key
            )));
        }

        // 4. Attempt with retry
        let mut attempts = 0u32;
        let mut last_error = SavantError::Unknown("Chain exhausted".to_string());

        while attempts < self.max_retries {
            match self
                .inner
                .stream_completion(messages.clone(), tools.clone())
                .await
            {
                Ok(stream) => {
                    let chunks: Vec<ChatChunk> =
                        stream.filter_map(|r| async move { r.ok() }).collect().await;

                    self.breaker.record_success();
                    self.cooldown.record_success(&self.chain_key);
                    self.cache.put(&messages, &chunks);

                    return Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))));
                }
                Err(e) => {
                    let category = classify_error(&e);
                    attempts += 1;

                    tracing::warn!(
                        "[{}] Attempt {} failed: {:?} ({})",
                        self.chain_key,
                        attempts,
                        category,
                        e
                    );

                    self.cooldown.record_failure(&self.chain_key, category);
                    self.breaker.record_failure();

                    if !Self::is_retryable(category) {
                        return Err(e);
                    }

                    last_error = e;

                    // Exponential backoff: 500ms * 2^attempt
                    let delay = Duration::from_millis(500 * 2u64.pow(attempts - 1));
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(last_error)
    }
}
