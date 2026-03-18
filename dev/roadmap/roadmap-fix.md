# Savant Development Roadmap

**Created:** 2026-03-18  
**Source:** `dev/PENDING.md` (archived audit → new work items)  
**Standard:** AAA quality. Zero bloat. Production-ready.  
**Baseline:** 107/121 audit issues fixed. 157 tests passing. Clean compilation.

---

## Phase 1: Docker Sandbox Verification (HIGH RISK)

**Goal:** Ensure the Docker sandbox actually works end-to-end  
**Risk:** HIGH — primary untrusted code execution path  
**Files:** `crates/skills/src/docker.rs`, `crates/skills/src/sandbox.rs`

| # | Task | File | Status |
|---|------|------|--------|
| DS-001 | Verify bollard 0.16 API compatibility | `crates/skills/src/docker.rs` | PENDING |
| DS-002 | Test container create → start → wait → logs → cleanup lifecycle | `crates/skills/src/docker.rs:48-160` | PENDING |
| DS-003 | Verify 512MB memory limit enforcement | `crates/skills/src/docker.rs:60` | PENDING |
| DS-004 | Verify 1 CPU (nano_cpus=1B) enforcement | `crates/skills/src/docker.rs:61` | PENDING |
| DS-005 | Test network isolation (`network_mode: none`) | `crates/skills/src/docker.rs:59` | PENDING |
| DS-006 | Test SIGKILL timeout after 30s | `crates/skills/src/docker.rs:83-116` | PENDING |
| DS-007 | Test Windows named pipe Docker Desktop integration | `crates/skills/src/docker.rs` | PENDING |
| DS-008 | Verify stdout/stderr streaming from container | `crates/skills/src/docker.rs:119-136` | PENDING |
| DS-009 | Test container cleanup on process crash | `crates/skills/src/docker.rs:141-157` | PENDING |
| DS-010 | Add read-only rootfs config | `crates/skills/src/docker.rs:59` | PENDING |
| DS-011 | Add volume mount security (no path traversal) | `crates/skills/src/docker.rs` | PENDING |
| DS-012 | Add Docker connection health check on startup | `crates/skills/src/docker.rs:18-23` | PENDING |

### Implementation Notes:
- Current code at line 59 has `network_mode` commented out — needs to be `"none"`
- Current code at line 59 has `read_only_rootfs` missing — needs to be `Some(true)`
- Host config should include `security_opt: Some(vec!["no-new-privileges:true"])`
- Container should have `user: Some("nobody")` for non-root execution
- Volume mounts should be explicit allowlist only

---

## Phase 2: Nix Sandbox — Windows Decision (MEDIUM RISK)

**Goal:** Decide Windows support strategy, implement accordingly  
**Risk:** MEDIUM  
**Files:** `crates/skills/src/nix.rs`

| # | Task | File | Status |
|---|------|------|--------|
| NX-001 | Add `#[cfg(windows)]` stub returning clear error | `crates/skills/src/nix.rs:1-30` | PENDING |
| NX-002 | Verify path canonicalization works on Linux | `crates/skills/src/nix.rs:117-135` | PENDING |
| NX-003 | Test flake reference validation for all 5 prefixes | `crates/skills/src/nix.rs:80-115` | PENDING |
| NX-004 | Verify 10KB payload limit enforcement | `crates/skills/src/nix.rs:200-250` | PENDING |
| NX-005 | Document WSL2 detection approach if pursued | `docs/ops/DEPLOYMENT_CHECKLIST.md` | PENDING |

### Implementation Notes:
- Add at top of nix.rs:
```rust
#[cfg(windows)]
pub async fn execute_skill(&self, ...) -> Result<String, SavantError> {
    Err(SavantError::Unsupported("Nix sandbox requires Linux. On Windows, use Docker sandbox or WSL2.".to_string()))
}
```

---

## Phase 3: MCP End-to-End Testing (MEDIUM RISK)

**Goal:** Verify MCP auth, rate limiting, circuit breaker all work  
**Risk:** MEDIUM  
**Files:** `crates/mcp/src/server.rs`, `crates/mcp/src/client.rs`, `crates/mcp/src/circuit.rs`

