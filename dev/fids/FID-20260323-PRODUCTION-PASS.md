# FID-20260323-PRODUCTION-PASS

**Date:** 2026-03-23
**Status:** PLANNING
**Protocol:** Perfection Loop + Checkpoint Gates (brain surgery protocol)
**Source:** Full project audit (AUDIT-REPORT.md) — ~250 issues across 16 crates
**Standard:** $1M+ enterprise valuation — zero tolerance for stubs, data loss, security gaps
**Perfection Loop:** Iteration 1 complete — restructured for file-grouped execution, merged dependent fixes, added missing items

---

## CRITICAL RULES FOR THIS FID

1. **Brain Surgery Protocol:** Every file is interconnected. One fix in file A can break logic in file Z. Before ANY change:
   - Read the target file 0-EOF
   - Read ALL files that import from or are imported by the target
   - Trace the data flow end-to-end
   - Present the impact matrix to Spencer BEFORE making the change
   - Get approval → Make change → Verify → Checkpoint

2. **Checkpoint Gates:** After EVERY fix group:
   - `cargo check --workspace` must pass with 0 errors
   - For frontend changes: `npx tsc --noEmit` must pass
   - Present what changed and what could be affected
   - Spencer approves before proceeding to next group

3. **No Autonomous Changes:** If anything is unclear, STOP. Get clarity.

4. **Read Before Touch:** Every file read 0-EOF before any edit. No exceptions.

5. **Git Safety:** Work on a branch. Commit after each phase checkpoint. Rollback available.

---

## PHASE 0: DECISION GATES (Spencer decides BEFORE any code changes)

Before any implementation begins, Spencer must decide on all stubs and removals.

| # | Item | Option A: Implement | Option B: Remove/Disable | Decision |
|---|------|-------------------|-------------------------|----------|
| 0.1 | Web tool (navigate/snapshot/scrape) | Implement real browser automation | Remove tool from registry | **IMPLEMENT** |
| 0.2 | Web projection (hardcoded DOM) | Implement real DOM projection | Remove tool from registry | **IMPLEMENT** |
| 0.3 | PromotionEngine | Integrate with memory engine | Remove module | **IMPLEMENT** |
| 0.4 | Consolidation LLM summary | Call LLM for summarization | Disable, keep raw messages only | **IMPLEMENT** |
| 0.5 | Nostr adapter | Implement secp256k1 signing | Remove adapter + lib.rs declaration | **IMPLEMENT** |
| 0.6 | X/Twitter adapter | Fix API endpoints | Remove adapter + lib.rs declaration | **IMPLEMENT** |
| 0.7 | Feishu adapter | Fix container_id polling | Remove adapter + lib.rs declaration | **IMPLEMENT** |
| 0.8 | JWT secret missing | Skip distillation pipeline | Error on missing config | **ERROR ON MISSING** |

---

## FIX MATRIX (File-Grouped Execution)

Fixes are grouped by target file to minimize reads and catch intra-file interactions.

### Phase 1: Memory Crate — Data Integrity (5 fixes, 3 files)

**Files:** `memory/async_backend.rs`, `memory/lsm_engine.rs`, `memory/engine.rs`, `core/db.rs`

