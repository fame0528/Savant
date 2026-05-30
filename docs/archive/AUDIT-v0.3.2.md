# Savant v0.3.2 — Extended Code Audit Report

**Auditor:** Nova (Hermes Agent)
**Date:** 2026-05-26
**Scope:** Full review of new v0.3.2 systems — consciousness layer, resource governor, provider chain, auth middleware, cost-aware routing, semantic window scoring, skill system.
**Build status:** `cargo check` — 0 errors, 0 warnings | `cargo clippy --workspace -- -D warnings` — clean

---

## Summary

The v0.3.2 sprint is genuinely impressive: 27 FIDs closed, 525 files changed, ~80K lines inserted. The architecture is sound, the error handling has been thoroughly audited (0.3.1 remediation was clearly effective), and the security hardening demonstrates real understanding of production threat models. However, several subsystems still contain stubs, optimization gaps, and subtle correctness issues that need attention before this is truly production-ready.

**Issues found: 15** (2 high, 5 medium, 8 low)

---

## HIGH PRIORITY

### H1: WonderEngine `explore()` Never Calls LLM — Still a Stub

**File:** `crates/agent/src/consciousness/wonder.rs:37-60`
**Severity:** HIGH — Contradicts "zero stubs" goal

The `explore()` method builds a prompt but never sends it to the LLM. It returns a hardcoded `reward: 0.5` with the raw prompt as "content". The actual LLM-driven exploration that the CHANGELOG describes ("elevated temperature", "reward-based pruning") doesn't happen.

```rust
// Current (STUB):
Some(WonderInsight {
    content: prompt,     // Just the raw prompt string, not an LLM response
    reward: 0.5,         // Hardcoded, never evaluated
})
```

**Fix required:**

- Accept an `Arc<dyn LlmProvider>` in `WonderEngine::new()` or `explore()`
- Actually call `llm.stream_completion()` with elevated temperature (0.9)
- Feed the LLM response through `evaluate_reward()` (which IS implemented)
- The `exploration_temperature` and `reward_threshold` fields should drive actual LLM params, not sit as `#[allow(dead_code)]`

**Also:** `evaluate_reward()` is implemented with heuristic keyword scoring, which is fine for now, but should eventually use a lightweight reward model or embedding similarity against the workspace context.

---

### H2: `constant_time_eq` Leaks Key Length via Early Return

**File:** `crates/gateway/src/auth/http_middleware.rs:22-31`
**Severity:** HIGH — Security

```rust
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {   // ← Timing leak: returns immediately on length mismatch
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}
```

The length comparison short-circuits, revealing whether the attacker's key length matches the expected key length. This is a classic timing attack vector.

**Fix required:**
```rust
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    // Always compare against the expected key length
    // Use the longer length to prevent length leakage
    let max_len = a.len().max(b.len());
    let mut result = if a.len() != b.len() { 1 } else { 0 };
    // Pad iteration to always run max_len steps
    for i in 0..max_len {
        let ai = a.get(i).copied().unwrap_or(0);
        let bi = b.get(i).copied().unwrap_or(0);
        result |= ai ^ bi;
    }
    result == 0
}
```

Or better: use the `subtle` crate's `ConstantTimeEq` trait which handles this correctly.

---

## MEDIUM PRIORITY

### M1: Narrative Truncation Can Panic on UTF-8 Boundary

**File:** `crates/agent/src/consciousness/mod.rs:280-281`
**Severity:** MEDIUM — Will panic on non-ASCII content

```rust
if new_narrative.len() > 8000 {
    self.current_narrative = new_narrative[..8000].to_string();  // ← PANICS if 8000 is mid-UTF-8 char
}
```

`str::len()` returns bytes, and slicing at a byte boundary that falls inside a multi-byte UTF-8 character (emoji, CJK, accented chars) will panic. The narrative synthesizer generates text via LLM, which frequently includes emoji and unicode.

**Fix required:**
```rust
if new_narrative.len() > 8000 {
    // Find the nearest char boundary at or before 8000
    let mut truncate_at = 8000;
    while !new_narrative.is_char_boundary(truncate_at) {
        truncate_at -= 1;
    }
    self.current_narrative = new_narrative[..truncate_at].to_string();
}
```

