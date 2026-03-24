# FID-20260324-DEEP-AUDIT-EXPANDED

> **Framework Issue Document** | Deep Audit Expansion & Remediation Plan

| Field            | Value                                                                 |
|------------------|-----------------------------------------------------------------------|
| **Document ID**  | FID-20260324-DEEP-AUDIT-EXPANDED                                     |
| **Date Created** | 2026-03-24                                                           |
| **Status**       | `OPEN` — Pending Implementation                                      |
| **Priority**     | `P0` — Security Vulnerabilities + Broken Core Functionality            |
| **Author**       | Automated Audit Pipeline (Python) + Manual Review (Kilo)              |
| **Protocol**     | FID / Perfection Loop — Iteration 2 (corrected + stub scan)           |
| **Scope**        | `crates/*` (16 crates) + `lib/cortexadb/` (vendored, read-only)       |
| **Supersedes**   | None — new FID                                                      |

---

## 1. Executive Summary

A Python-based static analysis pipeline scanned the full Savant codebase and flagged **1,492 violations** across 5 pattern categories. A manual review filtered false positives, test code, vendored library artifacts, and legitimate patterns. A subsequent manual scan identified **8 additional stub implementations** the Python pipeline could not detect.

| Metric                          | Value        |
|---------------------------------|--------------|
| Raw violations (Python)         | 1,492        |
| False positives filtered        | 1,173 (79%)  |
| Actionable findings (post-filter)| 319          |
| Stub/placeholder findings       | 14           |
| **Total actionable findings**   | **333**      |
| Critical (P0)                   | 5            |
| High (P1)                       | 10           |
| Medium (P2)                     | 5            |
| Low (P3)                        | 4            |
| Estimated remediation effort    | 3 phases     |

---

## 2. Raw Finding Taxonomy

### 2.1 Python Audit Categories (Pre-Filter)

| #  | Category                              | Raw Count | Production Count | Filtered Out | Reason for Filtering                          |
|----|---------------------------------------|-----------|------------------|--------------|-----------------------------------------------|
| 1  | Global Mutex Contention               | 174       | 42               | 132          | Single-writer TCP streams, test code          |
| 2  | Unsandboxed File IO / TOCTOU          | 224       | 67               | 157          | Test code, CortexaDB vendored, canvas events  |
| 3  | Silent State Corruption (`let _ =`)   | 191       | 127              | 64           | Test code, fire-and-forget patterns           |
| 4  | Unbounded SSRF / No Timeouts          | 70        | 58               | 12           | Test code, CortexaDB internal                 |
| 5  | Unhandled Panics (`unwrap/expect`)    | 833       | 25               | 808          | Test code (~500), regex init (~80), intentional loud failures (~45), CortexaDB (~150), misc (~33) |

### 2.2 Manual Scan Categories (Post-Filter)

| #  | Category                              | Count | Severity       |
|----|---------------------------------------|-------|----------------|
| 6  | Agent delegate stubs (S-1)            | 3     | `CRITICAL`     |
| 7  | Feature implementation stubs (S-2–S-4)| 8     | `HIGH`/`MEDIUM`|
| 8  | Dead code / exposure risk (S-5–S-8)   | 3     | `LOW`/`MEDIUM` |

---

## 3. Remediation Plan

### Phase 1 — Security & Critical Functionality

> **Gate:** `cargo check --workspace && cargo clippy --workspace && cargo test -p savant-core && cargo test -p savant-agent`

**Phase 1 contains 5 CRITICAL items (C-1–C-4, S-1) and 3 HIGH items (C-5–C-7). The CRITICAL items are security vulnerabilities and broken core functionality. The HIGH items are observability gaps.**

#### C-1 — TOCTOU Permission Escalation on Crypto Key Files

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | C-1                                                                   |
| **Severity**   | `CRITICAL`                                                            |
| **File**       | `crates/core/src/crypto.rs:71-78`                                     |
| **Risk**       | Low — isolated file write                                             |
| **Status**     | `OPEN`                                                                |

**Problem:** `fs::write()` creates files with default permissions (0o644), then `set_permissions(0o600)` is applied in a separate step. An attacker with inotify access can read the master encryption key during the TOCTOU window.

```rust
// CURRENT — vulnerable
fs::write(path, json)?;                          // ← 0o644 world-readable
let perms = Permissions::from_mode(0o600);
let _ = std::fs::set_permissions(path, perms);   // ← fixed AFTER creation
```

