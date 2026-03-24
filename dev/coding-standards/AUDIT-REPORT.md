# MASTER AUDIT REPORT — Savant Framework
**Date:** 2026-03-23  
**Scope:** Full project-wide audit (all crates, dashboard, desktop)  
**Standard:** Enterprise-grade, $1M+ valuation  
**Method:** Every file read 0-EOF, logic analysis, pattern detection  

---

## EXECUTIVE SUMMARY

**Total Issues Found: ~250+**  
- **Critical Bugs: 15** — Data corruption, data loss, security vulnerabilities  
- **High Severity: 25** — Race conditions, broken features, runtime panics  
- **Medium Severity: 80+** — Sub-optimal logic, missing error handling, hardcoded values  
- **Low Severity: 60+** — Non-enterprise patterns, dead code, inconsistencies  
- **Stubs/Dead Code: 20+** — Non-functional implementations masquerading as working features  

**Verdict:** The codebase has solid architectural foundations but contains critical bugs that would cause data corruption and loss in production. Error handling is largely `let _ =` fire-and-forget. Many channel adapters are stubs. The gateway has pervasive error swallowing. Immediate focus needed on data integrity, error handling, and stub elimination.

---

## CRITICAL BUGS (Must Fix Immediately)

### 1. DATA CORRUPTION: MemoryEntry ID Collision
**File:** `crates/memory/src/async_backend.rs:94`  
**Issue:** `id: (msg_id.len() as u64).into()` — Uses string LENGTH as numeric ID. All UUIDs (36 chars) get the same ID, causing every subsequent memory entry to overwrite the previous one.  
**Impact:** Silent data loss — only the last memory entry per ID-length survives.  
**Fix:** Use a hash of the content or UUID-based ID generation.

### 2. DATA LOSS: Atomic Compact Deletes Before Insert
**File:** `crates/memory/src/lsm_engine.rs:335-388`  
**Issue:** `atomic_compact` deletes ALL existing entries for a session BEFORE inserting the new batch. If `add_batch` fails, all session data is permanently lost. No transactional rollback.  
**Impact:** Complete session history loss on any compaction failure.  
**Fix:** Insert first, then delete old entries, or use a transactional wrapper.

### 3. SECURITY: Hardcoded JWT Secret
**File:** `crates/memory/src/engine.rs:290-292`  
**Issue:** `unwrap_or_else(|| "default_secret".to_string())` — If no JWT secret is configured, all distilled knowledge triplets are signed with a publicly visible secret.  
**Impact:** Anyone can forge memory entries.  
**Fix:** Fail loudly if no secret is configured; never use a default.

### 4. SECURITY: Master Key Leaked to Agents
**File:** `crates/agent/src/swarm.rs:289`  
**Issue:** When derivative key creation fails, the master OpenRouter API key is used directly as the agent's API key.  
**Impact:** Individual agents have access to the master billing key.  
**Fix:** Fail the agent startup; never fall back to master key.

### 5. BUG: `turn_failed` Never Set to True
**File:** `crates/agent/src/react/stream.rs:130`  
**Issue:** `let turn_failed = false;` is declared and never updated. Used at line 637 to determine `TurnPhase::Failed`. Failed turns are always reported as `Completed`.  
**Impact:** Monitoring shows 100% success rate; errors are invisible.  
**Fix:** Set `turn_failed = true` when tool execution or LLM calls fail.

### 6. BUG: Excluded Tools Computed But Never Used
**File:** `crates/agent/src/react/stream.rs:406`  
**Issue:** `let _excluded_tools = self.self_repair.get_excluded_tools().await;` — The excluded tools list (tools marked broken by self-repair) is retrieved and discarded. Tools marked as broken are never filtered.  
**Impact:** The entire self-repair / ToolHealthTracker system is non-functional.  
**Fix:** Pass excluded tools to the tool execution phase.

### 7. BUG: Context Compaction at 50% Capacity
**File:** `crates/agent/src/react/stream.rs:139`  
**Issue:** `ContextMonitor::new(128_000)` but TokenBudget is 256,000. Compaction triggers at ~50% of actual capacity, causing premature context loss.  
**Impact:** Half the context window is wasted; conversations are compacted unnecessarily early.  
**Fix:** Use the same budget value (256,000) for the monitor.

