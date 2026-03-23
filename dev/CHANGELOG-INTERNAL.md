# Internal Changelog

> **Purpose:** Detailed project changelog for agents. More detailed than root CHANGELOG.md.  
> **Updated:** As work happens, not just at release time.

---

## [Unreleased]

### Added

#### 2026-03-23: Full Project Audit + Production Pass FID

**Source:** Full project audit — every file read 0-EOF across 16 crates
**Method:** Perfection Loop (2 iterations on FID), Development Workflow
**Result:** Audit report with ~250 issues, Production Pass FID with 30 fixes certified

**Audit Report:**
- `dev/AUDIT-REPORT.md` — ~250 issues catalogued
- 15 Critical bugs, 25 High, 80+ Medium, 60+ Low, 20+ Stubs
- Every source file in all 16 crates + dashboard read completely

**Critical Findings:**
- MemoryEntry ID collision (async_backend.rs:94) — string length as ID, silent data loss
- Atomic compact deletes before insert (lsm_engine.rs:335) — no rollback on failure
- VECTOR_DIM=384 leftover in lsm_engine.rs:27 + core/db.rs:15 (previous fix missed 2 files)
- turn_failed never set to true (stream.rs:130) — all failures reported as successes
- Excluded tools computed but never used (stream.rs:406) — self-repair non-functional
- Dashboard shared session ID (auth/mod.rs:57) — state collision
- Hardcoded API key in frontend (page.tsx:472)
- Blocking std::thread::sleep in async context (email.rs:438)

**Production Pass FID:**
- `dev/fids/FID-20260323-PRODUCTION-PASS.md` — 30 fixes across 10 phases
- Brain surgery protocol: read 0-EOF, cross-impact analysis, Spencer approval per fix
- Checkpoint gates: cargo check after every fix, Spencer approval before next
- Phase 0: 8 implement-vs-remove decisions for stubs (all decided IMPLEMENT)
- File-grouped execution to minimize reads and catch intra-file interactions
- Risk register with 6 identified risks and mitigations
- Competitor research: 17 projects scanned for implementation approaches

#### 2026-03-23: Production Pass — Phase 1 (Memory Crate Data Integrity)

**Source:** FID-20260323-PRODUCTION-PASS Phase 1
**Result:** 5 critical data integrity fixes in memory crate

**Fix 1.1: MemoryEntry ID Collision**
- File: `crates/memory/src/async_backend.rs:94-104`
- Issue: `(msg_id.len() as u64).into()` — all same-length IDs collided
- Fix: blake3 content hash of `session_id + "|" + msg_id` as u64
- Added `blake3 = "1.5"` to `crates/memory/Cargo.toml`

**Fix 1.2: Atomic Compact Data Loss**
- File: `crates/memory/src/lsm_engine.rs:335-388`
- Issue: Delete before insert — no rollback on insert failure
- Fix: Insert new batch FIRST, then delete old entries (write-before-delete)

**Fix 1.3: VECTOR_DIM=384 Leftover**
- Files: `crates/memory/src/lsm_engine.rs:27` + `crates/core/src/db.rs:15`
- Issue: Hardcoded at 384, Ollama qwen3-embedding outputs 2560
- Fix: Configurable vector dimension via `LsmConfig.vector_dimension`, stored in `LsmStorageEngine`, used by `CortexaDB::open()` and `zero_embedding()`. Same for `core/db.rs Storage`.
- Also fixed pre-existing bug: `iter_metadata()` referenced undefined `&collection`

**Fix 1.4: JWT Secret Hardcoded Default**
- File: `crates/memory/src/engine.rs:280-310`
- Issue: `unwrap_or_else(|| "default_secret".to_string())` — publicly visible secret
- Fix: Ephemeral crypto-random secret generated at runtime via blake3(UUID + UUID + PID + timestamp). In-memory only, destroyed on process exit. Same pattern as agent keys.