**Fix:** Use `OpenOptions` with atomic mode setting at file descriptor allocation.

```rust
// FIXED — atomic permissions
use std::os::unix::fs::OpenOptionsExt;
let f = OpenOptions::new()
    .write(true).create(true).truncate(true)
    .mode(0o600)
    .open(path)?;
f.write_all(json.as_bytes())?;
f.sync_all()?;
```

**Acceptance Criteria:**
- [ ] File created with 0o600 permissions atomically (verified via `stat`)
- [ ] `cargo test -p savant-core crypto` passes
- [ ] No new `unwrap()` introduced

---

#### C-2 — TOCTOU on Config File Writes

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | C-2                                                                   |
| **Severity**   | `CRITICAL`                                                            |
| **File**       | `crates/core/src/config.rs:446-450`                                   |
| **Risk**       | Low — isolated file write                                             |
| **Status**     | `OPEN`                                                                |

**Problem:** Config tmp file created with default permissions. Config contains API keys, model configurations, and security settings.

**Fix:** Same `OpenOptions::mode(0o600)` pattern as C-1. Apply to tmp file creation.

**Acceptance Criteria:**
- [ ] Tmp file created with 0o600 permissions atomically
- [ ] `cargo test -p savant-core config` passes
- [ ] Atomic rename preserved

---

#### C-3 — SSRF Unsafe Fallback Client (web.rs)

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | C-3                                                                   |
| **Severity**   | `CRITICAL`                                                            |
| **File**       | `crates/agent/src/tools/web.rs:59-64`                                 |
| **Risk**       | Low — single file change                                              |
| **Status**     | `OPEN`                                                                |

**Problem:** HTTP client builder fallback silently creates an untimeouted, unprotected client. If the builder fails, the agent has no SSRF protection, no timeout, and no redirect limit.

```rust
// CURRENT — silent security downgrade
.unwrap_or_else(|_| reqwest::Client::new())  // ← NO timeout, NO SSRF protection
```

**Fix:** Replace with `.expect()` — crash loudly rather than silently creating an insecure client.

```rust
// FIXED — loud failure
.build()
.expect("CRITICAL: Failed to build HTTP client with security constraints")
```

**Acceptance Criteria:**
- [ ] No `reqwest::Client::new()` fallback in web.rs
- [ ] `cargo test -p savant-agent tools::web` passes
- [ ] Manual: agent hitting `http://169.254.169.254` returns error (not hangs)

---

#### C-4 — SSRF Global `reqwest::Client::new()` Replacement

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | C-4                                                                   |
| **Severity**   | `CRITICAL`                                                            |
| **Files**      | 58 call sites across 15+ crates                                       |
| **Risk**       | Medium — global change affecting every HTTP call                      |
| **Status**     | `OPEN`                                                                |

**Problem:** Every `reqwest::Client::new()` has no timeout, no connection pool limits, no SSRF protection. Agents can be weaponized to query cloud metadata endpoints (AWS `169.254.169.254`, Alibaba `100.100.100.200`) or hang on slow-loris targets.

**Fix:** Create centralized secure client factory.

**New file:** `crates/core/src/net/mod.rs`
```rust
use std::time::Duration;

pub fn secure_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .connect_timeout(Duration::from_secs(5))
        .pool_max_idle_per_host(4)
        .build()
        .expect("Failed to build secure HTTP client")
}
```

**Migration:** Replace all `reqwest::Client::new()` with `savant_core::net::secure_client()`.

| Crate        | Call Sites | Files                                              |
|--------------|------------|-----------------------------------------------------|
| channels     | 25         | bluesky, dingtalk, feishu, generic_webhook, google_chat, line, matrix, mattermost, notion, reddit, signal, slack, teams, voice, wecom, whatsapp_business, x |
| gateway      | 5          | auth/mod.rs, handlers/mod.rs, handlers/setup.rs     |
| core         | 4          | ollama_embeddings.rs, ollama_vision.rs               |
| skills       | 8          | clawhub.rs, lambda.rs, security.rs                   |
| agent        | 4          | swarm.rs, providers/mod.rs, tools/web.rs             |

**Acceptance Criteria:**
- [ ] Zero `reqwest::Client::new()` in production code (excluding tests)
- [ ] `cargo check --workspace` passes
- [ ] `cargo clippy --workspace` passes
- [ ] All channel adapters functional after migration

