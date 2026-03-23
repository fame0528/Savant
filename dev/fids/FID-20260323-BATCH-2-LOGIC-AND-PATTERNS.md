# FID-20260323-BATCH-2-LOGIC-AND-PATTERNS

**Date:** 2026-03-23
**Status:** RE-AUDITED — Fix matrix certified via Perfection Loop

---

## FIX MATRIX (Re-audited post-production-pass + Batch 1)

| # | Severity | Issue | File | Current Line | Fix | Cross-Impact | Gate |
|---|----------|-------|------|------|-----|-------------|------|
| N1 | MEDIUM | All HTTP failures mapped to `AuthError` | `agent/providers/mod.rs` | 223 | Classify by status: 401→AuthError, 429→RateLimit, 500-599→Transient, timeout→Timeout, other→NetworkError | Provider chain error classification, cooldown, circuit breaker |
| N2 | MEDIUM | Leap year bug — month calc misses century/400-year rules | `memory/daily_log.rs` | 237 | Replace manual date math with `chrono` crate (already a dependency) for `days_to_ymd` and `date_to_days` | Daily log file naming, date-based queries |
| N3 | MEDIUM | `DefaultHasher` for memory entry IDs — collision risk, unstable across Rust versions | `memory/distillation.rs` | 93-96 | Use `blake3` hash of `msg.id` as deterministic u64 (same pattern as Phase 1.1 fix for async_backend) | Distillation pipeline, memory entry persistence |
| N4 | MEDIUM | `text.split('.')` breaks URLs, decimals, abbreviations | `memory/entities.rs` | 154 | Context-aware splitting: split on sentence-ending `.` (followed by space + capital) but not `.` in URLs, decimals, abbreviations | Entity extraction accuracy |
| N5 | MEDIUM | `println!` in production code | `channels/cli.rs` | 15 | Replace with `tracing::info!` | CLI channel log aggregation |

**Re-audit findings:**
- N1: CONFIRMED at line 223 — `SavantError::AuthError(format!("OpenAI request failed: {}", e))` for ALL failures
- N2: CONFIRMED at line 237 — month leap year check `y % 4 == 0` misses century/400-year rules (year check at line 223 is correct)
- N3: CONFIRMED at lines 93-96 — `DefaultHasher::new()` used for entry ID generation
- N4: CONFIRMED at line 154 — `text.split('.')` — naive period splitting
- N5: CONFIRMED at line 15 — `println!("[CLI] {}", event.payload)`

**Cross-Impact for Batch 2:**
```
providers/mod.rs:223 → Provider chain error classification, cooldown timers, circuit breaker
daily_log.rs:237 → Daily log file naming, date-based queries, log rotation
distillation.rs:93-96 → Memory entry ID generation, deduplication, persistence
entities.rs:154 → Entity extraction accuracy, knowledge graph construction
cli.rs:15 → CLI channel output, log aggregation
```

---

## PERFECTION LOOP ITERATIONS

| Iteration | Changes |
|-----------|---------|
| 1 | Initial FID — 5 fixes, line numbers from audit |
| 2 | Re-audited all 5 files 0-EOF — confirmed issues, verified current line numbers, added certified fix approaches |

---

## CERTIFIED FIX APPROACHES

**N1 (providers/mod.rs:223):**
```rust
// BEFORE:
.map_err(|e| SavantError::AuthError(format!("OpenAI request failed: {}", e)))?

// AFTER:
.map_err(|e| classify_http_error(e))?

// Helper:
fn classify_http_error(e: reqwest::Error) -> SavantError {
    if e.is_timeout() { SavantError::Timeout(format!("Request timed out: {}", e)) }
    else if e.is_connect() { SavantError::NetworkError(format!("Connection failed: {}", e)) }
    else if let Some(status) = e.status() {
        match status.as_u16() {
            401 | 403 => SavantError::AuthError(format!("Auth failed: {}", e)),
            429 => SavantError::RateLimit(format!("Rate limited: {}", e)),
            500..=599 => SavantError::Unknown(format!("Server error {}: {}", status, e)),
            _ => SavantError::NetworkError(format!("HTTP {}: {}", status, e)),
        }
    } else { SavantError::NetworkError(format!("Request failed: {}", e)) }
}
```

**N2 (daily_log.rs:218-282):**
```rust
// BEFORE: Manual date math with leap year bug in month calc
fn days_to_ymd(days: i64) -> (i32, u32, u32) { /* manual loop */ }

// AFTER: chrono-based (already a dependency)
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    chrono::NaiveDate::from_num_days_from_ce_opt(days as i32 + 719163) // Unix epoch offset
        .map(|d| (d.year(), d.month(), d.day()))
        .unwrap_or((1970, 1, 1))
}
fn date_to_days(date: &str) -> i64 {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|d| d.signed_duration_since(chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()).num_days())
        .unwrap_or(0)
}
```

**N3 (distillation.rs:93-96):**
```rust
// BEFORE: DefaultHasher (unstable across Rust versions)
let mut s = std::collections::hash_map::DefaultHasher::new();
s.write(msg.id.as_bytes());
let entry_id = s.finish();

// AFTER: blake3 (deterministic, stable, already a dependency)
let hash = blake3::hash(msg.id.as_bytes());
let bytes = hash.as_bytes();
let entry_id = u64::from_le_bytes(bytes[..8].try_into().unwrap_or([0u8; 8]));
```

**N4 (entities.rs:154):**
```rust
// BEFORE: Naive period splitting
for sentence in text.split('.') { ... }

// AFTER: Context-aware sentence boundary detection
// Split on periods that are followed by space + uppercase (sentence end)
// But NOT periods in URLs, decimals, or common abbreviations
fn split_sentences(text: &str) -> Vec<String> { /* regex or char-by-char parser */ }
```

**N5 (cli.rs:15):**
```rust
// BEFORE:
println!("[CLI] {}", event.payload);

// AFTER:
tracing::info!("[CLI] {}", event.payload);
```