**Fix 1.5: Temporal Entity Search Ignores Parameter**
- File: `crates/memory/src/lsm_engine.rs:575-598`
- Issue: `_entity_name` parameter unused — returned ALL active temporal entries
- Fix: Added `&& temporal.entity_name == entity_name` filter

#### 2026-03-23: Production Pass — Phase 2 (Agent Loop Critical Bugs)

**Source:** FID-20260323-PRODUCTION-PASS Phase 2
**Result:** 4 critical agent loop fixes + context window discovery system

**Fix 2.1: `turn_failed` Never Set to True**
- File: `crates/agent/src/react/stream.rs:130`
- Issue: `let turn_failed = false;` never updated — all failures reported as successes
- Fix: `let mut turn_failed = false;` + set `turn_failed = true` on fatal heuristic error path

**Fix 2.2: Excluded Tools Computed But Never Used**
- File: `crates/agent/src/react/stream.rs:406`
- Issue: `let _excluded_tools = ...` — self-repair system non-functional
- Fix: Pass excluded tools to tool execution; skip tools in `excluded_tools` list before matching

**Fix 2.3: Context Window Discovery-Based (Not Hardcoded)**
- Files: `crates/core/src/traits/mod.rs`, `crates/agent/src/providers/mod.rs`, `crates/agent/src/react/mod.rs`, `crates/agent/src/react/stream.rs`, `crates/agent/src/swarm.rs`
- Issue: `ContextMonitor::new(128_000)` and `TokenBudget::new(256000)` hardcoded
- Fix: Added `fn context_window(&self) -> Option<usize>` to `LlmProvider` trait (default None). OpenRouter provider fetches from `GET /api/v1/models/{model_id}` API. AgentLoop stores discovered value. ContextMonitor + TokenBudget use stored value. Compactor keep_recent scales proportionally.
- Discovery-based: different model = different context window = automatically detected
- All providers (OpenRouter, LmStudio, Local) query OpenRouter catalog for model specs

**Fix 2.4: Turn Finalization Skipped on Error Returns**
- File: `crates/agent/src/react/stream.rs` (multiple error paths)
- Issue: `return` at lines 207, 339, 577 skipped turn finalization block — turns stuck in Processing
- Fix: Added inline turn finalization (save_turn + save_session) before every `return` in error paths

#### 2026-03-23: Production Pass — Phase 3 (Gateway Security + Error Handling)

**Source:** FID-20260323-PRODUCTION-PASS Phase 3
**Result:** 8 gateway security and error handling fixes

**Fix 3.1: Dashboard Shared Session ID**
- File: `crates/gateway/src/auth/mod.rs:57`
- Issue: `SessionId("dashboard-session")` — all users shared same session, state collision
- Fix: `SessionId(format!("dash-{}", uuid::Uuid::new_v4()))` — unique per connection
- Added `uuid = "1.8"` to `gateway/Cargo.toml`

**Fix 3.2: CORS Wildcard on All Responses**
- File: `crates/gateway/src/server.rs:390, 427, 455`
- Issue: `ACCESS_CONTROL_ALLOW_ORIGIN: "*"` on manual headers in 3 responses
- Fix: Replaced manual headers with `tower_http::cors::CorsLayer` middleware
- Added `cors` feature to tower-http in `gateway/Cargo.toml`

**Fix 3.3: Non-Constant-Time API Key Comparison**
- File: `crates/gateway/src/auth/mod.rs:54`
- Issue: `key == configured_key` vulnerable to timing attacks
- Fix: Added `constant_time_eq()` function — XOR-based constant-time byte comparison

**Fix 3.4: WebSocket Only Handles Text Frames**
- File: `crates/gateway/src/server.rs:129, 321`
- Issue: Only `Message::Text` handled; Ping/Pong/Close ignored or caused silent failures
- Fix: Added explicit handling for Close, Ping (auto-pong by axum), error propagation

**Fix 3.5: Config Re-Read from Disk on Every Update**
- File: `crates/gateway/src/server.rs:536, 653`
- Issue: `Config::load()` re-read from disk on every settings update — concurrent requests race
- Fix: Use `state.config.clone()` (in-memory) instead of disk re-read

