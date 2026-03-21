# Internal Changelog

> **Purpose:** Detailed project changelog for agents. More detailed than root CHANGELOG.md.  
> **Updated:** As work happens, not just at release time.

---

## [Unreleased]

### Added

#### 2026-03-20: Post-Rollback Recovery & Ollama Integration

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