### 8. BUG: Temporal Entity Search Ignores Parameter
**File:** `crates/memory/src/lsm_engine.rs:554-577`  
**Issue:** `find_active_temporal_by_entity` takes `_entity_name` (unused parameter) and scans ALL temporal entries, returning all active ones regardless of entity name.  
**Impact:** Incorrect temporal lookups; returns unrelated facts.  
**Fix:** Filter by entity_name in the query.

### 9. BUG: Vector Recall Hardcodes k=100
**File:** `crates/memory/src/vector_engine.rs:575`  
**Issue:** `recall_within_distance` hardcodes `k: 100`. Results beyond position 100 are silently excluded even if within `max_distance`.  
**Impact:** Semantic search silently drops valid results for large datasets.  
**Fix:** Use a large k or implement range-based filtering.

### 10. SECURITY: Dashboard Shared Session ID
**File:** `crates/gateway/src/auth/mod.rs:57`  
**Issue:** All dashboard users share `SessionId("dashboard-session")`. Multiple connections overwrite each other's state.  
**Impact:** Session collision; telemetry routed incorrectly; state corruption.  
**Fix:** Generate unique session IDs per connection.

### 11. SECURITY: Hardcoded API Key in Frontend
**File:** `dashboard/src/app/page.tsx:472`  
**Issue:** `"DASHBOARD_API_KEY:savant-dev-key"` sent over WebSocket in plaintext.  
**Impact:** Anyone monitoring traffic can capture the API key.  
**Fix:** Use environment-based auth; never hardcode keys in frontend code.

### 12. BUG: Blocking Sleep in Async Context
**File:** `crates/channels/src/email.rs:438`  
**Issue:** `std::thread::sleep(Duration::from_secs(30))` inside an async function. Blocks the Tokio worker thread for 30 seconds.  
**Impact:** Runtime stalling; other tasks on the same worker are starved.  
**Fix:** Use `tokio::time::sleep`.

### 13. BUG: Cross-Channel Echo in Bluesky
**File:** `crates/channels/src/bluesky.rs:111-117`  
**Issue:** `||` instead of `&&` in filter — sends ALL Assistant-role messages to Bluesky regardless of channel.  
**Impact:** Messages from other channels (Discord, Telegram) echoed to Bluesky.  
**Fix:** Use `&&` to check both recipient and role.

### 14. BUG: Consolidation Produces Placeholder Summary
**File:** `crates/memory/src/async_backend.rs:239`  
**Issue:** `let summary = "Conversation summary of older messages"` — Hardcoded placeholder instead of LLM-generated summary.  
**Impact:** Compaction destroys context with meaningless placeholder text.  
**Fix:** Integrate actual LLM summarization or remove the feature.

### 15. BUG: No-Op Rollback in Heuristic Recovery
**File:** `crates/agent/src/react/reactor.rs:143`  
**Issue:** `last_stable_checkpoint.take()` consumes the checkpoint but never actually rolls back. The comment says "In a real loop, we'd reset the 'history' here".  
**Impact:** Recovery mechanism is a no-op; errors compound without correction.  
**Fix:** Implement actual history rollback or remove the mechanism.

---

## HIGH SEVERITY ISSUES

### Race Conditions
| File | Line | Issue |
|------|------|-------|
| `memory/engine.rs` | 216-225 | `get_or_create_session_state`: read without lock, then conditional write — concurrent callers can both create state |
| `channels/irc.rs` | 191 | Fixed 500ms sleep during SASL negotiation instead of waiting for server responses |
| `gateway/server.rs` | 536 | Config re-read from disk on every settings update; concurrent updates can be lost |

### Security Issues
| File | Line | Issue |
|------|------|-------|
| `agent/tools/shell.rs` | 71-93 | Incomplete destructive command detection; `rm -r -f` bypasses `rm -rf` check |
| `agent/tools/shell.rs` | 96-109 | No `cwd` sandboxing; agents can execute in any directory |
| `agent/tools/web.rs` | 119-122 | Wildcard network capability `["*"]` — unrestricted internet access |
| `gateway/server.rs` | 390 | CORS `*` on all responses |
| `gateway/server.rs` | 139 | API key compared with non-constant-time equality |
| `gateway/handlers/skills.rs` | 392 | Path traversal bypass via non-canonical fallback |
| `gateway/handlers/skills.rs` | 509 | Skill responses broadcast to ALL sessions (data leak) |
| `gateway/auth/mod.rs` | 58 | Zero-filled public key used as sentinel |
| `mcp/server.rs` | 198-203 | `DefaultHasher` for auth token hashing (not cryptographic) |