**Fix 3.6: Persistence Fire-and-Forget**
- File: `crates/gateway/src/server.rs:250, 271`
- Issue: `let _ = persist_chat(...)` silently discards persistence failures
- Fix: Changed to `if let Err(e) = ... { tracing::warn!(...) }`

**Fix 3.7: Prune-Before-Append Message Loss**
- File: `crates/gateway/src/handlers/mod.rs:49`
- Issue: `prune_history()` called BEFORE `append_chat()` — if append fails, old data already deleted
- Fix: Reversed order — append first, prune after

**Fix 3.8: Path Traversal Bypass + Skill Broadcast Leak**
- Files: `crates/gateway/src/handlers/skills.rs:392, 509`
- Issue 1: `canonicalize().unwrap_or(path.clone())` — fallback to non-canonical path bypasses traversal check
- Issue 2: `send_skill_response` published to `skills.{event}` broadcast channel — ALL sessions receive ALL responses
- Fix 1: Removed fallback — canonicalize must succeed or operation fails
- Fix 2: Publish to `session.{session_id}.{event}` — only requesting session receives response

#### 2026-03-23: Development Workflow v2.0.1 — Perfection Over Convenience Standard

**File:** `dev/DEVELOPMENT-WORKFLOW.md`
**Added:** "The Standard: Perfection Over Convenience" section
**Key points:**
- No time pressure, no speed optimization, no "simple" approaches
- Quality IS the product — other teams ship fast and fix later, we ship right
- 5-question evaluation: all cases, 1000 agents, hostile attacker, 2-year maintenance, industry standard
- Anti-patterns expanded: "good enough" is never good enough

#### 2026-03-23: Production Pass — Phase 4 (Shell Tool Enterprise Sandboxing)

**File:** `crates/agent/src/tools/shell.rs`, `crates/agent/src/tools/foundation.rs`, `crates/agent/src/swarm.rs`
**Status:** COMPLETE — 8 components implemented, `cargo check --workspace` 0 errors
**Scope:** Enterprise-grade shell tool sandboxing with workspace isolation, destructive pattern prevention, path injection detection, and audit trail

**Component 4.1: `pub(crate) secure_resolve_path`**
- File: `crates/agent/src/tools/foundation.rs:11`
- Made `secure_resolve_path` `pub(crate)` — reusable within agent crate

**Component 4.2: Workspace root on SovereignShell**
- File: `crates/agent/src/tools/shell.rs`
- `pub struct SovereignShell { workspace_root: PathBuf }` — explicit workspace boundary
- Constructor: `new(workspace_root: PathBuf)`
- Removed `Default` impl — workspace_root is required, no sensible default

**Component 4.3: Updated construction**
- File: `crates/agent/src/swarm.rs:556`
- `SovereignShell::new(agent_cfg.workspace_path.clone())` — matches FoundationTool, FileCreateTool, etc.

**Component 4.4: CWD sandboxing**
- File: `crates/agent/src/tools/shell.rs` (execute method)
- CWD resolved through `secure_resolve_path` — rejects ParentDir escape, re-roots absolute paths
- No cwd defaults to workspace root

**Component 4.5: Expanded destructive patterns (30+)**
- File: `crates/agent/src/tools/shell.rs` (DESTRUCTIVE_PATTERNS constant)
- 30+ patterns: spaced flag variants (`rm -r -f`, `rm -fr`), disk formatting (`mkfs.*`), git destruction, secure deletion, permission escalation, ownership changes, remote code execution (`curl | sh`), fork bombs, code evaluation, Python destruction