---

#### C-5 — Gateway Handler Result Discard

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | C-5                                                                   |
| **Severity**   | `HIGH`                                                                |
| **File**       | `crates/gateway/src/handlers/mod.rs:296-347`                          |
| **Risk**       | Low — adding logging only                                             |
| **Status**     | `OPEN`                                                                |

**Problem:** 12 gateway handlers discard their results with `let _ =`. Users receive no response and no error when handlers fail.

| Line | Silenced Handler                        |
|------|-----------------------------------------|
| 296  | `handle_config_get`                     |
| 308  | `handle_config_set`                     |
| 311  | `handle_models_list`                    |
| 314  | `handle_parameter_descriptors`          |
| 317  | `handle_agent_config_get`               |
| 347  | `handle_agent_config_set`               |

**Fix:** Replace each with traced error handling + client error response.

```rust
if let Err(e) = handle_config_get(&state.nexus).await {
    tracing::error!("[gateway] Config get failed: {}", e);
    let _ = send_control_response("CONFIG_ERROR",
        serde_json::json!({"error": e.to_string()}),
        session_id, nexus).await;
}
```

**Acceptance Criteria:**
- [ ] All 12 handlers have error logging
- [ ] Error responses sent back to client on failure
- [ ] `cargo test -p savant-gateway` passes

---

#### C-6 — Agent Pulse Telemetry Silent Loss

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | C-6                                                                   |
| **Severity**   | `HIGH`                                                                |
| **File**       | `crates/agent/src/pulse/heartbeat.rs:195-716`                        |
| **Risk**       | Low — adding logging only                                             |
| **Status**     | `OPEN`                                                                |

**Problem:** 15 telemetry operations silently fail. Heartbeat data, emergent behavior detection, and proactive context distillation are lost. ALD metrics become unreliable.

**Fix:** Replace each `let _ =` with `tracing::warn!` on failure.

**Acceptance Criteria:**
- [ ] All 15 `let _ =` instances have error tracing
- [ ] `cargo test -p savant-agent pulse` passes

---

#### C-7 — Session/Turn State Save Failures

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | C-7                                                                   |
| **Severity**   | `HIGH`                                                                |
| **File**       | `crates/agent/src/react/stream.rs:122-766`                           |
| **Risk**       | Low — adding logging + retry                                          |
| **Status**     | `OPEN`                                                                |

**Problem:** 10 session/turn save operations silently fail. Agent loses conversation context on restart. ALD training data is lost.

**Fix:** Log at `error` level. Add single retry on session save failure.

**Acceptance Criteria:**
- [ ] All save operations have error tracing
- [ ] Session save retries once on failure
- [ ] `cargo test -p savant-agent react` passes

---

#### S-1 — Agent Delegates Return Empty Responses

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | S-1                                                                   |
| **Severity**   | `CRITICAL`                                                            |
| **Files**      | `crates/agent/src/react/mod.rs:81-89, 131-138, 172-179`              |
| **Risk**       | High — core agent loop change                                         |
| **Status**     | `OPEN`                                                                |

**Problem:** All three delegate `call_llm` implementations return empty `ChatResponse`:
```rust
async fn call_llm(&self, _ctx: &mut LoopContext<'_, M>) -> Result<ChatResponse, ...> {
    Ok(ChatResponse { content: String::new(), tool_calls: vec![] })  // ← STUB
}
```

| Delegate              | Location  | Status   |
|-----------------------|-----------|----------|
| `ChatDelegate`        | Line 81   | `STUB`   |
| `HeartbeatDelegate`   | Line 131  | `STUB`   |
| `SpeculativeDelegate` | Line 172  | `STUB`   |

**Impact:** The agent loop infrastructure exists (tools, hooks, memory, streaming) but the core LLM integration was never wired. The agent cannot function.

**Fix:** Wire `call_llm` to `ctx.provider.stream_completion(messages, tools)` and collect the response. This is the single highest-priority fix in the entire audit.

**Acceptance Criteria:**
- [ ] `ChatDelegate::call_llm` returns non-empty response from LLM provider
- [ ] `HeartbeatDelegate::call_llm` wired to provider
- [ ] `SpeculativeDelegate::call_llm` wired to provider
- [ ] `cargo test -p savant-agent` passes
- [ ] Manual: agent responds to user input with LLM-generated content

