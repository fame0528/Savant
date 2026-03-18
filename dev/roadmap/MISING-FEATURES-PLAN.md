# Savant Missing Features - Implementation Plan

**Created:** 2026-03-18  
**Source:** Post-audit deep analysis of 133 source files  
**Status:** Planning — Perfection Loop reviewed  
**Total Features:** 14  
**Total Estimated Effort:** 44-58 hours

---

## Architecture Decisions

### Embedding Service
The `fastembed` crate uses `TextEmbedding` which is NOT `Send`. This means:
- Cannot use `spawn_blocking` with embeddings
- Must create one `TextEmbedding` instance per blocking thread
- OR: Use a message-passing architecture where a dedicated thread handles embedding requests

**Decision:** Create `EmbeddingService` with a dedicated embedding thread + channel:
- Input: `(String, oneshot::Sender<Vec<f32>>)` 
- Thread owns `TextEmbedding`, processes requests
- Returns embeddings via oneshot channel

### Dedup Strategy
Use `blake3` (already in workspace) for message hashing. Store hashes in a separate Fjall keyspace `msg_hashes:{agent_id}` with TTL-based eviction (Fjall's built-in TTL).

---

## Phase 1: Core Features (10-13 hours)

### 1. Vector Search / Semantic Memory (CRITICAL) — 4-6 hrs

**Current:** `retrieve()` does substring match, ignores `SemanticVectorEngine`  
**Impact:** Agents can't semantically search past conversations

**Implementation:**
1. Create `crates/memory/src/embedding_service.rs` — dedicated embedding thread with channel
2. Modify `async_backend.rs::store()` — generate embedding, store in vector engine
3. Modify `async_backend.rs::retrieve()` — if query non-empty:
   - Send to embedding thread, get embedding back
   - Call `vector.search(query_embedding, limit * 2)`
   - Map vector results to LSM messages
   - Fall back to substring match if vector returns nothing
4. Add `vector_ids` field to `AgentMessage` for vector-to-message mapping

**Files:** `crates/memory/src/embedding_service.rs` (new), `async_backend.rs`, `engine.rs`, `vector_engine.rs`

**Tests required:**
- [ ] Store 100 messages, search by content, verify relevant results
- [ ] Fallback to substring when vector search returns empty
- [ ] Embedding service handles concurrent requests
- [ ] Embedding cache hit reduces latency

---

### 2. Token Auto-Rotation (CRITICAL) — 3-4 hrs

**Current:** Tokens minted once at spawn, never rotated  
**Impact:** Compromised token remains valid until agent restart

**Implementation:**
1. Add to `AgentToken`:
   ```rust
   pub issued_at: u64,
   pub ttl_secs: u64,
   ```
2. Add method: `should_rotate(&self) -> bool` (80% TTL elapsed)
3. In `swarm.rs` heartbeat loop: check `token.should_rotate()`, mint new token
4. Send new token via `sync_delta` broadcast
5. Agent receives token update in reactor, swaps atomically

**Files:** `crates/security/src/enclave.rs`, `crates/agent/src/swarm.rs`, `crates/agent/src/react/reactor.rs`

**Tests required:**
- [ ] Token rotates after 80% TTL elapsed
- [ ] Token does NOT rotate before 80% TTL
- [ ] New token accepted by reactor
- [ ] Old token rejected after rotation

---

### 3. Crash Recovery Verification (CRITICAL) — 2-3 hrs

**Current:** WAL exists, no test verifies crash survival  
**Impact:** Unknown data loss risk on power failure

**Implementation:** Create `crates/memory/tests/crash_recovery.rs`:
1. Write 1000 messages → close engine → reopen → verify all present
2. Write 500 → drop engine (no close) → reopen → verify
3. Write mixed roles → reopen → verify role preservation
4. Write during consolidation → verify both old and new messages present

**Tests required:**
- [ ] 1000 messages survive graceful restart
- [ ] 500 messages survive crash (drop without close)
- [ ] Message ordering preserved across restart
- [ ] Consolidation doesn't lose data after restart

---

## Phase 2: Integration Features (15-18 hours)

### 4. MCP Client Tool Discovery (HIGH) — 6-8 hrs

**Current:** Client exists but can't discover external MCP servers  
**Impact:** Can't use tools from Claude Desktop, VS Code, etc.

**Implementation:**
1. Add `tokio-tungstenite` to mcp Cargo.toml
2. Implement `connect(url, auth_token) -> Result<McpClient>`
3. Send `initialize` with capabilities, receive server capabilities
4. Implement `list_tools()` → calls `tools/list`
5. Implement `call_tool(name, args)` → calls `tools/call`  
6. Register discovered tools in agent's tool registry via `ToolBridge`
7. Add circuit breaker around external calls

**Files:** `crates/mcp/src/client.rs`, `crates/agent/src/tools/mod.rs`

**Tests required:**
- [ ] Client connects to mock MCP server
- [ ] Client discovers tools via `tools/list`
- [ ] Client calls tool via `tools/call`
- [ ] Circuit breaker trips on repeated failures
- [ ] Auth failure returns clear error

---

### 5. Docker Skill Execution (HIGH) — 3-4 hrs

**Current:** Container launches but skill code doesn't actually run  
**Impact:** Docker sandbox security works but doesn't execute skills

**Implementation:**
1. Read SKILL.md from container filesystem
2. Pass skill payload via stdin (SAVANT_INPUT env var for backward compat)
3. Execute: `sh -c "cat /dev/stdin | node skill.js"` or `python3 skill.py`
4. Capture stdout as result
5. SIGKILL on timeout (already done)
6. Log metrics: start time, end time, memory usage, exit code

**Files:** `crates/skills/src/docker.rs`

**Tests required:**
- [ ] Docker container executes `echo test` successfully
- [ ] Container times out after 30s
- [ ] Stdout captured as skill result
- [ ] Stderr logged but not returned
- [ ] Container cleaned up after timeout

---

### 6. WASM Skill Sandboxing (HIGH) — 6-8 hrs

**Current:** Stubs only — WASM skills don't run  
**Impact:** Secure execution path is non-functional

**Implementation:**
1. WASM modules loaded via `wasmtime` (already in workspace)
2. WASI context: preopen empty dir, no network, fuel limit
3. Pass payload via WASI stdin
4. Collect result from WASI stdout
5. Fuel exhaustion = timeout
6. Clean up instance on drop

**Files:** `crates/skills/src/sandbox/wasm.rs` (rewrite), `crates/skills/src/wasm/mod.rs`

**Tests required:**
- [ ] WASM module executes and returns result
- [ ] WASM timeout after fuel exhaustion
- [ ] WASM cannot access filesystem outside preopen
- [ ] WASM cannot make network requests

---

## Phase 3: Stability (5-8 hours)

### 7. Message Deduplication (HIGH) — 2-3 hrs

**Implementation:**
1. Add `msg_hash_window` keyspace to `Storage` (Fjall)
2. Before insert in `append_chat`, compute blake3 hash of content
3. Check if hash exists in window → skip if duplicate
4. Add hash to window, evict oldest if window > 100
5. `dedup_window: usize` parameter (default 100, 0 = disabled)

**Files:** `crates/core/src/db.rs`

**Tests:**
- [ ] Duplicate message rejected on second insert
- [ ] Different messages both inserted
- [ ] After 100 unique messages, oldest hash evicted
- [ ] `dedup_window=0` disables dedup

---

### 8. Telegram Graceful Disconnect (MEDIUM) — 1-2 hrs

**Implementation:**
1. Wrap main receive loop in `loop`
2. On error, log + sleep with exponential backoff (1s → 60s max)
3. After 10 consecutive failures, log `error!` and break
4. Preserve update offset across reconnections

**Files:** `crates/channels/src/telegram.rs`

---

### 9. WhatsApp Sidecar Health (MEDIUM) — 2-3 hrs

**Implementation:**
1. Add `health_task: Option<JoinHandle<()>>` to WhatsAppAdapter
2. Spawn task that checks `child_process.try_wait()` every 30s
3. If process dead: log warning, attempt restart
4. Max 3 restart attempts, then log error and disable channel

**Files:** `crates/channels/src/whatsapp.rs`

---

## Phase 4: Polish (6-9 hours)

### 10. Dashboard WebSocket Reconnection (MEDIUM) — 1-2 hrs

**Implementation:**
1. Replace `setInterval` reconnect with `onclose` + backoff
2. Exponential backoff: 1s, 2s, 4s, max 30s
3. "Reconnecting..." indicator in UI
4. Re-subscribe to lanes on reconnect

**Files:** `dashboard/src/app/page.tsx`

---

### 11. Skill Testing CLI (MEDIUM) — 2-3 hrs

**Implementation:**
1. `savant-cli test-skill <path>` subcommand
2. Parse SKILL.md → validate format
3. Run security scanner on skill directory
4. Execute with test payload
5. Compare against expected output in `## Test` section
6. Report pass/fail

**Files:** `crates/skills/src/testing.rs` (new), `crates/cli/src/main.rs`

---

### 12. Fjall Backup/Restore (MEDIUM) — 3-4 hrs

**Implementation:**
1. `LsmStorageEngine::backup(&self, dest: &Path)` — iterate all keyspaces, copy entries
2. Open new Fjall instance at dest, insert all entries
3. `LsmStorageEngine::restore(src: &Path)` — load from backup path
4. Validate integrity before swap
5. Atomic swap via temp directory

**Files:** `crates/memory/src/lsm_engine.rs`

---

## Phase 5: Future (8-10 hours)

### 13. Proactive Learning (HIGH) — 4-5 hrs

**Implementation:**
1. `PerceptionEvent` struct with timestamp + observation
2. `VecDeque<PerceptionEvent>` sliding window (max 100)
3. Pattern detection: count modifications per file in window
4. Adaptive thresholds based on signal-to-noise ratio
5. Report patterns: "File X modified N times in window"

**Files:** `crates/agent/src/proactive/perception.rs`, `crates/agent/src/proactive/mod.rs`

---

### 14. Lambda Executor (LOW) — 4-5 hrs

**Implementation:**
1. `lambda_runtime` crate integration
2. Parse Lambda event → skill payload
3. Execute via Docker/WASM sandbox
4. Return Lambda response
5. Handle cold starts

**Files:** `crates/skills/src/lambda.rs` (new)

---

## Phase Order

| Phase | Features | Effort |
|-------|----------|--------|
| **1: Core** | Vector search (1), Token rotation (2), Crash recovery (3) | 10-13 hrs |
| **2: Integration** | MCP client (4), Docker skills (5), WASM skills (6) | 15-18 hrs |
| **3: Stability** | Dedup (7), Telegram reconnect (8), WhatsApp watchdog (9) | 5-8 hrs |
| **4: Polish** | Dashboard reconnect (10), Skill testing (11), Fjall backup (12) | 6-9 hrs |
| **5: Future** | Proactive learning (13), Lambda executor (14) | 8-10 hrs |

**Total: 44-58 hours of focused development**

---

## Dependencies Already in Workspace

All required dependencies are already in `Cargo.toml`:
- `fastembed` — embedding generation (vector search)
- `wasmtime` — WASM skill execution
- `tokio-tungstenite` — WebSocket client (MCP)
- `blake3` — message deduplication
- `tokio` — async runtime with `spawn_blocking`
- `serde_json` — JSON serialization
- `tracing` — structured logging