**Component 4.6: Absolute path allowlist**
- File: `crates/agent/src/tools/shell.rs` (execute method)
- SAFE_SYSTEM_DIRS: `/usr/bin`, `/usr/local/bin`, `/bin`, `/sbin`, `/usr/sbin`, `/usr/lib`, `/usr/local/lib`, `/usr/share`, `/opt`, `/var/lib`, `/tmp`
- Dangerous paths detected: `/etc/`, `/root/`, `/home/`, `/var/log/`, `/dev/`, `/proc/`, `/sys/` (Linux) and `C:\Windows\`, `C:\Users\` (Windows)

**Component 4.7: Audit logging**
- File: `crates/agent/src/tools/shell.rs` (execute method)
- Format: `[SHELL_AUDIT] decision={} cwd={} command_hash={}`
- Command hash (truncated) instead of full command — prevents credential leakage
- Every execution logged (ALLOWED or REJECTED with reason)

**Component 4.8: Pre-flight workspace verification**
- File: `crates/agent/src/tools/shell.rs` (execute method)
- Verifies workspace root exists before execution; creates if missing

#### 2026-03-21: Top 5 Competitive Features — Sovereign Audit Implementation

**Source:** Ultimate Sovereign Audit — 6 competitors, ~1,000,000 LOC scanned, ~200 features catalogued
**Method:** Perfection Loop (per feature), Development Workflow (dev/DEVELOPMENT-WORKFLOW.md)
**Result:** 5 features, ~2,110 LOC, 20+ files modified, 5 new files, 0 compilation errors

**Feature 1: Session/Thread/Turn Model (~600 LOC)**
- rkyv-serialized `SessionState` and `TurnState` in `memory/src/models.rs`
- `TurnPhase` enum: Processing, Completed, Failed, Interrupted, AwaitingApproval
- `LsmStorageEngine` 6 new methods for session/turn CRUD
- `MemoryEnclave` write-locked session operations
- `MemoryBackend` trait extended with 6 methods (all implementors updated)
- Agent loop: session init, turn tracking, tool call recording, turn finalization
- `AgentEvent::SessionStart` and `AgentEvent::TurnEnd` events
- Files: `memory/src/models.rs`, `memory/src/lsm_engine.rs`, `memory/src/engine.rs`, `memory/src/async_backend.rs`, `memory/src/lib.rs`, `core/src/types/mod.rs`, `core/src/traits/mod.rs`, `core/src/memory/mod.rs`, `agent/src/memory/mod.rs`, `agent/src/react/stream.rs`, `agent/src/react/events.rs`, `agent/src/react/heuristic_tests.rs`, `agent/src/react/mod.rs`

**Feature 2: Provider Chain (~410 LOC)**
- New file: `providers/chain.rs` with 4 components
- Error Classifier: 7 categories (Auth, RateLimit, Billing, Timeout, Format, Overloaded, Transient)
- Cooldown Tracker: exponential backoff (standard: 1min * 5^n, billing: 5h * 2^n)
- Circuit Breaker: Closed/Open/HalfOpen with configurable thresholds
- Response Cache: SHA-256 keyed, LRU eviction, TTL-based, tool calls excluded
- `ProviderChain` implements `LlmProvider` with all 4 layers
- Added `sha2 = "0.10"` dependency
- Files: `providers/chain.rs` (NEW), `providers/mod.rs`, `agent/Cargo.toml`

**Feature 3: Context Compaction (~350 LOC)**
- New file: `react/compaction.rs`
- `ContextMonitor`: usage ratio calculation, strategy selection (3 tiers)
- `Compactor`: truncate, partition, compact with system message injection
- Token estimation: word count * 1.3 + 4 overhead
- Integration: pre-LLM-call check in agent loop
- Files: `react/compaction.rs` (NEW), `react/mod.rs`, `react/stream.rs`

**Feature 4: Approval Gating (~100 LOC)**
- `ApprovalRequirement` enum on `Tool` trait: Never/Conditional/Always
- `requires_approval()` method with default Never
- Tool-level overrides: SovereignShell (Conditional), FileDelete (Always), FileMove (Conditional), FileAtomicEdit (Conditional)
- Foundation for future approval flow with user consent
- Files: `core/src/traits/mod.rs`, `tools/foundation.rs`, `tools/shell.rs`

**Feature 5: Tool Coercion + Schema Validation (~650 LOC)**
- New file: `tools/coercion.rs` — recursive argument coercion against JSON Schema
- Empty string → null, string → typed coercion, $ref resolution, oneOf/anyOf discriminators
- New file: `tools/schema_validator.rs` — strict (CI) + lenient (runtime) validation
- Integration: coercion in reactor.rs before tool execution
- Fixed pre-existing bugs in FileDeleteTool (base_path → workspace_dir, SavantError::Validation/Security → Unknown)
- Files: `tools/coercion.rs` (NEW), `tools/schema_validator.rs` (NEW), `tools/mod.rs`, `react/reactor.rs`

#### 2026-03-21: Tool System v2 (Prior Session)
- Tools sent to LLM API with `parameters_schema()` on Tool trait
- LlmProvider trait extended with `tools` parameter
- All 14 providers updated to send tools to API
- 5-format parser + JSON curly-brace Action parser
- HIDDEN_TAGS expanded for tool tag filtering
- System prompt updated with native function calling preference

#### 2026-03-21: Ultimate Sovereign Audit
- 6 exhaustive competitor scans: IronClaw (50 features), NanoBot (30+), NanoClaw (15+), OpenClaw (35+), PicoClaw (30+), ZeroClaw (40+)
- ~200 total features catalogued with file:line citations
- `dev/Master-Gap-Analysis.md` — full parity matrix
- `dev/fids/FID-20260321-SUPREME-AUDIT-SUBTASK-*.md` — 6 exhaustive FIDs

#### 2026-03-22: Smithery CLI + Dashboard (~650 LOC)
- `gateway/src/smithery.rs` — SmitheryManager wraps @smithery/cli (install/list/uninstall/info)
- `gateway/src/handlers/mcp.rs` — 6 REST endpoints for MCP management
- `dashboard/src/app/mcp/page.tsx` — MCP management UI (server list, add, install from Smithery)
- `core/src/config.rs` — McpConfig + McpServerEntry structs, `[mcp]` config section
- Config auto-updates on install/uninstall

#### 2026-03-22: Channel Expansion (25 channels built)
- 25 channels: Slack, Email, Signal, IRC, Feishu, DingTalk, WeCom, LINE, Google Chat, Teams, Mattermost, Matrix, Generic Webhook, WhatsApp Business, Bluesky, Reddit, Nostr, Twitch, Notion, Voice, X + 4 existing (Discord, Telegram, WhatsApp, CLI)
- All use same pattern: ChannelAdapter trait + spawn() + NexusBridge integration
- HTTP/API channels: Slack, Email, Feishu, DingTalk, WeCom, LINE, Google Chat, Teams, Mattermost, Webhook, WhatsApp Business, Bluesky, Reddit, Notion
- Protocol channels: Signal (SSE to signal-cli), IRC (TCP+TLS), Matrix (REST API), Nostr (WebSocket), Twitch (IRC), Voice (edge-tts/whisper), X (Twitter API v2)
- Dependencies added: imap, native-tls, lettre, mailparse, tokio-rustls, rustls, webpki-roots, chrono, tokio-tungstenite, tokio-native-tls
- Files: 25 new .rs files in crates/channels/src/

#### 2026-03-21: MCP Agent Loop Integration

**Perfection Loop: 2 iterations. Gap: MCP client existed but never wired into agent loop.**

- `McpConfig` + `McpServerEntry` in `core/src/config.rs` — `[mcp]` config section
- `Config.mcp` field with `servers: Vec<McpServerEntry>`
- `SwarmController.mcp_servers` — threaded through `new()` → `spawn_agent()`
- MCP discovery in `spawn_agent()`: connects to all configured servers, calls `discover_tools()`, appends to `agent_tools`
- `McpRemoteTool` fix: `input_schema` now passed through constructor + `parameters_schema()` implemented
- `McpToolDiscovery::get_remote_tools()` output flows into agent tool list
- ignition.rs passes `config.mcp.servers.clone()` to SwarmController
- Files: `core/src/config.rs`, `agent/src/swarm.rs`, `mcp/src/client.rs`, `agent/src/orchestration/ignition.rs`, `agent/tests/production.rs`

#### 2026-03-21: FID — MCP + Next 6 Features

- `dev/fids/FID-20260321-MCP-INTEGRATION-PLUS-NEXT-5.md` — MCP integration + Smithery + Self-Repair + Hooks + Truncation + Mount Security

**Core Recovery (19 fixes):**
- Dashboard WebSocket URLs fixed (port 3000→8080)
- `.env` loaded in Tauri via `dotenvy::dotenv().ok()` (main.rs)
- OR_MASTER_KEY resolution via `std::env::var()` instead of empty `agent_cfg.env_vars` (swarm.rs)
- Agent restart loop fixed - removed `.env` file writes that triggered SwarmWatcher (swarm.rs)
- Avatar path resolution using `config.resolve_path()` (server.rs)
- Agent identity loading from SOUL.md/AGENTS.md/USER.md/IDENTITY.md (registry.rs)
- Coding prompts removed from default system prompt (context.rs)
- Tool tag filtering in stream parser (environment_details, function calls, etc.)
- Dashboard message duplication fixed (WebSocket onmessage only in non-Tauri mode)
- Debug console: copy button, expand, pause on highlight, dayjs formatting, log level colors

**Ollama Integration:**
- `ollama_embeddings.rs` - Ollama embedding service with fastembed fallback
- Default embedding model: `qwen3-embedding:4b` (2560 dims)
- Default vision model: `qwen3-vl` (configured in savant.toml)
- `[ollama]` section added to `savant.toml`
- `async_backend.rs` updated to use `dyn EmbeddingProvider` trait
- `reqwest` added to `core/Cargo.toml`
- Old 384-dim vector data cleared for migration

**Reflections System Redesign:**
- Removed forced `generate_reflection()` from agent loop (stream.rs)
- Removed synthetic `AgentEvent::Reflection` handling from heartbeat (heartbeat.rs)
- `swarm_insight_history` gateway handler reads from LEARNINGS.jsonl directly
- Dashboard shows only diary entries from LEARNINGS.md (no synthetic reflections)
- LEARNINGS.md and LEARNINGS.jsonl wiped clean for fresh start

**Dashboard Enhancements:**
- Chat messages have timestamps (dayjs HH:mm:ss)
- Collapsible thoughts on assistant messages (from `<thought>` tags)
- `is_telemetry` field on ChatChunk for event routing
- Code blocks with working COPY button (stable content-based ID)
- Inline code styled with accent color
- Auto-scroll with streaming content
- `cleanMessage` strips tool tags (environment_details, function calls, etc.)

**Tool Access:**
- Savant bypasses CCT policy (stream.rs - `is_savant` check)
- Sub-agents still go through CCT when added

**Branding:**
- agent.json: `agent_id: "savant"`, `agent_name: "Savant"`, `model: "stepfun/step-3.5-flash:free"`

#### Memory System (All 7 Phases Complete)

- **Auto-Recall** — `auto_recall()` — `auto_recall()` method in AsyncMemoryBackend with EmbeddingService + semantic search + ContextCacheBlock
- **Bi-Temporal Tracking** — `TemporalMetadata` struct (separate Fjall keyspace), `semantic_search_temporal()` filtering active facts
- **Daily Ops Logs** — `DailyLog` with append/read/rotate, markdown format, 500 token cap, 30-day retention
- **Hive-Mind Notifications** — `NotificationChannel` with `tokio::sync::broadcast`, triggers on `index_memory()` when importance >= 7
- **DAG Session Compaction** — `DagNode` struct, `dag_nodes` keyspace, `store/load/fetch_message_by_id()` for reversible compaction
- **Personality-Driven Promotion** — `PromotionEngine` with OCEAN trait scalars, scoring algorithm, promote/archive decisions
- **Entity Extraction** — Rule-based `EntityExtractor` with 5 entity types (project, service, credential, file, config)

#### Other

- Memory System Research — Gemini 3 Deep Research (390 lines, 87 citations)
- Memory System Plan — 7-phase plan certified via Perfection Loop (5 iterations)
- `dev/plans/MEMORY-SYSTEM-PLAN.md` — full implementation specs
- `docs/prompts/MEMORY-SYSTEM-RESEARCH.md` — research prompt (448 lines)

### Changed [Unreleased]

- All paid models removed from model list (only free models shown)
- Config default model changed to `openrouter/hunter-alpha`
- SOUL manifestation engine uses configured model instead of hardcoded anthropic/claude-3.5-sonnet
- Models list handler updated to use FreeModelRouter definitions
- dev/ folder cleaned — stale files archived to dev/archive/2026-03-19/
- development-process.md reorganized into DEVELOPMENT-WORKFLOW.md
- perfection.md renamed to PERFECTION-LOOP.md

### Memory System Research Findings

- `atomic_compact()` is DESTRUCTIVE — deletes all messages before inserting compacted batch
- `MemoryEntry` is rkyv `#[repr(C)]` — adding fields breaks existing serialized data
- Vector engine is global (no per-agent isolation) — confirms hive-mind architecture
- `EmbeddingService` IS `Send` in fastembed 5.12.1 — no dedicated thread needed
- Auto-recall latency target: <200ms (30ms FastEmbed + 15ms Fjall scan)
- Bi-temporal contradiction detection threshold: cosine similarity > 0.92

