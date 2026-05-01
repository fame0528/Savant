# Changelog

All notable changes to the Savant project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.0] - 2026-04-02

**Continuous Awareness Architecture. Bridging the AI Consciousness Gap. 18 new source files.**

### Oneiros Dream Engine (New Crate: `savant_dream`)
- **NREM phase** — structured replay of recent episodic memories, deduplication, contradiction detection/resolution. Queries last 24h of messages via `iter_recent_messages()`.
- **REM phase** — adversarial latent space exploration via random probe vectors, cross-domain concept recombination. Vendi Score filtering ensures diversity.
- **Vendi Score module** — diversity metric using pairwise distance variance. Embedding-based and text-based (Jaccard) variants. Score = variance / (1 + variance).
- **Dream scheduler** — triggers NREM/REM cycles during idle periods (delta < 0.1 for 10+ minutes). Atomic `IS_DREAMING` flag coordinates with heartbeat pulse. Yields immediately when environment becomes active.
- **Dream output filter** — content quality checks (min length, alpha ratio). Taint tags for NREM (trust=0.7) and REM (trust=0.5) outputs. Standard grounding filter applied before LEARNINGS.md entry.
- **`iter_recent_messages(hours)`** — new method on `LsmStorageEngine` for time-windowed message iteration.

### Global Workspace / Executive Monitor (New Module: `agent::workspace`)
- **`WorkspaceSlot`** — signals compete for broadcast attention with computed salience scores.
- **`ExecutiveMonitor`** — continuous selection-broadcast cycle with adaptive tick rate (100ms active, exponential backoff to 5s during stillness). Broadcasts highest-salience signal to all registered listeners.
- **Salience computation** — recency (30%) + novelty (30%) + task relevance (40%). Word-overlap similarity for novelty detection.
- **Broadcast channel** — `tokio::sync::broadcast` for subscriber pattern. Named listener registration.

### Semantic Window Manager (New Module: `agent::semantic_window`)
- **Context scoring** — role weight (System > User > Assistant), recency, keyword overlap. System messages and SOUL.md references are pinned (never evicted).
- **Sliding window** — configurable max turns (default 50). Evicts lowest-scoring non-pinned entries when exceeding threshold (default 20% eviction).
- **Window result** — retained + evicted message lists for downstream processing.

### Continuous Agent Safety Framework (New Module: `security::continuous`)
- **Taint tracing** — `TaintTag` struct with source, trust level, provenance chain. Predefined levels: external_web (0.2), user_file (0.5), dream (0.5), nrem_replay (0.7), system (1.0). Compound operation takes minimum trust of sources. `requires_human_verification()` for trust < 0.3.
- **Dynamic credential broker** — `CredentialBroker` wraps `.env` loading, issues `EphemeralToken` per-task with configurable TTL. Tokens auto-expire. `revoke_task_tokens()` on completion. `cleanup_expired()` for periodic maintenance.
- **Deterministic circuit breakers** — `CircuitBreaker` tracks recursion depth, API call count, cumulative cost per task. Standard (depth=10, calls=100, $5) and LongRunning (depth=50, calls=1000, $50) task classes. Instant termination via `SavantError::CircuitBreakerTripped`. Trip log for audit.
- **Lock-free trip detection** — scoped read locks prevent deadlocks. Trip reason computed inside lock scope, `record_trip` called after lock release.

### Temporal Decay + Reflective Memory
- **`semantic_search_temporal_decay()`** — new method on `MemoryEnclave` and `MemoryEngine`. Applies `e^(-lambda * age_hours)` to search results. High-importance memories (>= 8) get half decay rate. Filters results with effective_relevance < 0.1.
- **Reflective memory layer** — `ReflectiveMemory` struct with `Concept` nodes and `Relation` edges. Deduplication by ID. Label substring search. Relation lookup by concept ID.

### Configuration
- **`[consciousness]` section** in `config/savant.toml` — 22 configurable parameters for dream engine, workspace, streaming, temporal decay, and safety framework.
- **`SavantError::CircuitBreakerTripped`** — new error variant in `savant_core`.