### Runtime Panics
| File | Line | Issue |
|------|------|------|
| `desktop/main.rs` | 194 | `unwrap()` on `default_window_icon()` — panics if no icon configured |
| `desktop/main.rs` | 218 | `.expect()` in production code |
| `agent/react/reactor.rs` | 172-179 | Byte-based truncation may split UTF-8 characters, causing panic |
| `memory/daily_log.rs` | 131 | `content[content.len() - 2000..]` may panic on multi-byte UTF-8 boundary |
| `cognitive/predictor.rs` | 98 | `unreachable!()` in error path — will panic if validation changes |
| `cli/main.rs` | 737 | `unwrap()` on `file_name()` — panics on `..` paths |

### Non-Functional Adapters
| File | Issue |
|------|------|
| `channels/nostr.rs` | Events are unsigned (`"sig": ""`); ALL relays reject them |
| `channels/x.rs` | Invalid Twitter API v2 DM endpoints (always 404) |
| `channels/feishu.rs` | Empty `container_id` in poll request; polling is non-functional |
| `channels/twitch.rs` | No reconnection logic; permanent failure on disconnect |
| `agent/tools/web.rs` | Navigate/snapshot/scrape are all stubs returning fake data |
| `agent/tools/web_projection.rs` | DOM projection is hardcoded (always returns same 2 nodes) |

---

## MEDIUM SEVERITY ISSUES

### Missing Error Handling (`let _ =` Pattern)
The codebase has **100+ instances** of `let _ =` silently discarding errors. Key locations:
- `crates/agent/src/react/stream.rs` — Session saves (lines 110, 120, 651, 655)
- `crates/gateway/src/server.rs` — Persistence writes, event sends
- `crates/gateway/src/handlers/mod.rs` — 40+ response sends
- `crates/gateway/src/handlers/skills.rs` — 15+ response sends
- `crates/channels/` — Error discards in all adapters

### Hardcoded Values That Should Be Configurable
| File | Line | Value |
|------|------|-------|
| `agent/react/mod.rs` | 215 | Token budget: 256,000 |
| `agent/react/stream.rs` | 139 | Compaction monitor: 128,000 (MISMATCH) |
| `agent/react/stream.rs` | 151 | Compaction keep_recent: 10 |
| `agent/react/mod.rs` | 230-231 | Max parallel tools: 5, max iterations: 10 |
| `agent/swarm.rs` | 606 | Token TTL: 24 hours |
| `agent/swarm.rs` | 664 | Shutdown timeout: 12 seconds |
| `memory/lsm_engine.rs` | 27 | Vector dim: 384 (should use config) |
| `memory/lsm_engine.rs` | 197 | Message size limit: 10MB |
| `gateway/handlers/mod.rs` | 692 | max_tokens: 16384, temperature: 0.78 |
| `desktop/tauri.conf.json` | 50 | Placeholder public key |
| `channels/` (all) | various | Poll intervals, buffer sizes, timeouts |

### Sub-Optimal Logic
| File | Line | Issue |
|------|------|------|
| `agent/react/stream.rs` | 33-36 | Plugin prompt concatenation loses message structure (no delimiters) |
| `agent/react/stream.rs` | 376 | `MalformedMockTool` creates tool call to non-existent tool |
| `agent/providers/mod.rs` | 222-223 | All HTTP failures mapped to `AuthError` |
| `agent/providers/chain.rs` | 483-484 | Response cache materializes entire stream (negates streaming) |
| `agent/orchestration/branching.rs` | 84-124 | Non-idempotent tools executed 3x speculatively |
| `memory/async_backend.rs` | 564-565 | Token estimation uses doc ID length instead of content length |
| `memory/distillation.rs` | 93-96 | `DefaultHasher` for memory entry IDs (collision risk) |
| `memory/daily_log.rs` | 209-273 | Manual date math with leap year bug (misses century/400-year rules) |
| `memory/entities.rs` | 146-177 | Splits on `.` breaking abbreviations, URLs, decimals |
| `gateway/handlers/mod.rs` | 49 | Prune-before-append can lose messages |