#### 2026-03-22: Tauri 2.x Upgrade + Desktop Features (~700 LOC)

**Tauri 2.x Migration:**
- `tauri = { version = "2", features = ["tray-icon"] }` — 1.7 → 2.x
- `tauri-build = { version = "2" }` — build dependency upgrade
- `tauri.conf.json` v2 format: `app.windows`, `app.security`, `plugins.updater`
- API: `TrayIconBuilder`, `Emitter` (replaces `emit_all`), `MenuItemBuilder`, `MenuBuilder`
- `get_window()` → `get_webview_window()`
- System tray: `SystemTray` → `TrayIconBuilder` with `on_menu_event`
- Files: `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/tauri.conf.json`, `desktop/src-tauri/src/main.rs`

**Auto-Updater:**
- `tauri-plugin-updater = "2"` — built-in Tauri updater plugin
- GitHub Releases integration via `latest.json` manifest
- Ed25519 signature verification for binary integrity
- Config: `plugins.updater.endpoints` + `plugins.updater.pubkey`

**Splash Screen:**
- `components/SplashScreen.tsx` + `SplashScreen.module.css`
- Logo + spinner + status messages + history log
- Auto-dismiss on "Swarm Ignition Sequence Complete" or timeout (10s)
- Tauri event listener for `system-log-event`
- Files: `dashboard/src/components/SplashScreen.tsx`, `dashboard/src/components/SplashScreen.module.css`