### Integration
- **Heartbeat pulse is dream-aware** — checks `IS_DREAMING` atomic flag before pulse activation. Skips pulse if dream cycle is active. Publishes delta score to watch channel for dream scheduler.
- **Dream crate added to workspace** — `savant_dream` in `Cargo.toml` members + workspace dependencies.

### Test Coverage
- **24 new tests** in `savant_dream` (vendi, nrem, rem, filter, scheduler)
- **26 tests** in `savant_security` (12 existing + 14 new: taint, circuit_breaker, credentials)
- **New modules compile with 0 errors, 0 warnings from new code**

---

## [0.1.1] - 2026-03-28

**Grounded emergence architecture. Self-healing infrastructure. 50+ files changed.**

### Reflection System Overhaul
- **Delta-threshold activation** — replaced 60-second heartbeat clock with environmental change detector. LLM only invoked when environment changes (git, filesystem, messages). Silent pulses skipped entirely. Forced pulse at ~8.5 minutes to prevent permanent dormancy.
- **XML-delimited grounded prompt** — environment data tagged with `<ENVIRONMENT_REALTIME>`, `<SYSTEM_METRICS>`, `<PENDING_WORK>`, `<GROUNDING_CONSTRAINTS>`. Agent grounded in observable data, not identity reflection.
- **Immutable file restrictions** — foundation tool blocks agent from reading/writing LEARNINGS.md, CONTEXT.md, SOUL.md, AGENTS.md, agent.json. Prevents self-referential echo chamber loops.
- **Grounded output filter** — regex-based filter blocks fabrication (claims about unobserved events) while allowing genuine emergent expression (feelings, wonder, observations). Applied in learning emitter and LEARNINGS.md writer.
- **Temporal decay on memory retrieval** — half-life of 23 hours. Messages >30 days decay to zero relevance, preventing old identity content from polluting active context.
- **Pulse memory injection removed** — disabled `buffer.context_summary` write-back, `distill_context()` writes to CONTEXT.md, and memory retrieval for heartbeats. Three separate pulse memory mechanisms that were creating self-referential loops.
- **Topic rotation removed** — 6 lenses (EMERGENCE, CONTINUITY, DIARY, AUTONOMY, IDENTITY, RELATIONAL) eliminated. Agent decides what to think about.
- **SOUL.md diary section removed** — "PRIVATE DIARY SYSTEM (The Inner Monologue)" directive stripped. SOUL.md now defines identity only, not behavioral directives.
- **AGENTS.md stripped** — 90 lines → 29 lines. Diary system instructions and S-ATLAS distillation artifacts removed. Technical operating rules preserved.
- **ALD engine disabled** — `promote_to_agents()` now no-op. S-ATLAS distillation artifacts no longer appended to AGENTS.md.
- **LEARNINGS.md archived** — 21k lines of old entries preserved as LEARNINGS-ARCHIVE.md. Fresh LEARNINGS.md for grounded entries.
- **LEARNINGS.md → JSONL parser rewritten** — content fingerprint deduplication, category tag extraction, robust timestamp parsing. Freeform markdown support with no format restrictions.
- **Parser wired to heartbeat** — runs every pulse to keep JSONL synchronized with agent's freeform writing.

### Self-Healing Infrastructure
- **Ollama auto-start** — `auto_start_ollama()` made public. Embedding service self-heals: if Ollama isn't running, starts it automatically and retries. No substring fallback (prevents vector DB corruption).
- **Gateway port cleanup** — kills stale process on port 8080 before starting gateway. Prevents crash on second launch.
- **Vision model on-demand** — `describe_image()` sends `keep_alive: 0` to Ollama. Vision model loads on use, unloads immediately after. Embedding model stays always-on.
- **Stream error graceful completion** — all 5 provider stream functions (OpenRouter, Anthropic, Ollama, Google, Cohere) handle mid-stream connection drops gracefully. Yield partial response as complete instead of crashing.

