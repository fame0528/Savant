# Changelog

All notable changes to the Savant project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

_No unreleased changes._

---

## [2.0.1] - 2026-03-18

**Feature completion. Quality pass. Gap analysis. Autonomous workflow documented.**

**Stats:** 14/14 features complete. 324/324 tests passing. 0 errors. 0 warnings. 28 files changed, +2615 / -476 lines.

### Added

#### Vector Search / Semantic Memory
- `EmbeddingService` with fastembed AllMiniLML6V2 (384 dimensions) in `crates/core/src/utils/embeddings.rs`
- LRU cache (1000 entries) for embedding reuse
- Batch embedding (`embed_batch`) for high-throughput scenarios
- Semantic retrieval in `AsyncMemoryBackend` — hybrid search with fallback to substring matching
- Auto-indexing during `store()` for messages >= 3 characters
- Re-exported from `savant_memory` crate

#### MCP Client Tool Discovery
- `McpClient` — full WebSocket client: connect, initialize handshake, tools/list, tools/call in `crates/mcp/src/client.rs`
- `McpRemoteTool` — implements `Tool` trait, proxies execution to remote MCP servers
- `McpToolDiscovery` — discovers tools from multiple servers, bridges to local registry
- `McpClientPool` — compatibility wrapper with `connect()`, `execute_tool()`, `list_tools()`
- Auth support via `connect_with_auth()`
- 30-second request timeout with oneshot channel cleanup

#### Docker Skill Execution
- `DockerToolExecutor` — implements `ToolExecutor` trait in `crates/skills/src/docker.rs`
- `ExecutionMode::DockerContainer(String)` in `crates/core/src/types/mod.rs`
- `SandboxDispatcher::create_executor()` routes to Docker with fallback executor
- 8 integration tests passing (requires Docker daemon)

#### Skill Testing CLI
- `savant test-skill --skill-path <path> --input <json> --timeout <secs>`
- Loads via `SkillRegistry`, executes with timeout enforcement
- Pass/fail output formatting

#### Database Backup/Restore
- `savant backup --output <path> [--include-memory]`
- `savant restore --input <path>`
- Atomic backup with pre-restore safety copy
- Supports main database and memory database

#### CLI Subcommand Architecture
- Converted from flags-only to `clap` subcommands in `crates/cli/src/main.rs`
- `start` — launch swarm orchestrator (default, backward compatible)
- `test-skill` — test individual skills
- `backup` / `restore` — database management
- `list-agents` — discover workspace agents
- `status` — system health check

#### Lambda Executor
- `LambdaSkillExecutor` — AWS Lambda Invoke API integration in `crates/skills/src/lambda.rs`
- `LambdaTool` — implements `Tool` trait for skill registry
- Configurable region, function name, timeout, sync/async invocation
- 8 unit tests

#### Autonomous Development Workflow
- `docs/AUTONOMOUS-WORKFLOW.md` — formalized overnight automation protocol
- `docs/GAP-ANALYSIS.md` — 10 high-impact features, 12 easter eggs, impact ratings
- `dev/SESSION-SUMMARY.md` — session report

### Fixed

#### Test Suite (8 files)
- `crates/gateway/tests/security_tests.rs` — `DashMap` import, unique Fjall temp paths, `NexusBridge::new()` API
- `crates/echo/tests/circuit_breaker_tests.rs` — rewrote for `ComponentMetrics` / `CircuitState` API
- `crates/echo/tests/speculative_tests.rs` — rewrote for actual circuit breaker interface
- `crates/cognitive/src/synthesis.rs` — broadened error detection to include plain text "Error:" prefix
- `crates/gateway/src/auth/mod.rs` — auth tests check generic message instead of internal details
- `crates/cognitive/src/predictor.rs` — doc-test examples fixed (missing `max_history_size`, `Result::unwrap()`)
- `crates/mcp/tests/integration.rs` — unused variable warnings
- `crates/mcp/src/client.rs` — unused variable warning

#### Security
- `SavantError::AuthError` Display impl changed from `"Authentication failed: {0}"` to `"Authentication failed"` — prevents information leakage to external callers