**Version Display:**
- `v1.6.0` badge in sidebar below SAVANT title
- `get_version` Tauri command returning `app.config().version`
- Files: `dashboard/src/app/page.tsx`

**Changelog Page:**
- `/changelog` route with embedded markdown content (v1.6.0 + v1.5.0)
- Sidebar link with 📋 icon
- Files: `dashboard/src/app/changelog/page.tsx`

**Dependency Check:**
- `GET /api/setup/check` — Ollama health check + model availability + issues/instructions
- `POST /api/setup/install-model` — `ollama pull qwen3-embedding:4b` via Ollama API
- `SetupWizard.tsx` + `SetupWizard.module.css` — checklist UI, install button, skip option
- Files: `gateway/src/handlers/setup.rs`, `dashboard/src/components/SetupWizard.tsx`

**Dimension Fix:**
- `engine.rs` — vector dimension now dynamic from embedding service (`emb.dimensions()`)
- Fixed hardcoded 384 → 2560 for qwen3-embedding:4b
- Vector index deleted and recreated at correct dimension

---

## [2.0.1] - 2026-03-18

**Feature completion. Quality pass. 324/324 tests passing.**

### Added [2.0.1]

#### Vector Search / Semantic Memory

- `EmbeddingService` with fastembed AllMiniLML6V2 (384 dimensions)
- LRU cache (1000 entries) for embedding reuse
- Batch embedding (`embed_batch`) for high-throughput
- Semantic retrieval in `AsyncMemoryBackend` — hybrid search
- Auto-indexing during `store()` for messages >= 3 chars