### Version Mismatches
- `dashboard/package.json`: `0.0.01`
- `tauri.conf.json`: `0.0.1`  
- `dashboard/src/app/page.tsx`: `v1.6.0`
- `dashboard/package.json`: `@tauri-apps/api` `^1.6.0` (Tauri 1.x) but config uses Tauri 2.x

---

## LOW SEVERITY ISSUES

### Non-Enterprise Patterns
- **Emojis in log messages**: `gateway/handlers/mod.rs`, `gateway/server.rs`, `core/pulse/watchdog.rs`, `core/fs/registry.rs` — Break log aggregation
- **`println!` in production**: `channels/cli.rs:15` — Should use tracing
- **Blanket clippy suppression**: `channels/discord.rs:1`, `channels/email.rs:1`, `channels/slack.rs:1`, `channels/whatsapp.rs:1`
- **Debug formatting for client output**: `gateway/handlers/mod.rs:352` — `format!("{:?}"...)` exposes internal enum repr
- **Health checks return plain strings**: `gateway/server.rs:94-95` — Should return structured JSON

### Dead Code / Unused Modules
| File | Issue |
|------|------|
| `agent/streaming.rs` | Entire module unused; main loop has own parsing |
| `memory/promotion.rs` | `PromotionEngine` never instantiated |
| `memory/arbiter.rs` | `calculate_shannon_entropy_from_logprobs` never called |
| `memory/safety.rs` | `verify_memory_safety()` is a no-op |
| `memory/distillation.rs:89` | Signed JWT token generated but immediately discarded |
| `agent/react/mod.rs` | `ChatDelegate`, `HeartbeatDelegate`, `SpeculativeDelegate` are no-ops |
| `core/fs/registry.rs` | `ensure_stable_id` marked `#[allow(dead_code)]` |
| `channels/irc.rs` | `_send_privmsg` is dead code |
| `memory/lsm_engine.rs` | `find_by_key` marked `#[allow(dead_code)]` |

### Inconsistent Patterns
- **Constructor patterns**: Some adapters return `Result<Self>`, others return `Self`
- **Error types**: Some use `SavantError::Unknown`, others use specific variants
- **`unwrap_or_default()` on time**: 8+ instances across `security/`, `memory/`, `gateway/` — silently treats clock errors as epoch 0
- **Gateway URL construction**: 3 different implementations across dashboard files

---

## CRATE-BY-CRATE SUMMARY

### `crates/core` — 18 files audited
- **Status:** Mostly solid. Types and traits are well-structured.
- **Issues:** `db.rs` has hardcoded VECTOR_DIM=384 (should be dynamic). `bus.rs` uses unsafe `mlockall`. `config.rs` has config path scanning logic that's duplicated.

### `crates/memory` — 12 files audited
- **Status:** Has 3 critical bugs (ID collision, data loss on compact, hardcoded JWT secret).
- **Issues:** 6 bugs, 8 hardcoded values, 4 dead code modules, 3 stub implementations.

### `crates/agent` — 20+ files audited
- **Status:** Core loop has several critical bugs (turn_failed, excluded tools, context budget mismatch). Many tools are stubs.
- **Issues:** 8 bugs, 12+ hardcoded values, 6+ stub implementations, pervasive `let _ =` error handling.

### `crates/gateway` — 15+ files audited
- **Status:** Pervasive `let _ =` error swallowing (40+ instances). Auth stubbed. Chat routing is broadcast-only.
- **Issues:** 5 security concerns, 12+ missing error handlers, 25+ hardcoded values, 3 stub implementations.

### `crates/channels` — 27 files audited
- **Status:** 3 non-functional adapters (Nostr, X/Twitter, Feishu). 11 adapters have stub `send_event`/`handle_event` implementations. Blocking async bug in email.
- **Issues:** 16 bugs, 14 hardcoded values, 8+ stub implementations.

### `crates/mcp` — 4 files audited
- **Status:** Mostly functional. Server has security concern with DefaultHasher.
- **Issues:** 3 hardcoded values, 1 security concern, 1 silent error drop.

### `crates/skills` — 6 files audited
- **Status:** Security scanner has `is_blocked = false` (never blocks). Lambda AWS integration is stubbed. Blocking I/O in async contexts.
- **Issues:** 4 bugs, 6 hardcoded values, 1 stub implementation.