---

## [2.0.0] - 2026-03-17

**Deep audit. Security hardening. 121 issues audited, 107+ fixed. Full line-by-line review of all 133 source files.**

### Added

#### MCP Server Authentication
- Token-based auth for MCP WebSocket connections
- Rate limiting: 100 requests/minute/connection
- Auth required before any method except `initialize`
- Hash-based token verification

#### MCP Circuit Breaker
- Full implementation: Closed → Open → HalfOpen with CAS transitions
- Configurable thresholds (failure, recovery, success)
- 5 unit tests

#### Security Scanner Enhancements
- Recursive directory traversal with `walkdir`
- SHA-256 content hashing replacing `DefaultHasher`
- Directory-wide hashing (all files, not just SKILL.md)

#### CLI Features
- `--keygen` flag for master key generation
- `--config` flag for custom config path
- Dynamic build timestamp

#### LCS-Based Array Diff
- Proper Longest Common Subsequence algorithm in `crates/canvas/src/diff.rs`

#### RAII Temp Directory Cleanup
- `TempDirGuard` with auto-cleanup on drop

### Fixed

#### Data Integrity
- `atomic_compact` — deletes old messages before inserting compacted batch
- `delete_session` — collects keys inside transaction snapshot
- Vector persistence — atomic write via temp file + rename
- Vector engine Drop — auto-persists on Drop
- `db.rs` — rewrote Storage with proper ghost_restore, partition counters

#### Security
- Path traversal — input validation on all skill handlers
- Gateway signing key — `OsRng` instead of deterministic
- SSRF — disabled redirects in threat intel client
- Auth error leak — generic messages
- Directive injection — length limit, control char rejection
- Token verification — proper error propagation
- File permissions — 0o600 on Unix

#### Agent Crate
- `ChatRole::Tool` variant added
- `MessageRole::Tool` → `ChatRole::Tool` mapping
- API key serialization skip
- Provider fallback warning

#### Echo Crate
- Circuit breaker — Mutex + CAS for TOCTOU protection
- AWS env leak — explicit allowlist all platforms
- Watcher thread — `mem::forget(debouncer)`

#### Cognitive Crate
- Forge panic — early return for empty population
- Goal decomposition — advance past conjunction
- Dependency depth — bounds check

#### Channels
- Discord token panic — safe slicing
- Telegram UTF-8 — `chars().take()` instead of byte slicing
- Discord resource leak — `spawn()` returns `JoinHandle`
- WhatsApp zombie process — handle storage + Drop cleanup

#### Gateway
- Replaced all 6 `.expect()` calls
- Auth error sanitized
- Agent image handler — name validation

#### Memory Engine
- Non-atomic delete operations fixed
- `retrieve()` uses query parameter
- Error propagation in `vector.remove()`
- Rollback failures logged at critical

### Changed
- Storage and Memory Engine use separate Fjall instances (prevents `Locked` error)
- `SessionMapper::sanitize()` returns `Option<String>`
- Build timestamp uses `std::time::SystemTime`

---

## [1.0.0] - 2026-03-15

**Initial release. Core system, agents, gateway, dashboard.**

### Core System
- Rust-native architecture with 14 crates
- Fjall LSM-tree persistent storage
- Axum WebSocket gateway
- Next.js 16 dashboard
- 15 AI providers (OpenRouter, OpenAI, Anthropic, Google, Mistral, Groq, Deepseek, Cohere, Together, Azure, xAI, Fireworks, Novita, Ollama, LmStudio)
- Multi-channel support (Discord, Telegram, WhatsApp, Matrix)
- Post-quantum cryptography (Dilithium2)
- WASM skill sandboxing (wasmtime)
- Docker skill execution (bollard)
- MCP server and client
- Circuit breaker pattern
- Autonomous agent heartbeat loop
- Semantic memory with vector search
- Security scanning for skills
- ClawHub skill marketplace
- Configuration auto-reload
- Smart build system with `start.bat`

---

_This changelog follows [Keep a Changelog](https://keepachangelog.com/). For the full development history, see the Git log._