| # | Task | File | Status |
|---|------|------|--------|
| MCP-001 | Write integration test: initialize with valid token | `crates/mcp/src/server.rs` | PENDING |
| MCP-002 | Write integration test: initialize without token (auth required) | `crates/mcp/src/server.rs` | PENDING |
| MCP-003 | Write integration test: initialize with invalid token | `crates/mcp/src/server.rs` | PENDING |
| MCP-004 | Write integration test: rate limit after 100 requests | `crates/mcp/src/server.rs` | PENDING |
| MCP-005 | Write integration test: tools/list returns available tools | `crates/mcp/src/server.rs` | PENDING |
| MCP-006 | Write integration test: tools/call with valid/invalid tool | `crates/mcp/src/server.rs` | PENDING |
| MCP-007 | Verify circuit breaker state transitions | `crates/mcp/src/circuit.rs` | PENDING |
| MCP-008 | Verify `auth_tokens` HashMap population from config | `crates/mcp/src/server.rs` | PENDING |

---

## Phase 4: Memory System — Stress Testing (MEDIUM RISK)

**Goal:** Verify data integrity under concurrent load  
**Risk:** MEDIUM — data corruption under load  
**Files:** `crates/memory/src/*.rs`

| # | Task | File | Status |
|---|------|------|--------|
| MEM-001 | Test concurrent writes (100 writers, same session) | `crates/memory/src/engine.rs` | PENDING |
| MEM-002 | Test concurrent writes (100 writers, different sessions) | `crates/memory/src/engine.rs` | PENDING |
| MEM-003 | Test consolidation with 10,000 messages | `crates/memory/src/async_backend.rs` | PENDING |
| MEM-004 | Test vector persistence: write → drop → reload → verify | `crates/memory/src/vector_engine.rs` | PENDING |
| MEM-005 | Test atomic delete cascade: engine → LSM → vectors | `crates/memory/src/engine.rs:237-251` | PENDING |
| MEM-006 | Test query filtering in retrieve() | `crates/memory/src/async_backend.rs:90-98` | PENDING |
| MEM-007 | Verify Drop impl fires on vector engine | `crates/memory/src/vector_engine.rs` | PENDING |
| MEM-008 | Stress test: write 10K messages → consolidate → verify summary | `crates/memory/src/async_backend.rs` | PENDING |

---

## Phase 5: Gateway Security — Penetration Testing (HIGH RISK)

**Goal:** Verify all security fixes hold under adversarial testing  
**Risk:** HIGH  
**Files:** `crates/gateway/src/**/*.rs`

| # | Task | File | Status |
|---|------|------|--------|
| SEC-001 | Test malformed Ed25519 tokens in pairing | `crates/gateway/src/handlers/pairing.rs` | PENDING |
| SEC-002 | Test expired token rejection | `crates/gateway/src/auth/mod.rs` | PENDING |
| SEC-003 | Test path traversal attempts in skill_name | `crates/gateway/src/handlers/skills.rs:50-57` | PENDING |
| SEC-004 | Test directive injection with control characters | `crates/gateway/src/lanes.rs:48-72` | PENDING |
| SEC-005 | Test WebSocket origin validation | `crates/gateway/src/server.rs` | PENDING |
| SEC-006 | Verify no internal error details leak to client | `crates/gateway/src/server.rs:102` | PENDING |
| SEC-007 | Test CORS headers for strictness | `crates/gateway/src/server.rs` | PENDING |
| SEC-008 | Test DoS via large skill packages | `crates/gateway/src/handlers/skills.rs` | PENDING |
| SEC-009 | Verify SSRF protection in threat intel | `crates/skills/src/security.rs:141` | PENDING |

---

## Phase 6: ECHO Protocol Verification (MEDIUM RISK)

**Goal:** Verify speculative ReAct loop and circuit breaker  
**Files:** `crates/echo/src/*.rs`

| # | Task | File | Status |
|---|------|------|--------|
| ECH-001 | Test circuit breaker CAS transitions under load | `crates/echo/src/circuit_breaker.rs` | PENDING |
| ECH-002 | Test Mutex protection prevents TOCTOU | `crates/echo/src/circuit_breaker.rs` | PENDING |
| ECH-003 | Verify env filtering removes AWS secrets | `crates/echo/src/compiler.rs` | PENDING |
| ECH-004 | Test speculative execution rollback | `crates/echo/src/` | PENDING |

---

## Phase 7: Dashboard UI/UX (LOW RISK)

**Goal:** Verify dashboard flows work correctly  
**Files:** `dashboard/src/app/page.tsx`, `dashboard/src/app/globals.css`

| # | Task | File | Status |
|---|------|------|--------|
| DSH-001 | Test WebSocket connection and reconnection | `dashboard/src/app/page.tsx` | PENDING |
| DSH-002 | Test skill install approval UI flow | `dashboard/src/app/page.tsx` | PENDING |
| DSH-003 | Verify security scan results display | `dashboard/src/app/page.tsx` | PENDING |
| DSH-004 | Test responsive design | `dashboard/src/app/globals.css` | PENDING |