---

### M2: Token Estimation Uses `len() / 4` — Wrong for Non-ASCII

**File:** `crates/agent/src/providers/chain.rs:616-617`
**File:** `crates/agent/src/consciousness/narrative.rs:60`
**Severity:** MEDIUM — Budget overruns on non-ASCII content

```rust
// chain.rs rate limiter:
let estimated_tokens: u32 = messages
    .iter()
    .map(|m| (m.content.len() / 4) as u32)
    .sum();

// narrative.rs budget enforcement:
if response.len() > self.max_tokens * 4 {
    break;
}
```

`str::len()` returns byte count, not character count. For CJK text (3 bytes/char), emoji (4 bytes/char), or code with unicode identifiers, this overestimates token count by 2-3x. For ASCII-heavy text it's reasonable, but the framework handles multi-language content.

**Fix required:** Either use `chars().count() / 4` for a better heuristic, or (ideally) use a tokenizer. At minimum, document the assumption that this is an approximation for ASCII-heavy content.

---

### M3: `AdaptiveSemaphore::adjust_permits()` Has a TOCTOU Race

**File:** `crates/agent/src/governor/adaptive.rs:49-77`
**Severity:** MEDIUM — Could double-add/double-forget permits under concurrent access

```rust
pub fn adjust_permits(&self) {
    let target = ...;
    let current = self.current_max.load(Ordering::Relaxed);  // ← Read
    // GAP: another thread could call adjust_permits() here
    if target > current {
        self.inner.add_permits(diff);                          // ← Write
        self.current_max.store(target, Ordering::Relaxed);     // ← Write
    } else {
        // ...
        self.inner.forget_permits(safe_forget);                // ← Write
        self.current_max.store(target, Ordering::Relaxed);     // ← Write
    }
}
```

Currently `adjust_permits()` is only called from a single interval task, so this is unlikely to trigger. But the type system doesn't enforce this invariant. If someone later adds a pressure-change callback that also calls `adjust_permits()`, permits will be corrupted.

**Fix required:** Either:

1. Document the single-caller invariant with a comment
1. Use a `Mutex<()>` or `AtomicBool` CAS spin to make it safe under concurrent access
1. Change to `&mut self` to enforce single access at the type level

---

### M4: `ConsciousnessBudget` Has No Automatic Time Reset

**File:** `crates/agent/src/consciousness/budget.rs`
**Severity:** MEDIUM — Budget counters never reset in practice

`reset_hourly()` and `reset_daily()` exist but are never called from the daemon loop. The `ConsciousnessDaemon::tick()` calls `budget.set_budget_multiplier()` every tick, which modifies `tokens_per_hour` and `tokens_per_day` based on entropy — but never resets `current_hour_tokens` or `current_day_tokens`.

This means once the budget is exhausted, it stays exhausted forever (or until the process restarts).

**Fix required:** Add a `last_hourly_reset: Instant` and `last_daily_reset: Instant` to `ConsciousnessBudget`, and check/reset them in `can_think()` or `record_usage()`:
```rust
pub fn can_think(&mut self) -> bool {
    let now = Instant::now();
    if now.duration_since(self.last_hourly_reset) >= Duration::from_secs(3600) {
        self.current_hour_tokens = 0;
        self.last_hourly_reset = now;
    }
    // ... same for daily
}
```

Note: This changes `can_think` from `&self` to `&mut self`, which requires adjusting the daemon's call site.

---

### M5: `set_budget_multiplier` Overwrites Base Budget — Accumulates Drift

**File:** `crates/agent/src/consciousness/budget.rs:73-86`
**Severity:** MEDIUM — Budget values drift with each entropy change

```rust
pub fn set_budget_multiplier(&mut self, entropy: f64) {
    let multiplier = ...; // 1.0, 0.4, 0.15, or 0.02
    self.tokens_per_hour = (100_000.0 * multiplier) as u32;  // ← Overwrites, not multiplies
    self.tokens_per_day = (500_000.0 * multiplier) as u32;
}
```