---

### Phase 2 — Concurrency, Performance & Feature Stubs

> **Gate:** `cargo check --workspace && cargo clippy --workspace && cargo test -p savant-agent && cargo test -p savant-memory && cargo test -p savant-mcp`

#### H-1 — Swarm Mutex Contention

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | H-1                                                                   |
| **Severity**   | `HIGH`                                                                |
| **File**       | `crates/agent/src/swarm.rs:71-725`                                    |
| **Lock Points**| 7 (5 hot paths)                                                       |
| **Risk**       | Medium — structural change to agent lifecycle                         |
| **Status**     | `OPEN`                                                                |

**Problem:** `handles: Mutex<HashMap<String, ...>>` serializes all agent lifecycle events. Under 50+ concurrent agents, lock contention blocks spawn/evacuate operations.

**Fix:**
- Replace `Mutex<HashMap<...>>` with `dashmap::DashMap`
- Replace `Mutex<Vec<String>>` (dead_agents) with `Arc<SegQueue<String>>`

**Acceptance Criteria:**
- [ ] No `Mutex<HashMap>` in swarm.rs
- [ ] `cargo test -p savant-agent swarm` passes
- [ ] Concurrent spawn test with 50 agents completes without contention

---

#### H-2 — React Loop Mutex Counters (MEDIUM)

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | H-2                                                                   |
| **Severity**   | `MEDIUM`                                                              |
| **File**       | `crates/agent/src/react/mod.rs:365-522`                              |
| **Lock Points**| 6                                                                     |
| **Risk**       | Low — type swap, no behavioral change                                |
| **Status**     | `OPEN`                                                                |

**Problem:** `Arc<Mutex<u32>>` and `Arc<Mutex<usize>>` for per-turn counters. Idiomatic Rust uses atomics for single-value counters.

**Fix:** Replace with `Arc<AtomicU32>` and `Arc<AtomicUsize>`.

**Acceptance Criteria:**
- [ ] No `Mutex` on primitive counters in react/mod.rs
- [ ] `cargo test -p savant-agent react` passes

---

#### H-3 — Memory Engine Write Lock Serialization

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | H-3                                                                   |
| **Severity**   | `HIGH`                                                                |
| **File**       | `crates/memory/src/engine.rs:42-260`                                  |
| **Lock Points**| 8                                                                     |
| **Risk**       | High — all memory operations depend on this lock                      |
| **Status**     | `OPEN`                                                                |

**Problem:** Single `tokio::sync::Mutex<()>` serializes all memory writes across all agents. Under high concurrency, operations queue up.

**Fix:** Partition locks per agent or per memory namespace.

**Acceptance Criteria:**
- [ ] No global write lock for memory operations
- [ ] `cargo test -p savant-memory` passes
- [ ] Concurrent write test with 100 operations completes without serialization

---

#### H-4 — Embedding Cache Mutex Serialization

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | H-4                                                                   |
| **Severity**   | `HIGH`                                                                |
| **File**       | `crates/core/src/utils/embeddings.rs:25-181`                         |
| **Lock Points**| 8                                                                     |
| **Risk**       | Low — RwLock is drop-in for read-heavy workloads                      |
| **Status**     | `OPEN`                                                                |

**Problem:** `Mutex<LruCache<...>>` serializes all cache reads. Embedding lookups are read-heavy — cache hits should not block each other.

**Fix:** Replace `Mutex<LruCache>` with `parking_lot::RwLock<LruCache>`.

**Acceptance Criteria:**
- [ ] Cache uses RwLock for reads
- [ ] `cargo test -p savant-core utils::embeddings` passes

---

#### H-5 — MCP Client Lock Contention

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | H-5                                                                   |
| **Severity**   | `HIGH`                                                                |
| **File**       | `crates/mcp/src/client.rs:81-634`                                     |
| **Lock Points**| 12                                                                    |
| **Risk**       | Medium — structural change to MCP client                              |
| **Status**     | `OPEN`                                                                |

**Problem:** `responses: Arc<Mutex<HashMap<...>>>` serializes all MCP response lookups. Multiple concurrent tool calls to the same MCP server queue up.

**Fix:** Replace `Mutex<HashMap>` with `DashMap` for responses. Keep `Mutex` for connection (single-writer TCP is correct).