---

## Phase 8: Threat Intelligence Feed (MEDIUM RISK)

**Goal:** Set up real threat intel endpoint or mock  
**Files:** `crates/skills/src/security.rs`

| # | Task | File | Status |
|---|------|------|--------|
| THR-001 | Determine endpoint URL (or build mock) | `crates/skills/src/security.rs:141` | PENDING |
| THR-002 | Implement periodic sync (cron/scheduler) | `crates/skills/src/security.rs` | PENDING |
| THR-003 | Test sync with mock server | `crates/skills/src/security.rs` | PENDING |
| THR-004 | Implement webhook push for real-time updates | `crates/skills/src/security.rs` | PENDING |

---

## Phase 9: Remaining Audit Issues (from 121)

**Status:** 107/121 fixed, 14 remaining  
**Priority:** All medium/low

| # | Phase | ID | File | Issue | Fix | Status |
|---|-------|-----|------|-------|-----|--------|
| 1 | 3 | L-026 | `crates/skills/src/docker.rs` | Unused `error` import | Remove line 1 | PENDING |
| 2 | 4 | H-023 | `crates/core/src/fs/mod.rs:142-145` | `semantic_search` stub returns empty | Implement using `full_text_search` or return `Err(Unsupported)` | ✅ FIXED |
| 3 | 4 | M-012 | `crates/core/src/fs/mod.rs:62-83` | Blocking I/O in async `index_directory` | Wrap `WalkDir` and `read_to_string` in `tokio::task::spawn_blocking` | ✅ FIXED |
| 4 | 4 | M-013 | `crates/core/src/fs/mod.rs:90-94` | New SQLite `Connection::open` per `index_file` call | Pass `&Connection` parameter or use connection pool | ✅ FIXED |
| 5 | 6 | M-030 | `crates/agent/src/tools/mod.rs` | Input filtering for cognitive events | N/A — `emit_cognitive_event` doesn't exist | N/A |
| 6 | 9 | M-024 | `crates/channels/src/discord.rs` | Channel resource leak | Store `cancellation_token: Arc<CancellationToken>` in struct, pass to spawned tasks | PENDING |
| 7 | 9 | M-025 | `crates/channels/src/whatsapp.rs` | WhatsApp child process Drop | Store `child: Option<Child>` and `reader_handle: Option<JoinHandle>`, implement `Drop` | PENDING |
| 8 | 11 | L-022 | `crates/core/src/utils/embeddings.rs:27` | Embedding service blocks async | Documented: TextEmbedding non-Send. Cache provides hit rate optimization. | DOCUMENTED |
| 9 | 13 | A-001 | `core/src/db.rs`, `memory/src/lsm_engine.rs` | Three separate Fjall instances | Document the separation (already done), no consolidation needed | DOCUMENTED |
| 10 | 13 | A-003 | Multiple crates | Error type proliferation | Add `From` impls to `SavantError` for common error types | PENDING |
| 11 | 13 | A-004 | `gateway/src/handlers/mod.rs:332` | Global mutable state for API keys | Move to `GatewayState` struct | PENDING |
| 12 | 14 | L-001 | `gateway/src/lib.rs`, `agent/src/lib.rs`, `cli/src/main.rs` | Blanket clippy suppress | Replace `#![allow(clippy::disallowed_methods)]` with specific suppressions | PENDING |
| 13 | 3 | L-006 | `crates/skills/src/security.rs` | `is_blocked` docs misleading | N/A — field removed in refactor | N/A |
| 14 | 3 | L-004/L-009 | `crates/skills/src/clawhub.rs` | Custom URL encoder / SSRF method | Already uses `urlencoding` crate, `with_base_urls` is `#[cfg(test)]` | FIXED |

### Implementation Details:

**H-023 — semantic_search stub** (`crates/core/src/fs/mod.rs:142-145`):
```rust
// Current: returns empty vec
pub fn semantic_search(&self, _query: &str, _limit: usize) -> Vec<MemoryEntry> {
    Vec::new()
}

// Fix: delegate to full_text_search
pub fn semantic_search(&self, query: &str, limit: usize) -> Vec<MemoryEntry> {
    self.full_text_search(query).into_iter().take(limit).collect()
}
```