This is actually correct behavior (it recalculates from base constants each time), but the constants `100_000` and `500_000` are hardcoded rather than stored as `base_tokens_per_hour` / `base_tokens_per_day` fields. If someone changes the base budget in `new()` without updating `set_budget_multiplier()`, they'll silently diverge.

**Fix required:** Store base values as fields and reference them in `set_budget_multiplier()`.

---

## LOW PRIORITY

### L1: Narrative Synthesizer Uses 4 chars/token Approximation

**File:** `crates/agent/src/consciousness/narrative.rs:59-61`
**Severity:** LOW — Covered by M2 but worth noting separately

The `max_tokens * 4` budget enforcement in the streaming loop is a rough heuristic. For a consciousness daemon running continuously, this compounds — the narrative can grow 2-3x larger than intended when processing non-ASCII content, consuming more memory over time.

---

### L2: `EntropyCalculator` Uses `DefaultHasher` — Not Collision-Resistant

**File:** `crates/agent/src/consciousness/entropy.rs:31`
**Severity:** LOW — Functional but suboptimal

`DefaultHasher` is not designed for collision resistance. Two different hivemind states could hash to the same value, causing the entropy calculator to miss state changes. For a consciousness daemon this is low-risk (missing one state change is fine), but blake3 (already a workspace dependency) would be more robust.

**Fix:** Replace `DefaultHasher::new()` with `blake3::Hasher::new()`.

---

### L3: `AntiEchoChamber` Word-Overlap Similarity Is Case-Sensitive on Input

**File:** `crates/agent/src/consciousness/diversity.rs:78-81`
**Severity:** LOW

```rust
fn simple_similarity(&self, a: &str, b: &str) -> f64 {
    let words_a: HashSet<&str> = a.split_whitespace().filter(|w| w.len() > 3).collect();
```

The filter uses `w.len() > 3` (byte length), and the comparison is case-sensitive. "Hello" and "hello" are treated as different words. For agent outputs that may vary in capitalization, this underestimates similarity.

**Fix:** `.map(|w| w.to_lowercase())` before collecting into the HashSet, and use `w.chars().count() > 3` for the length filter.

---

### L4: Response Cache Eviction Is O(n) on Every Insert

**File:** `crates/agent/src/providers/chain.rs:403-411`
**Severity:** LOW — Performance under high cache pressure

```rust
if entries.len() >= self.max_size {
    if let Some(oldest_key) = entries
        .iter()
        .min_by_key(|(_, e)| e.inserted_at)  // ← O(n) scan
        .map(|(k, _)| k.clone())
    {
        entries.remove(&oldest_key);
    }
}
```

With `cache_max_size: 256`, this scans all entries on every cache miss. Not critical at 256 entries, but if the cache size is ever increased, this becomes a bottleneck.

**Fix:** Use a `BTreeMap<Instant, String>` alongside the HashMap for O(log n) eviction, or switch to an LRU crate like `lru` (already a workspace dependency).

---

### L5: `CostAwareRouter::classify` Is Easily Fooled by Prompt Injection

**File:** `crates/agent/src/providers/cost_router.rs:32-61`
**Severity:** LOW — Cost optimization, not security

The heuristic keyword matching can be gamed. A prompt containing "implement" gets routed to the expensive model even if it's a simple question. Conversely, a complex prompt that avoids the trigger words gets the cheap model.

This is acceptable for v1 but should eventually use an embedding-based complexity classifier or a lightweight LLM call to classify.

---

### L6: `WonderEngine::sample_environment` Only Checks Git

**File:** `crates/agent/src/consciousness/wonder.rs:62-97`
**Severity:** LOW — Limited environment awareness

The environment sampling only looks at `git log` and `git diff`. For a consciousness daemon meant to explore autonomously, it should also sample:

- Recent memory entries (via the VHSS system)
- Active agent states
- Filesystem changes outside git (e.g., config files, logs)
- System metrics (CPU, memory, uptime)

**Fix:** Accept additional samplers as trait objects, or add a `Vec<Box<dyn EnvironmentSampler>>` field.