**Acceptance Criteria:**
- [ ] Response map uses DashMap
- [ ] `cargo test -p savant-mcp` passes

---

#### H-6 — Remaining `let _ =` Systematic Replacement

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | H-6                                                                   |
| **Severity**   | `HIGH`                                                                |
| **Scope**      | 77 instances across all crates                                        |
| **Risk**       | Low — adding logging only                                             |
| **Status**     | `OPEN`                                                                |

**Problem:** After C-5, C-6, C-7 are resolved, 77 additional `let _ =` patterns remain in production code. Each is a potential silent data loss.

**Priority order:**
1. Channel event bus sends (message delivery)
2. Memory vector operations (data integrity)
3. MCP server responses (tool execution)
4. Desktop event emission (UI events — `debug` level)
5. Miscellaneous cleanup

**Acceptance Criteria:**
- [ ] Zero `let _ =` in production code outside of documented fire-and-forget patterns
- [ ] `cargo check --workspace` passes

---

#### S-2 — Memory Consolidation Is a No-Op

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | S-2                                                                   |
| **Severity**   | `HIGH`                                                                |
| **File**       | `crates/core/src/memory/mod.rs:52-55`                                 |
| **Risk**       | Medium — affects memory subsystem behavior                            |
| **Status**     | `OPEN`                                                                |

**Problem:** `consolidate()` logs and returns `Ok(())` without performing any consolidation. Session memory grows unbounded.

**Fix:** Implement consolidation: merge duplicate memories, compress old entries, apply decay weights.

**Acceptance Criteria:**
- [ ] `consolidate()` reduces memory count for duplicate-heavy sessions
- [ ] `cargo test -p savant-memory` passes

---

#### S-4 — NLP Command Dispatchers Return Fake Responses

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | S-4                                                                   |
| **Severity**   | `HIGH`                                                                |
| **File**       | `crates/core/src/nlp/commands.rs:22-91`                               |
| **Risk**       | Medium — changes user-facing command behavior                         |
| **Status**     | `OPEN`                                                                |

**Problem:** All 6 command handlers return hardcoded static text. Users receive fake confirmations for commands that were never executed.

| Command    | Fake Response                                           | Should Do                        |
|------------|---------------------------------------------------------|----------------------------------|
| `"list"`   | "Use the agents sidebar..."                             | Query agent registry, list names |
| `"restart"`| "Restart command queued for..."                         | Signal swarm to restart agent    |
| `"stop"`   | "Channel disabled."                                     | Signal channel pool to stop      |
| `"status"` | "All services operational."                             | Check health of all subsystems   |
| `"memory"` | "Check the dashboard..."                                | Query memory engine stats        |

**Fix:** Wire each command to the actual subsystem. This is a trust-breaking UX issue.

**Acceptance Criteria:**
- [ ] `"list"` returns actual agent names from registry
- [ ] `"restart {agent}"` triggers actual swarm restart
- [ ] `"status"` returns real health check results
- [ ] `cargo test -p savant-core nlp` passes with operation verification

---

### Phase 3 — Cleanup & Low-Priority

> **Gate:** `cargo check --workspace && cargo clippy --workspace && cargo test --workspace`

#### M-1 — Production `unwrap()`/`expect()` Cleanup

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | M-1                                                                   |
| **Severity**   | `MEDIUM`                                                              |
| **Count**      | 25 production instances                                               |
| **Risk**       | Low — replace with `?` propagation                                    |
| **Status**     | `OPEN`                                                                |

**Breakdown:**

| Location                                    | Count | Context                                  |
|---------------------------------------------|-------|------------------------------------------|
| `gateway/src/auth/mod.rs:213-465`           | 15    | JWT serialization/deserialization        |
| `cognitive/src/predictor.rs:380-482`        | 2     | Predictor creation (test-adjacent)       |
| `agent/src/tools/coercion.rs:122`           | 1     | Tool schema coercion                     |
| `memory/src/async_backend.rs`               | 3     | Backend initialization                   |
| Other                                       | 4     | Scattered                                |

**Acceptance Criteria:**
- [ ] Zero `unwrap()` in non-test production code
- [ ] `cargo test --workspace` passes

---

#### M-2 — Channel Event Bus Tracing

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | M-2                                                                   |
| **Severity**   | `MEDIUM`                                                              |
| **Scope**      | 15 channel adapters                                                   |
| **Risk**       | Low — adding logging only                                             |
| **Status**     | `OPEN`                                                                |

