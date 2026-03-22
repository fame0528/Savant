# Internal Changelog

> **Purpose:** Detailed project changelog for agents. More detailed than root CHANGELOG.md.  
> **Updated:** As work happens, not just at release time.

---

## [Unreleased]

### Added

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