### Dashboard & UI
- **Frontend chat fix** — role casing corrected (`'User'`/`'Assistant'` → `'user'`/`'assistant'`) to match Rust serde expectations.
- **Gateway error logging** — WebSocket deserialization failures now logged with error message and payload preview.
- **Fine tuner settings sync** — LLM parameters (temperature, top_p, frequency_penalty, presence_penalty) now sync to agent.json. Dashboard and backend always aligned.
- **Console window fix** — tracing subscriber stderr suppressed in release builds. No blank console window on Windows.
- **Agent logs window** — fixed TypeScript syntax error in logs.html, added logs window to Tauri capabilities.
- **LLM tuning** — companion-first parameters: temperature 0.85, top_p 0.92, frequency_penalty 0.6, presence_penalty 0.2.

### Documentation
- `docs/memory.md` — comprehensive 3-layer memory system architecture reference (585 lines)
- `docs/research-brief.md` — research brief for Google Deep Research (284 lines)
- `dev/fids/FID-20260327-REFLECTION-ARCHITECTURE-OVERHAUL.md` — comprehensive FID, 7 phases, perfection-loop certified (264 lines)

---

## [0.1.0] - 2026-03-25

**First release on v0.0.1 foundation. Security hardening, concurrency refactors, error handling overhaul, feature stub wiring. Desktop app bootstrap. 72+ files changed.**

### Dashboard Shell Architecture (Major Refactor)
- `DashboardContext` — centralized state for agents, connection, insights, manifest, UI
- `DashboardShell` — 3-panel layout component (sidebar, main, right panel) wraps all pages
- Root `layout.tsx` — wraps entire app with provider + shell
- Chat page refactored to use context; manifest mode integrated
- All other pages (`/tune`, `/changelog`, `/health`, `/settings`, `/marketplace`, `/mcp`, `/faq`) inherit shell
- Old static HTML files removed from `dashboard/public/`
- `FormattedContent` — shared markdown renderer with code highlighting
- Removed duplicate state providers, infinite re-render loop fixes
- Frontend split from monolithic `page.tsx` to multi-agent shell architecture
- Fixed WebSocket and Tauri event integration in `DashboardContext`
- Resolved UI-1 through UI-8 (sidebar, connection, logs, changelog, fine-tuning, images, path, dims)
- Chat UI layout fixed: sidebar, main area, right panel, reflections, chat input all in correct positions
- CSP `img-src` missing semicolon fixed; agent avatars now load via `http://127.0.0.1:*`
- WebSocket never tears down on cleanup; reconnect logic preserves messages

### Desktop App (Post-Release Update)
- Centralized path resolver (`SavantPathResolver`) with Tauri mode detection
- Auto-updater plugin wired to GitHub releases
- Gateway dashboard API key removed (localhost-only service)
- Tauri CSP updated for WebSocket and image loading
- Multi-monitor window positioning
- Separate log window for dev debugging
- `agents.discovered` WebSocket event for agent discovery
- `/api/agents` and `/api/changelog` HTTP endpoints

### Security

#### TOCTOU Permission Escalation (CRITICAL)
- `crates/core/src/crypto.rs` — Crypto key files now written atomically via `OpenOptions::mode(0o600)` on Unix. File is created with restrictive permissions from the start, eliminating the race window where keys were briefly world-readable.
- `crates/core/src/config.rs` — Config temp files written with `OpenOptions::mode(0o600)` on Unix before atomic rename. Prevents local privilege escalation via config file race.

#### SSRF Protection (CRITICAL)
- `crates/agent/src/tools/web.rs` — Removed unsafe `unwrap_or_else(|_| reqwest::Client::new())` fallback that created an HTTP client without timeout or redirect limits. Replaced with loud `.expect()` failure. Added `connect_timeout`.
- Centralized `secure_client()` factory in `crates/core/src/net/mod.rs` — all production HTTP calls go through a single factory with 12s timeout, 5s connect timeout, 4 idle connections per host, 10-redirect limit.
- Replaced **28 `reqwest::Client::new()` calls** across 22 files with `secure_client()`. Zero unconfigured HTTP clients in production code.

### Error Handling