| # | Severity | Issue | File | Line | Fix | Cross-Impact |
|---|----------|-------|------|------|-----|-------------|
| 1.1 | CRITICAL | MemoryEntry ID uses string length — all same-length IDs collide | `memory/async_backend.rs` | 94 | Use blake3 hash of (session_id + msg_id) as u64 ID | LSM engine stores by ID, vector engine indexes by ID, enclave orchestrates both |
| 1.2 | CRITICAL | Atomic compact deletes before insert — no rollback | `memory/lsm_engine.rs` | 335-388 | Insert new batch FIRST, then delete old entries. If insert fails, no data lost. | Agent loop triggers compaction via enclave. Any session using compaction affected. |
| 1.3 | CRITICAL | VECTOR_DIM hardcoded at 384 in TWO files (leftover from dimension fix) | `memory/lsm_engine.rs` | 27 | Read dimension from config or use Ollama's 2560 | Both LSM and core DB create indices at wrong dimension. |
| 1.3b | CRITICAL | Same VECTOR_DIM issue | `core/db.rs` | 15 | Same fix — dynamic dimension | Core DB creates indices at wrong dimension. |
| 1.4 | CRITICAL | Hardcoded JWT secret "default_secret" | `memory/engine.rs` | 290-292 | If no jwt_secret configured, skip spawning distillation pipeline with warning log | Distillation pipeline stops. Config needs jwt_secret field or pipeline gracefully disabled. |
| 1.5 | HIGH | Temporal entity search ignores `_entity_name` parameter | `memory/lsm_engine.rs` | 554-577 | Filter results by entity_name in the query | Temporal metadata lookups return correct results. |

**Cross-Impact for Phase 1:**
```
MemoryEntry ID → LsmStorageEngine, VectorEngine, MemoryEnclave, AgentLoop, Gateway
Atomic Compact → MemoryEnclave.compact(), AgentLoop compaction, AsyncBackend.consolidate()
VECTOR_DIM → CortexaDB::open() in both lsm_engine and db.rs, vector index creation
JWT Secret → Distillation pipeline, Arbiter
Temporal Search → MemoryEnclave methods, any temporal query caller
```

**CHECKPOINT 1:** `cargo check --workspace` + `cargo test -p savant_memory` + Spencer approval

---

### Phase 2: Agent Loop — Critical Bugs (4 fixes, 1 file)

**Files:** `agent/react/stream.rs`

These 4 issues are all in the same function/loop — fixing them together prevents conflicting edits.

| # | Severity | Issue | File | Line | Fix | Cross-Impact |
|---|----------|-------|------|------|-----|-------------|
| 2.1 | CRITICAL | `turn_failed` never set to true | `agent/react/stream.rs` | 130, 637 | Set `turn_failed = true` when tool execution errors or LLM errors occur | TurnState phase tracking, SessionState, gateway telemetry |
| 2.2 | CRITICAL | Excluded tools computed but never used | `agent/react/stream.rs` | 406 | Pass `excluded_tools` into tool execution (reactor) | SelfRepair system becomes active, ToolHealthTracker affects execution |
| 2.3 | CRITICAL | Context compaction at 50% capacity (128K vs 256K budget) | `agent/react/stream.rs` | 139 | Use `TokenBudget` capacity (256,000) for ContextMonitor | Compaction triggers later, more context preserved |
| 2.4 | HIGH | Session save errors silently ignored | `agent/react/stream.rs` | 110, 120, 651, 655 | Log `tracing::warn` on save failure + increment error counter | Agent loop error visibility |

**Cross-Impact for Phase 2:**
```
turn_failed → TurnState (models.rs), SessionState persistence, gateway turn events
Excluded Tools → self_repair.rs ToolHealthTracker, reactor.rs tool execution
Context Budget → compaction.rs ContextMonitor + Compactor, react/mod.rs TokenBudget
Session Saves → memory backend, session persistence
```

**CHECKPOINT 2:** `cargo check --workspace` + `cargo test -p savant_agent` + Spencer approval

---

### Phase 3: Gateway — Security + Error Handling (8 fixes, 3 files)

**Files:** `gateway/auth/mod.rs`, `gateway/server.rs`, `gateway/handlers/mod.rs`, `gateway/handlers/skills.rs`