---

### L7: `ResourceMonitor` Stores `f64` as `AtomicU64` via `to_bits()`/`from_bits()`

**File:** `crates/agent/src/governor/monitor.rs:64-65, 91-92`
**Severity:** LOW — Correct but fragile

```rust
self.cpu_pct.store(cpu_pct.to_bits(), Ordering::Relaxed);
// ...
let cpu = f64::from_bits(self.cpu_pct.load(Ordering::Relaxed));
```

This is a valid pattern for atomic f64 on platforms without `AtomicF64`, but `from_bits(NaN)` or `from_bits(0)` could produce surprising behavior. Should add a debug assertion that the loaded value is finite.

---

### L8: `ConsciousnessState::Wondering` Is Never Naturally Reached

**File:** `crates/agent/src/consciousness/mod.rs:213-219`
**Severity:** LOW — Dead state transition

The `tick()` method maps entropy to states:

- `> 0.85` → Thinking
- `> 0.10` → Idle
- `<= 0.10` → Dormant

`Wondering` is only set manually inside `wonder()` (line 301) and immediately reset to `Idle` (line 332). The state machine never naturally transitions TO `Wondering` from the entropy-based classification. This means the `ConsciousnessState::Wondering` variant and the `COGNITIVE_LENSES` are partially disconnected — the lenses rotate during `Thinking`/`Idle` but `Wondering` bypasses them.

**Fix:** Either add a `Wondering` threshold (e.g., `0.10 < entropy <= 0.25` → Wondering), or remove the variant and track wonder state separately.

---

## ARCHITECTURE NOTES (Not Issues — Observations)

### A1: Provider Chain True Streaming Is Correct

The `stream_completion` implementation correctly yields chunks directly from the provider without collecting-then-replaying. The cache intentionally does NOT cache streaming responses (tool calls are excluded). This is the right trade-off for TTFT.

### A2: Circuit Breaker Single-Lock Design Is Correct

The `RwLock<CircuitBreakerInner>` pattern (single lock for state + failure_count + last_opened) eliminates the race condition that existed in v0.3.1. This is well-designed.

### A3: Resource Governor Deferred Queue Is Well-Designed

The deferred agent queue with retry counting and max deferral retries prevents infinite retry loops while still giving agents a chance to spawn when pressure decreases. The `try_lock()` on `defer_agent()` silently drops on contention — see M3 suggestion to log this.

### A4: Semantic Window Scoring Weights Are Reasonable

The multi-head scoring (role_weight *0.25 + recency* 0.30 + keyword_relevance * 0.30 + causal_boost) with pinned system messages is a solid approach. The `sqrt` recency curve is a good choice — it gives moderate recency bias without completely ignoring old context.

---

## RECOMMENDED PRIORITY ORDER

1. **H2** — Fix constant-time comparison (security, 15 min fix)
1. **H1** — Wire up WonderEngine LLM call (core feature gap, 1-2 hours)
1. **M4** — Add automatic budget reset (consciousness daemon is broken without this)
1. **M1** — Fix UTF-8 truncation panic (will crash on emoji/unicode)
1. **M2** — Fix token estimation (budget accuracy)
1. **M5** — Store base budget values as fields (prevents future drift)
1. **M3** — Document or fix semaphore TOCTOU
1. **L1-L8** — Address in next cleanup pass

---

## CHANGELOG VERIFICATION

I cross-referenced the CHANGELOG claims against actual code. Most are accurate. Two discrepancies:

1. **"WonderEngine — Autonomous exploration during idle with environment sampling and reward-based pruning"** — The environment sampling exists, but the LLM-driven exploration and reward-based pruning are stubbed (H1).

1. **"ConsciousnessBudget — Token/cost budget with quiet hours (10PM–7AM) and entropy-based scaling"** — Quiet hours work, entropy scaling works, but the budget counters never reset (M4), so the budget effectively becomes permanent after first exhaustion.

Everything else in the CHANGELOG is accurately implemented and functional.

---

*Generated by Nova — 2026-05-26.*