**Acceptance Criteria:**
- [ ] All channel adapters log event bus send failures
- [ ] `cargo check -p savant-channels` passes

---

#### M-3 — Desktop Event Emission Logging

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | M-3                                                                   |
| **Severity**   | `LOW`                                                                 |
| **File**       | `crates/desktop/src-tauri/src/main.rs:26-209`                        |
| **Risk**       | Low — adding debug logging                                            |
| **Status**     | `OPEN`                                                                |

**Acceptance Criteria:**
- [ ] All Tauri event emissions have debug-level logging on failure

---

#### S-3 — `cull_low_entropy_memories` Always Returns 0

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | S-3                                                                   |
| **Severity**   | `MEDIUM`                                                              |
| **File**       | `crates/memory/src/engine.rs:415`                                     |
| **Risk**       | Low — additive feature                                                |
| **Status**     | `OPEN`                                                                |

**Acceptance Criteria:**
- [ ] Function evaluates memory entropy and returns actual count of culled entries
- [ ] `cargo test -p savant-memory` passes

---

#### S-5 — `MemoryLayer` Enum Dead Code

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | S-5                                                                   |
| **Severity**   | `LOW`                                                                 |
| **File**       | `crates/memory/src/engine.rs:14-23`                                   |
| **Status**     | `OPEN`                                                                |

**Acceptance Criteria:**
- [ ] Enum either removed or wired into memory operations

---

#### S-6 — `AgentRegistry.defaults` Unused Field

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | S-6                                                                   |
| **Severity**   | `LOW`                                                                 |
| **File**       | `crates/core/src/fs/registry.rs:10-11`                                |
| **Status**     | `OPEN`                                                                |

**Acceptance Criteria:**
- [ ] Field either used or removed

---

#### S-7 — `ensure_stable_id` Dead Code

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | S-7                                                                   |
| **Severity**   | `LOW`                                                                 |
| **File**       | `crates/core/src/fs/registry.rs:473-480`                              |
| **Status**     | `OPEN`                                                                |

**Acceptance Criteria:**
- [ ] Function either called or removed

---

#### S-8 — `MockEmbeddingProvider` Publicly Exposed

| Field          | Value                                                                 |
|----------------|-----------------------------------------------------------------------|
| **ID**         | S-8                                                                   |
| **Severity**   | `MEDIUM`                                                              |
| **File**       | `crates/memory/src/lib.rs:46-64`                                      |
| **Status**     | `OPEN`                                                                |

**Problem:** Returns zero vectors (384-dim) and is `pub`. Nothing prevents production code from using it, making semantic search meaningless.

**Fix:** Gate behind `#[cfg(test)]` or change to `pub(crate)`.

**Acceptance Criteria:**
- [ ] `MockEmbeddingProvider` not accessible from external crates in release builds

---

## 4. Consolidated Fix Matrix

| Phase | ID    | Description                                | Items | Priority   | Status  |
|-------|-------|--------------------------------------------|-------|------------|---------|
| 1     | C-1   | TOCTOU crypto key permissions              | 1     | `CRITICAL` | `OPEN`  |
| 1     | C-2   | TOCTOU config file permissions             | 1     | `CRITICAL` | `OPEN`  |
| 1     | C-3   | SSRF unsafe fallback client                | 1     | `CRITICAL` | `OPEN`  |
| 1     | C-4   | SSRF global Client::new() replacement      | 58    | `CRITICAL` | `OPEN`  |
| 1     | C-5   | Gateway handler result discard             | 12    | `HIGH`     | `OPEN`  |
| 1     | C-6   | Agent telemetry silent loss                | 15    | `HIGH`     | `OPEN`  |
| 1     | C-7   | Session/turn save failures                 | 10    | `HIGH`     | `OPEN`  |
| 1     | S-1   | Wire agent delegates to LLM                | 3     | `CRITICAL` | `OPEN`  |
| 2     | H-1   | Swarm DashMap migration                    | 7     | `HIGH`     | `OPEN`  |
| 2     | H-2   | React loop Atomic counters                 | 6     | `MEDIUM`   | `OPEN`  |
| 2     | H-3   | Memory engine partitioned locking          | 8     | `HIGH`     | `OPEN`  |
| 2     | H-4   | Embedding cache RwLock                     | 8     | `HIGH`     | `OPEN`  |
| 2     | H-5   | MCP client DashMap                         | 12    | `HIGH`     | `OPEN`  |
| 2     | H-6   | Remaining let _ = tracing                 | 77    | `HIGH`     | `OPEN`  |
| 2     | S-2   | Implement memory consolidation             | 1     | `HIGH`     | `OPEN`  |
| 2     | S-4   | Implement NLP command dispatchers          | 6     | `HIGH`     | `OPEN`  |
| 3     | M-1   | Production unwrap cleanup                  | 25    | `MEDIUM`   | `OPEN`  |
| 3     | M-2   | Channel event bus tracing                  | 15    | `MEDIUM`   | `OPEN`  |
| 3     | M-3   | Desktop event emission logging             | 8     | `LOW`      | `OPEN`  |
| 3     | S-3   | Implement cull_low_entropy_memories        | 1     | `MEDIUM`   | `OPEN`  |
| 3     | S-5   | MemoryLayer dead code                      | 1     | `LOW`      | `OPEN`  |
| 3     | S-6   | AgentRegistry.defaults unused              | 1     | `LOW`      | `OPEN`  |
| 3     | S-7   | ensure_stable_id dead code                 | 1     | `LOW`      | `OPEN`  |
| 3     | S-8   | MockEmbeddingProvider exposure             | 1     | `MEDIUM`   | `OPEN`  |