| # | Severity | Issue | File | Line | Fix | Cross-Impact |
|---|----------|-------|------|------|-----|-------------|
| 3.1 | CRITICAL | Dashboard shared session ID | `gateway/auth/mod.rs` | 57 | Generate `format!("dashboard-{}", uuid::Uuid::new_v4())` per connection | All dashboard state, WebSocket routing, telemetry delivery |
| 3.2 | HIGH | CORS `*` on all responses | `gateway/server.rs` | 390 | Use `state.config.server.allowed_origins` (already in config) | Dashboard connectivity — verify frontend origin matches |
| 3.3 | HIGH | Non-constant-time API key comparison | `gateway/server.rs` | 139 | Use `subtle::ConstantTimeEq` or manual constant-time compare | Gateway auth security |
| 3.4 | MEDIUM | WebSocket only handles Text frames | `gateway/server.rs` | 136 | Handle Ping→Pong, Close→disconnect, Binary→ignore+log | WebSocket reliability |
| 3.5 | MEDIUM | Config re-read from disk on every update | `gateway/server.rs` | 536 | Use in-memory `state.config` + persist after mutation | Config consistency under concurrency |
| 3.6 | MEDIUM | Gateway persistence fire-and-forget | `gateway/server.rs` | 250, 271 | Log `tracing::warn` on persistence failure | Message delivery reliability |
| 3.7 | HIGH | Prune-before-append message loss | `gateway/handlers/mod.rs` | 49 | Move prune AFTER successful append | Message ordering — prune only committed messages |
| 3.8 | HIGH | Path traversal bypass in skills | `gateway/handlers/skills.rs` | 392 | Canonicalize BOTH paths before comparison; reject if canonicalize fails | Skills handler security |
| 3.8b | HIGH | Skill responses broadcast to all sessions | `gateway/handlers/skills.rs` | 509 | Use `send_control_response` with specific session_id | Skill response delivery — no more data leak |

**Cross-Impact for Phase 3:**
```
Dashboard Session → WebSocket handler, State.sessions DashMap, telemetry routing
CORS → Dashboard must match configured origin
API Key Comparison → All authenticated endpoints
WebSocket Frames → Connection lifecycle
Config → All config-dependent handlers
Prune Order → Message persistence reliability
Path Traversal → Skills file operations
Skill Responses → All skill management endpoints
```

**CHECKPOINT 3:** `cargo check --workspace` + Spencer approval

---

### Phase 4: Agent Tools — Security (1 fix)

**Files:** `agent/tools/shell.rs`

| # | Severity | Issue | File | Line | Fix | Cross-Impact |
|---|----------|-------|------|------|-----|-------------|
| 4.1 | HIGH | Shell tool no cwd sandboxing | `agent/tools/shell.rs` | 96-109 | Validate `cwd` is within workspace bounds; reject absolute paths outside workspace | Shell tool execution — agents can no longer escape workspace |

**Cross-Impact for Phase 4:**
```
Shell cwd → Agent workspace isolation, tool execution sandbox
```

**CHECKPOINT 4:** `cargo check --workspace` + `cargo test -p savant_agent` + Spencer approval

---

### Phase 5: Agent Error Recovery (1 fix)

**Files:** `agent/react/reactor.rs`

| # | Severity | Issue | File | Line | Fix | Cross-Impact |
|---|----------|-------|------|------|-----|-------------|
| 5.1 | HIGH | No-op rollback in heuristic recovery | `agent/react/reactor.rs` | 143 | Implement: truncate `full_trace` to checkpoint length, restore previous messages | Agent error recovery — actually recovers instead of compounding errors |

**Cross-Impact for Phase 5:**
```
Rollback → Agent loop message history, full_trace accumulation, heuristic state
```

**CHECKPOINT 5:** `cargo check --workspace` + Spencer approval

---

### Phase 6: Stub Implementation (ALL decided IMPLEMENT)

All stubs decided for implementation per Phase 0.

| # | Severity | Issue | File | Action |
|---|----------|-------|------|--------|
| 6.1 | HIGH | Nostr unsigned events | `channels/nostr.rs` | Implement secp256k1 event signing |
| 6.2 | HIGH | X/Twitter invalid endpoints | `channels/x.rs` | Fix DM API endpoints |
| 6.3 | HIGH | Feishu empty container_id | `channels/feishu.rs` | Fix polling with actual container_id |
| 6.4 | MEDIUM | Web tool stubs | `agent/tools/web.rs` | Implement real navigate/snapshot/scrape |
| 6.5 | MEDIUM | Web projection stub | `agent/tools/web_projection.rs` | Implement real DOM projection |
| 6.6 | MEDIUM | PromotionEngine unused | `memory/promotion.rs` | Integrate with memory engine |
| 6.7 | HIGH | Consolidation placeholder | `memory/async_backend.rs:239` | Implement LLM summarization |
| 6.8 | CRITICAL | JWT secret default | `memory/engine.rs:290` | Error on missing jwt_secret config |

