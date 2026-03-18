# Internal Changelog

> **Purpose:** Detailed project changelog for agents. More detailed than root CHANGELOG.md.  
> **Updated:** As work happens, not just at release time.

---

## [Unreleased]

### Added
- Savant Coding System v0.0.2 as embedded skill (`skills/savant-coding-system/`)
- Perfection Loop embedded in coding system (5-step quality protocol)
- Law 11: Utility-First, Universal Logic (combine overlap into universal functions)
- Free Model Router (`crates/agent/src/free_model_router.rs`) — hunter-alpha → healer-alpha → stepfun → openrouter/free
- Language supplements: RUST.md, TYPESCRIPT.md, PYTHON.md
- DEV-FOLDER-SPECIFICATION.md — complete agent-facing reference for /dev structure
- Dual-level architecture (swarm-wide skills/ vs agent-specific workspaces/)
- Key management architecture design (auto vs manual modes)

### Changed
- All paid models removed from model list (only free models shown)
- Config default model changed to `openrouter/hunter-alpha`
- SOUL manifestation engine uses configured model instead of hardcoded anthropic/claude-3.5-sonnet
- Models list handler updated to use FreeModelRouter definitions
- dev/ folder cleaned — stale files archived to dev/archive/2026-03-19/
- development-process.md reorganized into DEVELOPMENT-WORKFLOW.md
- perfection.md renamed to PERFECTION-LOOP.md

---

## [2.0.1] - 2026-03-18

**Feature completion. Quality pass. 324/324 tests passing.**

### Added

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

### Fixed

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

### Added
- MCP server authentication with token-based auth
- MCP circuit breaker with CAS transitions
- Security scanner with SHA-256 content hashing
- CLI --keygen and --config flags
- LCS-based array diff in canvas
- RAII TempDirGuard with auto-cleanup

### Fixed
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