### `crates/security` — 4 files audited
- **Status:** Attestation checks return `Verified` for non-verified checks (misleading).
- **Issues:** 3 misleading implementations, 4 `unwrap_or_default()` on time.

### `crates/ipc` — 3 files audited
- **Status:** `active_sessions` always returns 0. Bounds check missing on agent_index.
- **Issues:** 2 bugs, 2 hardcoded values.

### `crates/cognitive` — 3 files audited
- **Status:** Magic constants in DSP predictor. Genetic algorithm convergence is hardcoded.
- **Issues:** 3 hardcoded values, 1 misleading error path.

### `crates/canvas` — 2 files audited
- **Status:** LCS has O(m*n) allocation with no bound check (OOM risk).
- **Issues:** 2 bugs, 1 hardcoded value.

### `crates/echo` — 4 files audited
- **Status:** Incomplete env sanitization (only 3 secrets cleared). Landlock rules too broad.
- **Issues:** 3 security concerns, 2 hardcoded values.

### `crates/panopticon` — 2 files audited
- **Status:** `Sampler::AlwaysOn` will overwhelm telemetry backends.
- **Issues:** 1 configuration issue, 1 O(n) eviction.

### `crates/cli` — 2 files audited
- **Status:** Build timestamp calculation is wrong. 6+ hardcoded paths.
- **Issues:** 1 bug, 8 hardcoded values.

### `crates/desktop` — 2 files audited
- **Status:** Hardcoded developer paths. Tauri API v1.x vs runtime v2.x mismatch. No graceful shutdown.
- **Issues:** 5 bugs, 3 hardcoded values, 1 panic risk.

### `dashboard` — 10 files audited
- **Status:** Hardcoded API key. Version mismatch (3 different versions). Memory leaks in debug logs. Stale Tauri API version.
- **Issues:** 8 bugs, 5 hardcoded values, 3 memory leaks, 1 critical security issue.

---

## RECOMMENDED FIX PRIORITY

### Phase 1: Critical Data Integrity (Immediate)
1. Fix `async_backend.rs:94` — MemoryEntry ID collision
2. Fix `lsm_engine.rs:335-388` — Atomic compact data loss
3. Remove hardcoded JWT secret in `engine.rs:290`
4. Fix `turn_failed` never being set in `stream.rs:130`
5. Wire excluded tools into execution in `stream.rs:406`
6. Fix context budget mismatch in `stream.rs:139`

### Phase 2: Security Hardening (This Week)
7. Generate unique dashboard session IDs
8. Remove hardcoded API key from frontend
9. Fix master key fallback in `swarm.rs:289`
10. Add cwd sandboxing to shell tool
11. Fix path traversal bypass in skills handler
12. Fix skill response broadcast leak

### Phase 3: Error Handling Overhaul (This Week)
13. Replace all `let _ =` with proper error propagation in gateway
14. Add error logging for session save failures in agent stream
15. Fix `email.rs` blocking sleep → `tokio::time::sleep`
16. Add proper error handling to all channel adapters

### Phase 4: Stub Elimination (Next Sprint)
17. Implement or remove: Nostr signing, X/Twitter endpoints, Feishu polling
18. Implement or remove: Web tool navigate/snapshot/scrape
19. Implement or remove: Consolidation LLM summarization
20. Implement or remove: PromotionEngine integration

### Phase 5: Configuration Hardening (Next Sprint)
21. Externalize all hardcoded values to `savant.toml`
22. Fix version mismatches across package.json, tauri.conf.json, UI
23. Update Tauri API to v2.x in package.json
24. Extract hardcoded model lists, paths, thresholds to config

### Phase 6: Code Quality (Ongoing)
25. Remove emojis from log messages
26. Delete dead code modules
27. Add proper health check responses (JSON)
28. Implement constant-time API key comparison
29. Add graceful shutdown for IgnitionService
30. Fix leap year bug in daily_log date math

---

## NOTES

- The architectural foundation (trait system, memory models, provider chain) is solid
- The OMEGA-VIII audit previously fixed 111 unwrap/expect/panic violations in core paths
- This audit found new issues primarily in: memory persistence, gateway error handling, channel adapter completeness, and configuration management
- Many "medium" issues are systemic patterns (`let _ =`, hardcoded values) that need a project-wide policy rather than individual fixes

**Report will be updated as issues are resolved.**