#### MCP Client Tool Discovery

- `McpClient` — WebSocket client with initialize handshake
- `McpRemoteTool` — implements `Tool` trait, proxies to remote servers
- `McpToolDiscovery` — multi-server tool discovery
- `McpClientPool` — compatibility wrapper

#### Docker Skill Execution

- `DockerToolExecutor` — implements `ToolExecutor` trait
- `ExecutionMode::DockerContainer(String)` in core types
- `SandboxDispatcher` routes to Docker with fallback

#### Skill Testing CLI

- `savant test-skill --skill-path <path> --input <json> --timeout <secs>`

#### Database Backup/Restore

- `savant backup --output <path> [--include-memory]`
- `savant restore --input <path>`

#### CLI Subcommand Architecture

- Converted from flags-only to `clap` subcommands
- `start`, `test-skill`, `backup`, `restore`, `list-agents`, `status`

#### Lambda Executor

- `LambdaSkillExecutor` — AWS Lambda Invoke API
- `LambdaTool` — implements `Tool` trait

### Fixed [2.0.1]

#### Test Suite (8 files)

- `gateway/security_tests.rs` — DashMap import, unique temp paths
- `echo/circuit_breaker_tests.rs` — ComponentMetrics API
- `echo/speculative_tests.rs` — CircuitState API
- `cognitive/synthesis.rs` — broader error detection
- `gateway/auth/mod.rs` — generic auth error messages
- `cognitive/predictor.rs` — doc-test examples
- `mcp/integration.rs` — unused variable warnings
- `mcp/client.rs` — unused variable warnings