**CHECKPOINT 6:** `cargo check --workspace` + Spencer approval

---

### Phase 7: Channel Adapters — Fixes (5 fixes)

All adapters kept — these fixes apply to all.

| # | Severity | Issue | File | Line | Fix | Cross-Impact |
|---|----------|-------|------|------|-----|-------------|
| 7.1 | HIGH | Blocking sleep in async context | `channels/email.rs` | 438 | `tokio::time::sleep(Duration::from_secs(30)).await` | Email channel — no longer blocks Tokio runtime |
| 7.2 | HIGH | Bluesky cross-channel echo | `channels/bluesky.rs` | 111-117 | Change `\|\|` to `&&` in filter — require BOTH recipient check AND role check | Bluesky message routing — stops echoing other channels |
| 7.3 | MEDIUM | Bluesky silent login failure | `channels/bluesky.rs` | 56-58 | Return `Err` if JWT/DID fields missing from response | Bluesky auth — fails loudly instead of silently |
| 7.4 | HIGH | Twitch no reconnection logic | `channels/twitch.rs` | 60-164 | Add exponential backoff reconnection loop (match IRC adapter pattern) | Twitch channel — survives disconnects |
| 7.5 | MEDIUM | IRC race condition in SASL | `channels/irc.rs` | 191 | Parse server responses and react instead of fixed sleep | IRC auth — reliable on all server speeds |

**CHECKPOINT 7:** `cargo check --workspace` + Spencer approval

---

### Phase 8: Dashboard Frontend (4 fixes)

| # | Severity | Issue | File | Line | Fix | Cross-Impact |
|---|----------|-------|------|------|-----|-------------|
| 8.1 | CRITICAL | Hardcoded API key in frontend | `dashboard/src/app/page.tsx` | 472 | Read from `NEXT_PUBLIC_DASHBOARD_API_KEY` env var; fail if missing | Dashboard auth flow |
| 8.2 | MEDIUM | Version mismatch (3 different versions) | `dashboard/package.json`, `tauri.conf.json`, `page.tsx` | various | Unify: `package.json` = source of truth, `tauri.conf.json` reads from it, UI reads from env/build | Build, UI, runtime version consistency |
| 8.3 | MEDIUM | Tauri API v1.x vs runtime v2.x | `dashboard/package.json` | 13 | `npm install @tauri-apps/api@^2.0.0` | Dashboard Tauri integration |
| 8.4 | MEDIUM | Hardcoded developer paths in tauri.conf.json | `tauri.conf.json` | 7-8 | Use relative paths: `"../dashboard/out"` | Desktop build portability |

**CHECKPOINT 8:** `npx tsc --noEmit` + `npm run build` (if applicable) + Spencer approval

---

### Phase 9: Configuration (2 fixes)

| # | Severity | Issue | File | Line | Fix | Cross-Impact |
|---|----------|-------|------|------|-----|-------------|
| 9.1 | MEDIUM | Placeholder updater public key | `tauri.conf.json` | 50 | Generate real keypair with `tauri signer generate` | Auto-updater — needs keypair before release |
| 9.2 | MEDIUM | Config re-read race condition | `gateway/server.rs` | 536 | Already addressed in Phase 3.5 | N/A |

**CHECKPOINT 9:** `cargo check --workspace` + Spencer approval

---

### Phase 10: Final Verification

