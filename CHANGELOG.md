# Changelog

All notable changes to the Savant project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] - 2026-03-25

**First release on v0.0.1 foundation. Security hardening, concurrency refactors, error handling overhaul, feature stub wiring. 72 files changed, +1,407 / -375 lines.**

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