**Totals:**

| Metric                  | Value           |
|-------------------------|-----------------|
| Total fix items         | 24              |
| Total code locations    | 270             |

---

## 5. Cross-Impact Analysis

| Fix ID | Affected Subsystem          | Risk Level | Integration Test Required |
|--------|-----------------------------|------------|---------------------------|
| C-1    | crypto.rs                   | `LOW`      | crypto key roundtrip      |
| C-2    | config.rs                   | `LOW`      | config save/load cycle    |
| C-3    | web tool                    | `LOW`      | SSRF rejection test       |
| C-4    | ALL crates (global import)  | `MEDIUM`   | full HTTP call verification|
| C-5    | gateway, dashboard          | `LOW`      | dashboard config CRUD     |
| C-6    | pulse/heartbeat             | `LOW`      | telemetry delivery test   |
| C-7    | agent loop                  | `LOW`      | session persistence test  |
| S-1    | agent loop (core)           | `HIGH`     | end-to-end agent response |
| H-1    | swarm lifecycle             | `MEDIUM`   | concurrent spawn test     |
| H-2    | react loop                  | `LOW`      | turn counter accuracy     |
| H-3    | memory subsystem            | `HIGH`     | concurrent write load test|
| H-4    | embedding, distillation     | `LOW`      | embedding batch test      |
| H-5    | MCP tool execution          | `MEDIUM`   | concurrent MCP calls      |
| H-6    | all crates                  | `LOW`      | none (logging only)       |
| S-2    | memory subsystem            | `MEDIUM`   | consolidation correctness |
| S-4    | CLI, dashboard commands     | `MEDIUM`   | command execution tests   |
| M-1    | auth, cognitive, memory     | `LOW`      | none                      |
| M-2    | all channels                | `LOW`      | none (logging only)       |
| M-3    | desktop app                 | `LOW`      | none (logging only)       |
| S-3    | memory subsystem            | `LOW`      | entropy culling test      |
| S-5    | memory subsystem            | `LOW`      | none                      |
| S-6    | core/fs                     | `LOW`      | none                      |
| S-7    | core/fs                     | `LOW`      | none                      |
| S-8    | memory subsystem            | `LOW`      | external crate access test|

---

## 6. Validation Protocol

### Phase 1 Gate Criteria

| #  | Check                                              | Command                                              |
|----|----------------------------------------------------|------------------------------------------------------|
| 1  | Workspace compiles                                 | `cargo check --workspace`                            |
| 2  | Zero clippy warnings                               | `cargo clippy --workspace`                           |
| 3  | Core tests pass                                    | `cargo test -p savant-core`                          |
| 4  | Agent tests pass                                   | `cargo test -p savant-agent`                         |
| 5  | Gateway tests pass                                 | `cargo test -p savant-gateway`                       |
| 6  | SSRF rejection (manual)                            | Agent hitting `http://169.254.169.254` must error    |
| 7  | LLM integration (manual)                           | Agent responds to user input with non-empty content  |
| 8  | Crypto key permissions (manual)                    | `stat` on key file shows 0600                        |

