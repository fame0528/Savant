# Internal Changelog

> **Purpose:** Detailed project changelog for agents. More detailed than root CHANGELOG.md.
> **Updated:** As work happens, not just at release time.
> **Last Cleaned:** 2026-05-31 (v0.4.1)

---

## [v0.4.1] — 2026-05-31

### 2026-05-31: Deep Audit Remediation — 41 Issues Fixed (FID-20260531-AUDIT-REMEDIATION)

**FID:** `FID-20260531-AUDIT-REMEDIATION.md` (CLOSED)

**Problem:** Deep audit of 4 subsystems against 3 reference repos (agent-vault, zot, agentmemory) found 41 issues — including 5 critical bugs where functionality was defined, tested, and documented but never wired into the runtime.

**Fixes (41 issues across 5 phases):**

Phase A — Critical Bugs (5): Reranker content fix, BM25 persistence, approval gate wiring, ToolFilter wiring, SkillLookupTool.

Phase B — Memory System (10): Auto-recall hybrid search, graph search ranked matching, temporal decay, Ebbinghaus tier lifecycle, consolidation scheduler, procedures/lessons/insights persistence, tier migration, complementary re-embedding, Shannon entropy, entropy culling.

Phase C — Tool Execution (8): auto_approved/denied enforcement, CCT no-token deny, Savant bypass removal, file tool scanning, BeforeToolCall hook, external data taint, network threat intel, tool schemas in prompt.

Phase D — Session & Persistence (10): Per-message persistence, orphan cleanup, streaming persistence, ContextCompressor LLM call, archived text persistence, compact/ cleanup, circuit breaker persistence, session TTL, dedup window (as-is), session forking.

Phase E — Skills (8): Security scanning, double path fix, SkillChainExecutor, hot reload, Lambda SigV4, CapabilityGrants enforcement, output sanitization.

**28 commits. 1,265 tests pass.**

### 2026-05-31: Enhancement + Stub Completion — 7 Items (FID-20260531-ENHANCEMENTS-AND-STUBS)

**FID:** `FID-20260531-ENHANCEMENTS-AND-STUBS.md` (CLOSED)

**Fixes (7 items):**

Stubs (3): expire_stale_sessions() actually deletes, cleanup_orphaned_turns() actually scans, fork_session() copies message history.

Enhancements (4): Cross-encoder reranking architecture (feature-gated ONNX), structured checkpoint format (6 sections), per-tool approval memory helpers, cross-ecosystem skill discovery (.claude/.agents/.opencode).

**5 commits.**

### Version Bump

- 0.4.0 → 0.4.1 via `scripts/bump-version.ps1`
- All 98 FIDs closed. 0 active.
- 1,265 tests pass.

---

## [v0.4.0] — 2026-05-30

### 2026-05-30: Dashboard Response Pipeline — 5 Issues Fixed (FID-20260530-DASHBOARD-RESPONSE-PIPELINE)

**FID:** `FID-20260530-DASHBOARD-RESPONSE-PIPELINE.md` (archived)

**Problem:** Dashboard non-functional for chat. Agent processes (LLM streaming visible) but dashboard shows TIMEOUT on every message. User resends repeatedly, creating duplicates. Agent image shows fallback "S". Copy All broken. Governor bounces to CRITICAL on transient CPU spikes.

**Root cause:** No interim telemetry between "message sent" and "response published". The 30s timeout fires before the agent finishes processing.

**Fix (5 issues):**

1. **Agent image** — `AuthImage` component silently failed on auth mismatch. Added fallback to direct `<img>` when auth fetch fails. (`DashboardShell.tsx:14-25`)
1. **Copy All** — All 3 clipboard fallback paths (Tauri plugin, navigator.clipboard, execCommand) caught errors silently. Added error logging to each path. (`tauri.ts:120-148`)
1. **Message timeout** — Published interim `chat.chunk` telemetry before LLM call in BOTH code paths (heartbeat `process_user_message` AND orchestrator `execute_turn`). Increased timeout from 30s to 120s. (`heartbeat.rs`, `DashboardContext.tsx`)
1. **Governor bouncing** — `PressureLevel::from_metrics()` used instantaneous CPU/memory with no smoothing. Added EMA (Exponential Moving Average) with configurable alpha (default 0.7). Transient spikes absorbed within ~3 monitor cycles (15s). (`monitor.rs`, `pressure.rs`, `config.rs`)
1. **Duplicate messages** — Gateway had no dedup. Added blake3 content-hash dedup with 10s TTL in `handlers/mod.rs`. Batch prune on every insert. (`handlers/mod.rs`, `server.rs`)

**New files:** None (all changes to existing files)

**Tests:** 340/340 pass, 0 clippy, 0 TS errors, 0 markdownlint

### 2026-05-30: Two-Tier Agent System, DelegationEngine, 10 Bug Fixes (FID-20260530-AGENT-TIER-REDESIGN)

**FID:** `FID-20260530-AGENT-TIER-REDESIGN.md` (archived)

**Problem:** "Agent" meant two different things (full workspace agent vs fire-and-forget sub-agent) with no shared type system, no lifecycle model, no governance. Sub-agents bypassed the governor entirely. 10 pre-existing bugs in orchestration, governor, and swarm code.

**Fix (37 steps across 3 phases):**

Phase A — Bug Fixes (10/10):