| # | Check | Command | Status |
|---|-------|---------|--------|
| 10.1 | Workspace compiles | `cargo check --workspace` | **PENDING** |
| 10.2 | All tests pass | `cargo test --workspace` | **PENDING** |
| 10.3 | TypeScript compiles | `cd dashboard && npx tsc --noEmit` | **PENDING** |
| 10.4 | No remaining CRITICAL in audit | Manual review of AUDIT-REPORT.md | **PENDING** |
| 10.5 | IMPLEMENTATION-TRACKER.md updated | Manual update | **PENDING** |
| 10.6 | CHANGELOG-INTERNAL.md updated | Manual update | **PENDING** |
| 10.7 | Session summary written | Manual write | **PENDING** |
| 10.8 | Spencer final approval | Presentation | **PENDING** |
| 10.9 | Commit + Push | `git add -A && git commit && git push` | **PENDING** |

---

## EXECUTION ORDER

```
Phase 0: DECISION GATES (Spencer decides: implement or remove for 8 items)
  ↓ Spencer approves all decisions
Phase 1: Memory Crate (5 fixes — data integrity)
  ↓ CHECKPOINT: cargo check + cargo test -p savant_memory + Spencer approval
Phase 2: Agent Loop (4 fixes — all in stream.rs)
  ↓ CHECKPOINT: cargo check + cargo test -p savant_agent + Spencer approval
Phase 3: Gateway (8 fixes — security + error handling)
  ↓ CHECKPOINT: cargo check + Spencer approval
Phase 4: Agent Tools Security (1 fix — shell sandboxing)
  ↓ CHECKPOINT: cargo check + Spencer approval
Phase 5: Agent Error Recovery (1 fix — reactor rollback)
  ↓ CHECKPOINT: cargo check + Spencer approval
Phase 6: Channel/Tool Removals (per Phase 0 decisions)
  ↓ CHECKPOINT: cargo check + Spencer approval
Phase 7: Channel Adapter Fixes (5 fixes — for kept adapters only)
  ↓ CHECKPOINT: cargo check + Spencer approval
Phase 8: Dashboard Frontend (4 fixes)
  ↓ CHECKPOINT: npx tsc --noEmit + Spencer approval
Phase 9: Configuration (2 fixes)
  ↓ CHECKPOINT: cargo check + Spencer approval
Phase 10: Final Verification (full suite)
  ↓ Spencer final approval
  ↓ Commit + Push
```

---

## PER FIX PROTOCOL (Brain Surgery)

```
FOR EACH FIX:
  1. PRESENT to Spencer:
     - Issue description
     - File + line
     - Proposed fix (exact code change)
     - Cross-impact analysis (what else touches this code?)
     - Risk assessment
     - What could break if this is wrong?

  2. WAIT for Spencer approval

  3. READ target file 0-EOF
  4. READ all files that import from / export to the target
  5. TRACE data flow: where does the data come from, where does it go?

  6. MAKE the fix

  7. VERIFY:
     - cargo check --workspace (0 errors)
     - For frontend: npx tsc --noEmit (0 errors)
     - Manual review of change against cross-impact map

  8. CHECKPOINT with Spencer:
     - Show exact diff of what changed
     - Confirm no regressions in cross-impacted files
     - Get approval for next fix
```

---

## SUCCESS CRITERIA

- [ ] Phase 0 decisions made by Spencer for all 8 items
- [ ] All 5 CRITICAL memory bugs fixed (Phase 1)
- [ ] All 4 CRITICAL agent loop bugs fixed (Phase 2)
- [ ] All 8 gateway security/error bugs fixed (Phase 3)
- [ ] Shell tool cwd sandboxed (Phase 4)
- [ ] Heuristic recovery rollback implemented (Phase 5)
- [ ] All decided removals executed cleanly (Phase 6)
- [ ] All kept adapters fixed (Phase 7)
- [ ] Dashboard frontend issues resolved (Phase 8)
- [ ] Config issues resolved (Phase 9)
- [ ] `cargo check --workspace` — 0 errors
- [ ] `cargo test --workspace` — all passing
- [ ] `npx tsc --noEmit` — 0 errors
- [ ] IMPLEMENTATION-TRACKER.md updated
- [ ] CHANGELOG-INTERNAL.md updated
- [ ] Session summary written