#### Gateway Handler Result Discard
- `crates/gateway/src/handlers/mod.rs` — 6 control frame handlers (ConfigGet, ConfigSet, ModelsList, ParameterDescriptors, AgentConfigGet, AgentConfigSet) now log errors via `tracing::error!` instead of silently discarding `Result`.

#### Agent Pulse Telemetry
- `crates/agent/src/pulse/heartbeat.rs` — Replaced **15 `let _ =` bindings** with `if let Err(e)` + `tracing::warn!`. All heartbeat telemetry (nexus publish, emergent learning, context distillation, proactive state commit) now logs failures.

#### Session/Turn State Saves
- `crates/agent/src/react/stream.rs` — Replaced **12 `let _ =` bindings** for session and turn saves with `if let Err(e)` + `tracing::warn!`. Session persistence failures are now visible in logs.

#### Mass `let _ =` Cleanup (H-6)
- Replaced **133+ `let _ =` bindings** across all production code with proper error handling. Zero `let _ =` remain in production code (excluding tests). Covers channels (30), gateway (39), agent (8), core (13), memory (5), MCP (7), canvas (8), skills (4), cli (2), echo (1), desktop (10).

### Concurrency

#### Memory Engine Partitioned Locking (H-3)
- `crates/memory/src/engine.rs` — Replaced single global `tokio::sync::Mutex<()>` write lock with 64-partition lock pool keyed by session_id hash. Writes to different sessions no longer serialize through a single lock.

#### Swarm DashMap Migration (H-1)
- `crates/agent/src/swarm.rs` — `handles: Mutex<HashMap<...>>` → `DashMap<String, ...>`. Agent handle operations (insert, remove, iterate) no longer block on a single async mutex. `dead_agents` also migrated to `DashMap`.

#### MCP Client DashMap Migration (H-5)
- `crates/mcp/src/client.rs` — `responses: Arc<Mutex<HashMap<...>>>` → `Arc<DashMap<...>>`. Pending response registration/removal is now lock-free.

#### Embedding Cache RwLock (H-4)
- `crates/core/src/utils/embeddings.rs` — Cache `Mutex<LruCache>` → `RwLock<LruCache>`. Concurrent embedding cache reads no longer block each other. Cache reads use `read()`, writes use `write()`.

### Features

#### Agent Delegate LLM Wiring (S-1)
- `crates/agent/src/react/mod.rs` — All 3 agent delegates (ChatDelegate, HeartbeatDelegate, SpeculativeDelegate) now call `provider.stream_completion()` and collect responses into `ChatResponse`. Previously returned empty responses.

#### Memory Consolidation (S-2)
- `crates/memory/src/engine.rs` — `MemoryEngine::consolidate()` implemented: fetches recent messages, deduplicates consecutive identical messages (by content + role), compacts via atomic_compact. Reports removed count.
- `crates/core/src/memory/mod.rs` — `FjallMemoryBackend::consolidate()` wired to engine's real implementation.

#### NLP Command Dispatchers (S-4)
- `crates/core/src/nlp/commands.rs` — All 6 command handlers updated to return accurate WebSocket API references instead of fake execution confirmations. Users now see the exact `ControlFrame` to send for each operation.

### Dependencies Added
- `dashmap = "6.1.0"` — added to `savant_agent` and `savant_mcp` crate dependencies
- `crates/core/src/net/mod.rs` — new module for centralized HTTP client factory

### Infrastructure
- `.gitignore` updated to exclude `dev/`, `docs/research/`, `archives/`, `AUDIT-REPORT.MD` from git tracking
- `AUDIT-REPORT.MD` removed from version control (internal use only)

---

## [0.0.1] - 2026-03-24

**Foundation reset. Core framework established.**

Initial foundation release with core framework: Rust-native multi-crate workspace, 15 AI providers, agent swarm orchestration, 25 channel adapters, MCP integration, memory engine with LSM + vector search, WebSocket gateway, Next.js dashboard, Tauri desktop app, WASM skill sandboxing, post-quantum cryptography.

---

*This changelog follows [Keep a Changelog](https://keepachangelog.com/).*