- `pop_deferred()` duplicate spawn → remove re-push, add drain loop
- Blackboard clobbering → task-specific hashes
- CCT minting ×3 → consolidated `mint_subagent_cct()`
- No subagent count limit → `max_subagents_per_agent` check
- `handle.abort()` only → 10s drain timeout
- Errors swallowed → `JoinHandle<Result<(), String>>`
- Speculative delegation sequential → `join_all` parallel
- Agent index race → `compare_exchange` loop
- Per-agent registry → shared at Swarm level
- Naive DELEGATE: parser → structured JSON blocks

Phase B — New Architecture (15/15):

- `AgentTier` (Full/SubAgent), `AgentRole` (Main/Orchestrator/Leaf)
- `SubAgentProfile` with SOUL.md, tool restrictions, iteration budget
- `DelegationEngine` with routing, hooks, caching, events
- `SubAgentRegistry` with IterationBudget, cancel-by-parent
- `SubAgentSemaphore` for tier-aware governor gating
- 6 specialized profiles (coding, documentation, research, testing, orchestrator, general)
- `AgentFileLock`, `LoopDetector`, `ToolFilter`, `WorkspaceGuard`
- `MemoryEnclaveHandle` read-only wrapper
- `CancellationToken` in AgentLoop tool execution
- 5 integration tests

Phase C — Gap Fixes (12/12):

- Wire `AgentConfig.parent_id`, replace `orchestrator_enabled` with `AgentTier`
- `AgentLimits` config struct, `SubAgentEvent` observability
- `ResultCache` with 5min TTL

**New files:** 9 modules, 25 tests, 18 profile files (6 profiles × 3 files each)

### 2026-05-30: WAL Frontmatter Redesign (FID-20260530-SESSION-STATE-WAL-ENTERPRISE)

**FID:** `FID-20260530-SESSION-STATE-WAL-ENTERPRISE.md` (archived)

**Problem:** `DEV-SESSION-STATE.md` WAL writer outputs minified JSON wrapped in decorative markdown. Reader uses fragile brace-finding. No schema versioning.

**Fix:**

- `commit_state()` → YAML frontmatter + structured markdown sections
- `restore_state()` → frontmatter parser with legacy JSON fallback
- `WorkingBuffer` → `schema_version: u32` field added
- CLI `state --inspect` → colored frontmatter parsing + section display
- 6 unit tests for roundtrip, legacy, braces, missing file, schema default

### 2026-05-30: Markdown Zero Defect (FID-20260529-MARKDOWN-ZERO-DEFECT)

**FID:** `FID-20260529-MARKDOWN-ZERO-DEFECT.md` (archived)

**Problem:** 8,501 markdownlint violations across 300 files (29 rules).

**Fix:**

- Auto-fix pass: 7,126 violations (MD032, MD022, MD012, MD009, MD047, MD058, MD055, MD004, MD007, MD030, MD056)
- Manual fixes: 1,375 violations (MD040, MD024, MD036, MD029, MD025, MD026, MD001, MD041)
- 0 violations remaining

### 2026-05-30: Workspace and Repo Cleanup

- SOUL.md rewritten as enterprise persona specification
- 6 profile SOUL.md files rewritten to enterprise quality (coding profile references ECHO-UNIFIED.md)
- AGENTS.md updated with two-tier architecture
- IGNITION.md rewritten: v16.2 → v0.4.0
- README.md updated with two-tier agent system and DelegationEngine
- CHANGELOG.md updated with v0.4.0 entry
- Bloat removed: build.log, firebase-debug.log, FormattedContent.tsx.tmp, logs/*.log
- Stale files archived: AUDIT-v0.3.2.md, agent-orchestrator.yaml, diagnostic_run.md
- Duplicate image removed: img/security_gate - Copy.png
- Root package.json version fixed: 0.1.0 → 0.4.0
- Governor defaults updated: 16/8/4/1 → 128/64/32/8
- CapabilityRegistry: per-agent → shared at Swarm level

### 2026-05-29: Messaging Pipeline Hardening + Message Status UX (FID-20260529-MESSAGING-AND-SCALING)

**FID:** `FID-20260529-MESSAGING-AND-SCALING.md` (archived)

**Problem:** Agent silently drops messages at 9 failure points with zero user feedback. Message status indicator only has 3 active states, no error/timeout handling.

**Fix (26 items across 5 categories):**

Category A — Silent Drops (A1-A4):

- `heartbeat.rs`: Identity pinning drops logged with 80-char content preview
- `lanes.rs`: Error `ResponseFrame` sent on lane timeout
- `server.rs`: `session.mismatch` event + error on oversized message

Category B — Error Pipeline (B1-B3):

- `heartbeat.rs`: Agent loop errors publish `is_error: true` response
- `server.rs`: Task supervisor with panic detection + `tokio::select!` complete arm

Category C — Agent Availability (C1-C2):

- `swarm.rs`: `system.agent.ready` event published after boot
- `handlers/mod.rs`: Agent presence check in `route_chat_message()`

Category D — Diagnostics (D1-D3):

- Structured tracing at agent inbound, WS receipt, and Nexus outbound

Category E — Message Status UX (E-1 through E-11):

- 10-state `MessageStatus` type replacing 3-state system
- Gateway ACK event with frontend handler
- Telemetry-driven `thinking`/`executing` granularity
- Dual timeout timers (60s ACK + 30s response)
- Inline RETRY/DISMISS/WAIT recovery buttons
- 10-state status dot with CSS variables, animations, aria-label