**M-012 — Blocking I/O in async** (`crates/core/src/fs/mod.rs:62-83`):
```rust
// Wrap in spawn_blocking
pub async fn index_directory(&self, agent_id: &str, base_path: &Path) -> Result<(), SavantError> {
    let agent_id = agent_id.to_string();
    let base_path = base_path.to_path_buf();
    let indexer = self.clone();
    tokio::task::spawn_blocking(move || {
        // existing WalkDir logic here
    }).await.map_err(|e| SavantError::Unknown(format!("Task join error: {}", e)))?
}
```

**L-022 — Embedding blocks async** (`crates/core/src/utils/embeddings.rs:27`):
```rust
// Wrap model.embed() in spawn_blocking
pub async fn embed(&self, text: &str) -> Result<Vec<f32>, SavantError> {
    // Cache check stays async (just a mutex lock)
    // Model inference goes to blocking thread
    let text = text.to_string();
    let model = self.model.clone();
    tokio::task::spawn_blocking(move || {
        let mut m = model.lock().map_err(|_| "Lock poisoned")?;
        m.embed(vec![&text], None)
    }).await...?
}
```

---

## Phase 10: Cross-Platform Testing

| # | Task | Platform | Status |
|---|------|----------|--------|
| XP-001 | Run full test suite | Linux Ubuntu 22.04 | PENDING |
| XP-002 | Run full test suite | macOS Apple Silicon | PENDING |
| XP-003 | Verify Docker integration | Linux | PENDING |
| XP-004 | Test WASM execution | All platforms | PENDING |
| XP-005 | Verify signal handling (SIGKILL/SIGTERM) | Linux/macOS | PENDING |

---

## Phase 11: Performance Profiling

| # | Task | Tool | Status |
|---|------|------|--------|
| PERF-001 | Gateway WebSocket throughput | `cargo flamegraph` | PENDING |
| PERF-002 | Skill execution latency (Docker/Native/WASM) | Tracing spans | PENDING |
| PERF-003 | Memory engine benchmark (10K entries) | `cargo bench` | PENDING |
| PERF-004 | Vector search latency (100K embeddings) | `cargo bench` | PENDING |
| PERF-005 | Cognitive synthesis CPU profile | `cargo flamegraph` | PENDING |
| PERF-006 | Memory usage under load | `valgrind` / `heaptrack` | PENDING |

---

## Phase 12: Documentation

| # | Task | File | Status |
|---|------|------|--------|
| DOC-001 | Create example skill package | `skills/example-skill/` | PENDING |
| DOC-002 | Write ClawHub publishing tutorial | `docs/tutorials/clawhub-publishing.md` | PENDING |
| DOC-003 | Write Docker sandbox setup tutorial | `docs/tutorials/docker-sandbox.md` | PENDING |
| DOC-004 | Write troubleshooting guide | `docs/ops/TROUBLESHOOTING.md` | PENDING |
| DOC-005 | Write contributing guidelines | `CONTRIBUTING.md` | PENDING |

---

## Phase 13: CI/CD Pipeline

| # | Task | File | Status |
|---|------|------|--------|
| CI-001 | Create GitHub Actions workflow | `.github/workflows/ci.yml` | PENDING |
| CI-002 | Add `cargo check` on PR | `.github/workflows/ci.yml` | PENDING |
| CI-003 | Add `cargo test` on PR | `.github/workflows/ci.yml` | PENDING |
| CI-004 | Add `cargo clippy` on PR | `.github/workflows/ci.yml` | PENDING |
| CI-005 | Add `cargo fmt --check` on PR | `.github/workflows/ci.yml` | PENDING |
| CI-006 | Add Dependabot config | `.github/dependabot.yml` | PENDING |
| CI-007 | Add `cargo audit` check | `.github/workflows/security.yml` | PENDING |

---

## Summary

| Phase | Category | Count | Status |
|-------|----------|-------|--------|
| 1 | Docker Sandbox | 12 | PENDING |
| 2 | Nix Sandbox | 5 | PENDING |
| 3 | MCP Testing | 8 | PENDING |
| 4 | Memory Stress | 8 | PENDING |
| 5 | Gateway Security | 9 | PENDING |
| 6 | ECHO Verification | 4 | PENDING |
| 7 | Dashboard UI/UX | 4 | PENDING |
| 8 | Threat Intelligence | 4 | PENDING |
| 9 | Remaining Audit Issues | 14 | 3 N/A, 5 DONE, 6 PENDING |
| 10 | Cross-Platform | 5 | PENDING |
| 11 | Performance | 6 | PENDING |
| 12 | Documentation | 5 | PENDING |
| 13 | CI/CD | 7 | PENDING |
| **TOTAL** | | **91** | **0 / 91** |