### Phase 2 Gate Criteria

| #  | Check                                              | Command                                              |
|----|----------------------------------------------------|------------------------------------------------------|
| 1  | Workspace compiles                                 | `cargo check --workspace`                            |
| 2  | Zero clippy warnings                               | `cargo clippy --workspace`                           |
| 3  | Swarm concurrent test                              | `cargo test -p savant-agent swarm`                   |
| 4  | Memory tests pass                                  | `cargo test -p savant-memory`                        |
| 5  | MCP tests pass                                     | `cargo test -p savant-mcp`                           |
| 6  | NLP command tests (operation verified)             | `cargo test -p savant-core nlp`                      |
| 7  | 50 concurrent agents (manual)                      | Spawn test without lock contention                   |

### Phase 3 Gate Criteria

| #  | Check                                              | Command                                              |
|----|----------------------------------------------------|------------------------------------------------------|
| 1  | Workspace compiles                                 | `cargo check --workspace`                            |
| 2  | Zero clippy warnings                               | `cargo clippy --workspace`                           |
| 3  | All tests pass                                     | `cargo test --workspace`                             |
| 4  | Zero production unwrap (grep verification)         | `rg '\.unwrap\(\)' crates/ --glob '!**/tests/**'`    |
| 5  | MockEmbeddingProvider inaccessible (manual)        | External crate cannot import in release build        |

---

## 7. Implementation Sequence

```
Phase 1 ─── Security + Observability Gaps
│
├── C-1, C-2 (TOCTOU fixes) [CRITICAL]
├── C-3, C-4 (SecureClientBuilder) [CRITICAL]
├── C-5, C-6, C-7 (let _ = fixes) [HIGH]
├── S-1 (Wire agent delegates) [CRITICAL]
│
├── ▶ GATE: Phase 1 validation protocol
│
Phase 2 ─── Concurrency + Feature Stubs
│
├── H-2, H-4 (type swaps)
├── H-1, H-5 (DashMap migrations)
├── H-3 (Memory partitioned locking)
├── H-6 (Remaining let _ =)
├── S-2 (Memory consolidation)
├── S-4 (NLP command dispatchers)
│
├── ▶ GATE: Phase 2 validation protocol
│
Phase 3 ─── Cleanup
│
├── M-1 (unwrap cleanup)
├── M-2 (channel event bus)
├── M-3, S-5, S-6, S-7, S-8 (low cleanup)
├── S-3 (entropy culling)
│
├── ▶ GATE: Phase 3 validation protocol
│
└── CERTIFIED: FID closed
```

---

## 8. Risk Register

| Risk                                              | Probability | Impact   | Mitigation                                    |
|---------------------------------------------------|-------------|----------|-----------------------------------------------|
| C-4 breaks existing HTTP calls                    | Medium      | `HIGH`   | Test each crate's HTTP calls after migration  |
| S-1 LLM wiring introduces regressions             | Medium      | `CRITICAL`| Incremental wiring with per-delegate tests    |
| H-3 memory partitioning causes data races         | Low         | `HIGH`   | Extensive concurrent write testing            |
| S-4 NLP wiring changes command semantics          | Low         | `MEDIUM` | Preserve existing help_text, add new tests    |

---

## 9. Notes

- **Python audit value:** The Python pipeline was useful for bulk pattern detection (1,492 raw violations). It could not perform semantic analysis — it found `reqwest::Client::new()` but couldn't determine which were test code vs production. It found `.unwrap()` but couldn't determine which were in `#[cfg(test)]` blocks.
- **Manual scan value:** The manual scan identified 14 stub/placeholder findings the Python pipeline structurally cannot detect — functions that compile and run but return empty/fake results. These are the highest-impact findings (S-1 is the single most critical issue).
- **CortexaDB:** `lib/cortexadb/` contains ~150 unwrap/expect calls. These are in a vendored library. Do not patch locally — file upstream issues instead.
- **Highest-impact single fix:** C-4 (`SecureClientBuilder`) closes SSRF across 58 call sites in one change.
- **Highest-impact functional fix:** S-1 (wire agent delegates) — without this, the agent literally cannot respond to users.
- **Duplicate fix matrices removed:** Previous iteration had two conflicting fix matrices. This version has a single consolidated matrix in Section 5.