#### Security

- `AuthError` Display impl: generic "Authentication failed" (no internal details)

---

## [2.0.0] - 2026-03-17

**Deep audit. 121 issues audited, 107+ fixed.**

### Added [2.0.0]

- MCP server authentication with token-based auth
- MCP circuit breaker with CAS transitions
- Security scanner with SHA-256 content hashing
- CLI --keygen and --config flags
- LCS-based array diff in canvas
- RAII TempDirGuard with auto-cleanup

### Fixed [2.0.0]

- `atomic_compact` — deletes old messages before inserting
- Vector persistence — atomic write via temp file + rename
- Path traversal — input validation on skill handlers
- Gateway signing key — OsRng instead of deterministic
- SSRF — disabled redirects in threat intel client
- ChatRole::Tool variant added
- Discord token panic — safe slicing
- Telegram UTF-8 — chars().take() instead of byte slicing
- All 6 `.expect()` calls in gateway replaced
- Separate Fjall instances for storage and memory

---

## [1.0.0] - 2026-03-15

**Initial release. Core system, agents, gateway, dashboard.**

- Rust-native architecture with 14 crates
- Fjall LSM-tree persistent storage
- Axum WebSocket gateway
- Next.js 16 dashboard
- 15 AI providers
- Multi-channel support (Discord, Telegram, WhatsApp, Matrix)
- Post-quantum cryptography (Dilithium2)
- WASM skill sandboxing (wasmtime)
- Docker skill execution (bollard)
- MCP server and client
- Circuit breaker pattern
- Autonomous agent heartbeat loop

---

*Agent-facing changelog. Updated as work happens. Root CHANGELOG.md updated at release time.*