---

## CROSS-IMPACT MAP

```
MemoryEntry ID (async_backend.rs:94)
  ├── LsmStorageEngine (lsm_engine.rs) — stores/retrieves by ID
  ├── VectorEngine (vector_engine.rs) — indexes by ID
  ├── MemoryEnclave (engine.rs) — orchestrates both
  ├── AgentLoop (stream.rs) — calls enclave methods
  └── Gateway handlers — reads from storage

Atomic Compact (lsm_engine.rs:335-388)
  ├── MemoryEnclave.compact() — calls lsm.compact_session
  ├── AgentLoop compaction check — triggers compaction
  └── AsyncBackend.consolidate() — calls compact

VECTOR_DIM (lsm_engine.rs:27 + core/db.rs:15)
  ├── CortexaDB::open() — creates index at wrong dimension
  ├── zero_embedding() — creates zero vectors at wrong dimension
  └── Must match Ollama embedding output (2560 for qwen3-embedding:4b)

JWT Secret (engine.rs:290)
  ├── Distillation pipeline — signs triplets
  ├── Arbiter — reads signed triplets
  └── Config — needs jwt_secret field or pipeline gracefully disabled

turn_failed (stream.rs:130)
  ├── TurnState (models.rs) — phase tracking
  ├── SessionState — turn count, active turn
  └── Gateway telemetry — turn status events

Excluded Tools (stream.rs:406)
  ├── SelfRepair (self_repair.rs) — tracks tool health
  ├── ToolHealthTracker — determines excluded list
  └── Reactor (reactor.rs) — executes tools

Context Budget (stream.rs:139)
  ├── ContextMonitor (compaction.rs) — triggers compaction
  ├── TokenBudget (react/mod.rs) — defines capacity
  └── Compactor — performs compaction

Dashboard Session (auth/mod.rs:57)
  ├── WebSocket handler — manages connections
  ├── State.sessions (DashMap) — stores per-session state
  └── Telemetry routing — sends to specific session

Shell cwd (tools/shell.rs:96-109)
  ├── Agent workspace isolation
  └── Tool execution sandbox
```

---

## RISK REGISTER

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| VECTOR_DIM fix breaks existing DB indices | Medium | High | Delete and recreate vector indices after fix |
| Dashboard session ID change breaks WebSocket reconnect | Medium | Medium | Test reconnect flow after change |
| CORS restriction breaks dashboard in dev mode | Medium | High | Allow localhost origins in dev config |
| Removing adapters breaks compilation | Low | Medium | Remove module declarations in lib.rs simultaneously |
| Consolidation disabled loses context | Low | Medium | Keep raw messages; compaction just trims oldest |
| turn_failed change triggers false failure states | Low | Medium | Only set on actual error conditions, not edge cases |

---

## NOTES

- This FID covers ~30 individual fixes across 10 phases
- Each phase has a CHECKPOINT requiring Spencer's approval
- Fixes are file-grouped to minimize reads and catch intra-file interactions
- Phase 0 decisions MUST be made before any implementation
- The audit report (`dev/AUDIT-REPORT.md`) has full details on all ~250 issues
- MEDIUM/LOW items not in this FID are deferred to a future cleanup pass
- Git branch: create `production-pass-20260323` before starting

---

## PERFECTION LOOP ITERATIONS

| Iteration | Changes |
|-----------|---------|
| 1 | Initial FID created — 45 fixes, 9 phases |
| 2 | Restructured: file-grouped execution, merged stream.rs fixes, added Phase 0 decisions, added VECTOR_DIM leftover fix, added git branch strategy, added risk register, reordered phases (decisions first, removals consolidated, fixes only for kept items) |

---

*FID certified via Perfection Loop (Iteration 2). Follow brain surgery protocol. No autonomous changes.*
