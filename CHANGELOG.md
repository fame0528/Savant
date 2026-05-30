# Changelog

All notable changes to the Savant project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.4.0] - 2026-05-30

**v0.4.0: Two-tier agent system, DelegationEngine, 10 bug fixes, 8,501 markdown violations eliminated. 37 implementation steps across 3 FIDs.**

### Added

#### Two-Tier Agent System (FID-20260530-AGENT-TIER-REDESIGN)
- **AgentTier enum** (`Full` / `SubAgent`) added to `AgentConfig` for type-level distinction between workspace agents and ephemeral sub-agents.
- **AgentRole enum** (`Main` / `Orchestrator` / `Leaf`) for delegation depth control. Leaf agents cannot delegate further.
- **SubAgentProfile** struct with SOUL.md, tool restrictions, iteration budget, token budget, timeout, and delegation permissions.
- **6 specialized profiles** with enterprise-quality SOUL.md persona specs: coding (ECHO-compliant), documentation, research, testing, orchestrator, general.
- **DelegationEngine** — profile-based sub-agent spawning with keyword routing, delegation hooks (on_start, on_complete), result caching (5min TTL), and lifecycle observability events.
- **SubAgentRegistry** — DashMap-based in-memory tracking for active sub-agents with IterationBudget, CancellationToken, and cancel-by-parent.
- **SubAgentSemaphore** — separate governor permit pool for sub-agents (128/64/32/8).
- **AgentFileLock** — reader-writer locks with deadlock prevention for multi-agent file access.
- **LoopDetector** — multi-layered tool call loop detection (identical 4x, failing 6x, max 75 calls, max 20 failures).
- **ToolFilter** — per-profile tool restrictions for sub-agents.
- **WorkspaceGuard** — path validation for sub-agent file operations.
- **MemoryEnclaveHandle** — read-only wrapper for sub-agent memory access.
- **AgentLimits** config struct with sub-agent concurrency, depth, iteration, token, and timeout limits.
- **Structured delegation parser** — ` ```delegate ` JSON blocks with legacy `DELEGATE:` fallback.
- **Governor defaults updated** from 16/8/4/1 to 128/64/32/8 (proven: OpenClaw ran 101 agents on same hardware).
- **Shared CapabilityRegistry** — single instance at Swarm level instead of per-agent.
- **5 integration tests** for full delegation pipeline.

#### Bug Fixes (10 pre-existing bugs)
- **pop_deferred() duplicate spawn** — removed re-push before return, added deferred drain loop.
- **Blackboard clobbering** — subagent-specific hashes replace shared session_hash.
- **CCT minting consolidation** — 3 duplicate sites merged into single `mint_subagent_cct()`.
- **Subagent count limit** — `max_subagents_per_agent` check on all spawn paths.
- **Graceful subagent shutdown** — 10s drain timeout replaces `handle.abort()`.
- **Error propagation** — `JoinHandle<Result<(), String>>` replaces swallowed errors.
- **Speculative parallelism** — `join_all` for concurrent branch execution.
- **Agent index race condition** — `compare_exchange` loop replaces `fetch_add` + `store`.
- **Shared CapabilityRegistry** — Swarm-level singleton replaces per-agent instances.
- **Structured delegation parser** — ` ```delegate ` JSON blocks replace naive `DELEGATE:` prefix.

#### Session State WAL (FID-20260530-SESSION-STATE-WAL-ENTERPRISE)
- **YAML frontmatter + structured markdown** replaces minified JSON WAL format.
- **Schema versioning** — `schema_version: u32` field on WorkingBuffer (v0=legacy, v1=frontmatter).
- **Frontmatter parser** with legacy JSON fallback and auto-detection.
- **CLI `state --inspect`** — colored, structured display of WAL sections.
- **6 unit tests** for roundtrip, legacy compat, braces-in-content, missing file, schema default.

#### Markdown Zero Defect (FID-20260529-MARKDOWN-ZERO-DEFECT)
- **8,501 markdownlint violations eliminated** across 300 files (29 rules). Zero violations remaining.
- **Auto-fix pass** handled 7,126 violations (MD032, MD022, MD012, MD009, MD047, MD058, MD055, MD004, MD007, MD030, MD056).
- **Manual fixes** for 1,375 violations (MD040, MD024, MD036, MD029, MD025, MD026, MD001, MD041).

#### Workspace Updates
- **SOUL.md** rewritten as enterprise persona specification. Removed corrupted maxim entries, stale v16.2 references, marketing prose.
- **AGENTS.md** updated with two-tier architecture documentation and profile table.
- **6 profile SOUL.md files** rewritten to enterprise quality with behavioral profiles, operational constraints, decision frameworks, and identity invariants.

## [0.4.0] - 2026-05-29

**v0.4.0: Messaging pipeline hardening, 10-state message status UX, memory enclave wiring. 26 issues fixed across 3 FIDs.**

### Added

#### Message Status UX (Category E — 11 items)
- **10-state message status machine** replacing the previous 3-state system (`sent`/`processing`/`complete`). New states: `sending`, `sent`, `delivered`, `thinking`, `executing`, `streaming`, `complete`, `failed`, `error`, `timeout`.
- **Gateway delivery ACK** — gateway publishes `session.{id}.ack` after routing a message to Nexus. Frontend upgrades status from `sent` to `delivered`.
- **Telemetry-driven granularity** — telemetry chunks now drive `thinking`/`executing` state transitions. `**Executing Tool:**` content detected for `executing` state.
- **Dual timeout timers** — 60s ACK timer (`sent` → `error` if gateway never acknowledges) + 30s response timer (`delivered` → `timeout` if agent never responds).
- **Inline recovery actions** — `RETRY`, `DISMISS`, and `WAIT LONGER` buttons on terminal-state messages (`failed`, `error`, `timeout`). RETRY cancels in-flight processing before re-sending.
- **Activity-aware typing indicator** — replaces binary bouncing dots with context-aware display showing `thinking`, `executing` (with tool name), and elapsed time.
- **10-state status dot** — each state has distinct color, glow, animation, and micro-label. CSS variables for theme support. `prefers-reduced-motion` support. `aria-label` for screen readers.
- **`is_error` field on `ChatMessage`** — structured error detection replaces fragile string matching.

#### Messaging Pipeline Hardening (Categories A-D — 13 items)
- **Identity pinning logging** (A1) — silent drops from echo-back prevention now logged with content preview.
- **Lane backpressure error response** (A2) — timed-out messages now send error `ResponseFrame` instead of `continue`.
- **Session mismatch event** (A3) — stale session IDs trigger `session.mismatch` event with auto-reconnection.
- **Oversized message error** (A4) — messages >1MB now return error instead of silent drop.
- **Agent loop error response** (B1) — LLM errors publish `is_error: true` response to user instead of killing heartbeat loop.
- **Task supervisor** (B2) — task panics detected and logged with task name. Deterministic cleanup on all paths.
- **`tokio::select!` complete arm** (B3) — deterministic session cleanup when all tasks complete naturally.
- **`system.agent.ready` event** (C1) — agent publishes ready event after boot for presence tracking.
- **Agent presence check** (C2) — gateway checks `system.agents` before routing, returns "No agents available" error.
- **Structured tracing** (D1-D3) — inbound message tracing at agent, WS frame tracing at gateway, outbound Nexus tracing at gateway.

#### Memory Enclave Wiring
- **Orchestrator now receives `MemoryEnclave`** — `engine.enclave()` extracted before engine move, passed to `from_agent_loop()`. Both `delegate_task()` and `continue_delegation()` now receive real context packages instead of empty ones.

#### Version Infrastructure
- **Cargo workspace version inheritance** — all 28 crates use `version.workspace = true`. Version defined once in root `Cargo.toml` `[workspace.package]`.
- **`VERSION` file** — single source of truth. `scripts/bump-version.ps1` and `scripts/bump-version.sh` sync it to Cargo.toml, package.json, package-lock.json, and tauri.conf.json.

### Fixed

#### UI Scaling
- Dashboard window now spawns maximized (`"maximized": true` in tauri.conf.json).

### Changed

#### Version Bump
- **Root `Cargo.toml`** `[workspace.package]` version set to 0.4.0
- **All 28 Cargo.toml** converted to `version.workspace = true`
- **Both tauri.conf.json** bumped to 0.4.0
- **dashboard/package.json** bumped to 0.4.0
- **README.md + docs READMEs** updated to v0.4.0

---

## [0.3.5] - 2026-05-28

**v0.3.5: Dashboard history partition key fix, Agent Logs Copy All fix, version bump. 2 critical bugs fixed.**

### Fixed

#### Dashboard History Partition Key (Critical)
- **`GatewayPersistence::persist_chat()`** in `crates/gateway/src/persistence.rs` was storing agent responses in `chat.{session_uuid}` collections (UUID-keyed, invisible to dashboard) while `get_history()` queried `chat.{agent_name}`. Agent responses from months of development were stored but never retrievable.
- Flipped partition precedence from `session_id > agent_id > sender > recipient` to `agent_id > sender > recipient > session_id`. Agent responses now stored in `chat.{agent_name}` matching the dashboard's `get_history()` query path.
- Fixes "Loading conversation..." on dashboard when agent has stored memory.

#### Agent Logs Copy All
- Rewrote `copyAllLogs()` in `dashboard/public/logs.html` with 3-tier clipboard fallback: Tauri clipboard plugin (`plugin:clipboard|write_text`) → `navigator.clipboard` → `execCommand` with in-viewport textarea. Fixes silent failure in Tauri WebView where offscreen textarea selection was not recognized.

### Changed

#### Version Bump
- **All 28 Cargo.toml** bumped from 0.3.4 to 0.3.5
- **Both tauri.conf.json** bumped from 0.3.4 to 0.3.5
- **dashboard/package.json** bumped from 0.1.0 to 0.3.5 (was never synced from scaffold default)
- **README.md + docs READMEs** updated to v0.3.5

---

## [0.3.4] - 2026-05-28

**v0.3.4 Hotfix: vector DB lock, auth log spam, SSE parse noise, dashboard UX. 5 issues fixed.**

### Fixed

#### Vector Database Lock (Critical)
- **UNC path fallback** in `crates/memory/src/engine.rs` — `strip_unc_prefix()` converts `\\?\\` extended-length paths to canonical form
- Backup and remove operations now use canonical paths to avoid Windows os error 267
- Falls back to lock-file-only deletion (`enclave/vector/lock`) if full directory removal fails
- Resolves persistent ignition failure on second launch (BUILD3, BUILD4, BUILD6)

#### Auth Log Spam
- **Rate-limited WARN logging** in `crates/gateway/src/auth/http_middleware.rs` — max 1 WARN per 10s via `AtomicI64` timestamp tracking
- **Authenticated image loading** — `fetchAuthImage()` utility in DashboardContext fetches with `Authorization: Bearer` header, caches blob URLs
- **AuthImage component** in DashboardShell — replaces raw `<img src>` that triggered 401s on every render
- Eliminates ~500+ WARN lines per session

#### SSE Parse Failures
- **Downgraded to debug** in `crates/agent/src/providers/mod.rs` — partial/malformed SSE chunks from OpenRouter no longer generate WARN noise

#### Dashboard Input Button
- **Always visible** on home page — uses opacity + pointerEvents + disabled attribute when offline instead of hiding entirely

#### Build Hygiene
- Fixed UTF-8 BOM in 30 files (28 Cargo.toml + 2 tauri.conf.json) caused by PowerShell Set-Content
- Fixed pre-existing clippy `useless_conversion` in `crates/gateway/src/server.rs`

#### Agent Discovery & Copy Unification (Build 7)
- **Agent discovery dead code fix** — merged `activeAgent` logic into first `agents.discovered` handler, deleted unreachable second handler in `DashboardContext.tsx`
- **Auto-select first agent on discovery** — when no agent is active and no saved preference, auto-selects first discovered agent and requests lane history
- **Agent display names** — sidebar, header, and input placeholder now use `getAgentMeta()` for proper display names (`.savant` → `SAVANT`)
- **`.savant` default agent** — sidebar always shows the core `.savant` agent even before gateway discovery
- **Unified `copyToClipboard()`** in `dashboard/src/lib/tauri.ts` — 3-tier fallback (Tauri plugin, navigator.clipboard, execCommand), used by all copy buttons
- **Code block copy** now has Tauri fallback + visual feedback via `CodeCopyButton` component in `FormattedContent.tsx`
- **Removed duplicate helpers** — `cleanMessage()` (3 to 1), `formatEst()` (2 to 1), `getGatewayHost/Port/HttpUrl` (2 to 1 centralized in tauri.ts)
- **Agent Logs Copy All fix** — rewrote `copyAllLogs()` in `logs.html` with 3-tier clipboard fallback: Tauri clipboard plugin (`plugin:clipboard|write_text`) → `navigator.clipboard` → `execCommand` with in-viewport textarea. Fixes silent failure in Tauri WebView where offscreen textarea selection was not recognized.
- **Dashboard history partition key fix** — `GatewayPersistence::persist_chat()` in `persistence.rs` was storing agent responses in `chat.{session_uuid}` collections (UUID-keyed) while `get_history()` queried `chat.{agent_name}`. Flipped partition precedence to `agent_id > sender > recipient > session_id` so responses are stored in the same collection the dashboard reads from. Fixes "Loading conversation..." on dashboard when agent has stored memory.

---

## [0.3.3] - 2026-05-28

**v0.3.3 Release Hardening. 24 issues fixed across 5 live test rounds. OpenGateway key rotation, MalwareBazaar integration, dashboard connectivity, CLI resilience, version sync.**

### Added

#### OpenGateway Key Rotation
- **11 built-in API keys** with round-robin rotation at startup (`config.rs: OPENGATEWAY_DEFAULT_KEYS`)
- Defense-in-depth: consciousness daemon + main agent fallback to rotation when env var empty
- Keyring → env var → built-in rotation (priority chain)
- Easy to extend: append keys to the const array, no other changes needed

#### MalwareBazaar Threat Intelligence
- **Embedded API key** (`security.rs: MALWAREBAZAAR_AUTH_KEY`) — eliminates 401 errors on startup
- `Auth-Key` header sent with all MalwareBazaar API requests
- 401 responses downgraded from `warn!` to `debug!` (ignition.rs + security.rs)

#### Dashboard Dynamic Config
- **Dynamic API key from Tauri command** — `ignite_swarm` returns `dashboard_api_key` + `gateway_port` in JSON response
- **`getDashboardConfig()` helper** in `tauri.ts` — extracts key and port from Tauri command
- **Dynamic gateway port** — `_dynamicGatewayPort` set from ignition response, falls back to env var / 8080
- **Dynamic version from Tauri API** — `getAppVersion()` via `@tauri-apps/api/app`, always matches `tauri.conf.json`

#### CLI Companion Hardening
- **React Error Boundary** wrapping entire CLI app — catches render errors, shows diagnostic instead of blank screen
- **`isTauriAvailable()` check** — graceful degradation when Tauri API not ready
- Version synced to 0.3.3

#### Test Runner
- **`test.bat`** — CPU-limited test runner with `--test-threads=2` and `CARGO_BUILD_JOBS=2`

### Changed

#### Version Sync
- **All 28 workspace crate Cargo.toml** bumped from 0.3.2 to 0.3.3
- **CLI Companion tauri.conf.json** bumped from 0.3.2 to 0.3.3
- **Desktop tauri.conf.json** — identifier changed from `com.savant.app` to `com.savant.desktop` (fixes macOS `.app` conflict warning)
- **Gateway API** now reports correct v0.3.3 via `/api/status` and `/health`

#### Hot-Reload Boot Suppression
- Cooldown timer initialized at watcher start (`watcher.rs`) — covers entire boot window
- Agent file writes during boot no longer trigger evacuation + restart

#### Model Info Fetch
- Only fetches from OpenRouter when provider is `OpenRouter` — skips for OpenGateway/Ollama/etc
- Eliminates empty `context=0, max_completion=0` results for non-OR models

#### Log Noise Reduction
- Blackboard stale cleanup: `info!` → `debug!` (expected on Windows)
- Blackboard/Collective/CapabilityRegistry retry: `warn!` → `debug!` (expected retry behavior)
- `OpenRouter stream chunk` label → `LLM stream chunk` (shared helper used by all providers)
- Auto-updater plugin disabled (no valid endpoint configured)

#### Agent Logs Window (`logs.html`)
- **Copy All** rewritten with `document.execCommand('copy')` as primary method (works in all webviews)
- Event listening uses `window.__TAURI__` with async retry for Tauri API injection
- Window launches maximized (removed off-screen x: -2560 offset)

#### Vector DB Lock Handling
- If `remove_dir_all` fails (locked by another process), returns clear error instead of retrying
- Error message: "Vector database locked by another process. Close other Savant instances."

### Fixed

- **Dashboard SWARM_OFFLINE** — root cause identified: `NEXT_PUBLIC_DASHBOARD_API_KEY` undefined (no `.env` file). Fix: dynamic key from Tauri command. Diagnostic logging added to debug console.
- **Copy All button** — Tauri clipboard plugin invoke path wrong, navigator.clipboard requires secure context. Fix: `document.execCommand('copy')`.
- **Hot-reload at boot** — agent boot writes triggered immediate evacuation. Fix: cooldown timer at watcher start.
- **Consciousness 401s** — no OpenGateway API key in codebase. Fix: 11-key rotation with defense-in-depth fallbacks.
- **MalwareBazaar 401** — no `Auth-Key` header sent. Fix: embedded key + header.
- **Model info empty** — OpenRouter fetch for non-OR models returned nothing. Fix: skip fetch for non-OR providers.
- **Blackboard noise** — stale cleanup and retry logs were `info!`/`warn!`. Fix: downgraded to `debug!`.
- **Release tracing** — key rotation invisible in release mode. Fix: `bootstrap_log` for key source after ignition.
- **Gateway port hardcoded** — dashboard always used port 8080. Fix: dynamic port from Tauri command.
- **Cargo.toml version drift** — all crates reported 0.3.2. Fix: bumped to 0.3.3.
- **Bundle identifier** — `com.savant.app` conflicted with macOS `.app`. Fix: `com.savant.desktop`.
- **Vector DB lock** — second launch crashed with unclear error. Fix: clear error message, no retry.
- **Logs window** — launched off-screen at x: -2560. Fix: maximized, restored x:-2560 for left-screen placement.
- **CLI blank screen** — unhandled render error. Fix: Error Boundary + Tauri API check.
- **Dashboard diagnostics** — `logger.info()` invisible in debug panel. Fix: push to `debugLogs` state directly.
- **Dashboard SWARM_OFFLINE** — tower-http CorsLayer rejects WS upgrades with non-matching Origin header before they reach the handler. Fix: WS routes in separate Router without CORS/auth/rate-limit middleware. WS has own auth (first-message auth frame) + connection limit.
- **CLI Companion fails to launch** — updater plugin with empty pubkey/endpoints causes Tauri v2 auto-init failure. Fix: removed updater plugin config.
- **Agent Logs Copy All** — `navigator.clipboard` unavailable in Tauri WebView, `document.execCommand` fails for off-screen elements. Fix: try Tauri clipboard plugin first (`plugin:clipboard|write_text`), then `navigator.clipboard`, then `execCommand` fallback with visible textarea.

### Verification

- `cargo check --workspace` — 0 errors
- `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings
- `npx tsc --noEmit` (dashboard) — 0 errors
- `npx tsc --noEmit` (CLI) — 0 errors

### Pending (requires live test)

- Dashboard SWARM_OFFLINE — WS routes bypass middleware, awaiting rebuild + test
- Copy All button — Tauri clipboard plugin tried first, awaiting verification
- MalwareBazaar sync — key embedded, awaiting verification
- CLI Companion launch — updater plugin removed, awaiting rebuild + test

---

## [0.3.2] - 2026-05-26

**Complete Implementation Sprint. 27 FIDs closed. 186 items addressed. 1,197 tests pass. Zero open issues. Release ready.**

Full security hardening, provider chain resilience, consciousness layer, resource governor, LLM-driven skill synthesis, skill chaining, and comprehensive documentation overhaul. Includes post-release hotfix patch for security, concurrency, and functional wiring.

### Hotfix Patch (2026-05-26)

#### Security
- Fixed constant-time comparison timing leak in auth middleware — removed length-based early exit, always iterates over expected key length with XOR accumulator

#### Concurrency & Stability
- Resolved non-atomic `AdaptiveSemaphore` permit adjustment race condition — `Mutex<()>` guard serializes read-modify-write cycle
- Replaced silent `try_lock()` drop in `SwarmGovernor.defer_agent` with `.lock().await` backpressure + `tracing::warn!` observability
- `ConsciousnessBudget` auto-resets hourly/daily counters via `Instant`-based timing — daemon no longer goes permanently dormant after first budget exhaustion
- Budget base values stored as fields to prevent multiplier drift

#### Functional Wiring
- Wired `WonderEngine` exploration loop — LLM routing via `stream_completion()`, temperature sampling, reward threshold evaluation
- Removed `#[allow(dead_code)]` on `exploration_temperature` and `reward_threshold`
- Added `ConsciousnessBudget::with_quiet_hours()` and `always_active()` constructors
- Quiet hours configurable via `[evolution].quiet_hours_start` / `quiet_hours_end` in savant.toml
- Default quiet hours: 3AM–11AM UTC (11PM–7AM EDT)

#### Accuracy & Safety
- Improved token estimation — `chars().count()` instead of `len() / 4` for Unicode-safe counting in both `NarrativeSynthesizer` and `ProviderChain`
- Fixed UTF-8 truncation panic in consciousness daemon — `is_char_boundary()` for safe split point
- `EntropyCalculator` uses blake3 instead of `DefaultHasher` for collision resistance
- `AntiEchoChamber` word comparison now case-insensitive with `chars().count()` filter
- `ResourceMonitor` f64 values clamped to `>= 0.0` with `debug_assert!` on finiteness
- `Wondering` state naturally reached at entropy 0.10–0.25 (was only set manually)
- Configurable quiet hours (default 11PM–7AM EDT, was 6PM EDT)
- Desktop build: self-contained updater script in `src-tauri/scripts/`, fixed fragile relative paths

#### Infrastructure
- Tauri `beforeBuildCommand` uses local script path (no more `cd ../..` fragile navigation)
- 4 new consciousness budget tests (custom hours, disabled, overnight window, defaults)

### Added

#### Security Hardening

- **REST API authentication middleware** — Tower middleware with constant-time comparison, `Authorization: Bearer <key>` and `X-API-Key: <key>` support, public endpoint whitelist (`/health`, `/live`, `/ready`, `/ws`, `/ws/canvas`)
- **Immutable security fields** — `dashboard_api_key`, `host`, `port`, `signing_key`, `enable_blocklist_sync` cannot be changed at runtime via ConfigSet. Returns 403 Forbidden.
- **Canvas WebSocket authentication** — `/ws/canvas` requires API key on upgrade
- **Webhook auth token** — Random token generated on startup (blake3 hash of timestamp + UUID)
- **SoulUpdate size limit** — 100KB cap on `proposed_content` and `reasoning` fields
- **BulkManifest agent limit** — Max 10 agents per BulkManifest request
- **NLCommand input limit** — Max 10,000 characters per NLCommand
- **Environment variable filtering** — Shell commands use `env_clear()` + only pass `PATH`, `HOME`, `LANG`, `TERM`

#### Consciousness Layer

- **ConsciousnessState** — `Thinking`/`Idle`/`Dormant`/`Wondering` enum with entropy-based tick delay (0ms–300s)
- **EntropyCalculator** — Shannon entropy of hivemind state via hash-based change tracking
- **NarrativeSynthesizer** — LLM-driven Markov chain regeneration (bounded context, never accumulates)
- **WonderEngine** — Autonomous exploration during idle with environment sampling and reward-based pruning
- **AntiEchoChamber** — Jaccard similarity convergence detection across agent outputs
- **ConsciousnessBudget** — Token/cost budget with quiet hours (10PM–7AM) and entropy-based scaling

#### Resource Governor

- **ResourceGovernorConfig** — `[resource_governor]` section in `savant.toml` with CPU/memory pressure thresholds and per-level agent limits
- **PressureLevel** — `Low`/`Medium`/`High`/`Critical` with worst-case-wins (max of CPU and memory)
- **ResourceMonitor** — Background polling with atomic state, watch channel for pressure change notifications
- **AdaptiveSemaphore** — Pressure-based permit adjustment with safe bounds checking
- **SwarmGovernor** — Orchestrator with deferred agent queue, max deferral retries (5 min), shutdown support

#### Provider Chain Resilience

- **True streaming** — Chunks yielded directly from provider (no collect-then-replay). Significantly improved TTFT.
- **Cross-provider fallback** — Fallback provider actually invoked when primary exhausts retries
- **Circuit breaker race condition fix** — Single `RwLock<CircuitBreakerInner>` replaces separate locks
- **Provider call timeout** — 120s default, configurable in `ChainConfig`. Timeouts are retryable errors.
- **RateLimiter wiring** — `with_rate_limiter()` builder, token estimation from messages, pre-call check

#### Agent Intelligence

- **CostAwareRouter** — Heuristic task complexity classification (Simple/Moderate/Complex) with cheap/expensive model routing
- **ProactiveContextGatherer** — Parallel memory + git log gathering before user asks

#### LLM Interaction Quality

- **Tool heuristics** — `when_to_use()` and `when_not_to_use()` methods on `Tool` trait
- **Soul example parser** — Parses `## Example Exchanges` from SOUL.md for few-shot prompting
- **Skill verifier** — `SkillVerifier` checks required files, file sizes, and runs `cargo check` on generated skills

#### Skill System

- **LLM-driven synthesis** — `SovereignSynthesizer` now uses LLM for code generation with template fallback
- **Self-healing loop** — Failed `cargo check` output fed back to LLM for error correction (max 3 attempts)
- **Pinned dependencies** — No more wildcard `"*"` versions. Known versions mapped.
- **Skill chaining** — `SkillManifest` now has `depends_on` and `chain_with` fields. `SkillChain` and `SkillChainStep` types for composition.

#### Context Quality

- **Multi-head scoring** — Semantic window scoring uses role weight, exponential recency, case-insensitive keyword overlap, and causal pair preservation (Q→A boost)
- **Memory recall deduplication** — Recalled memories injected into system prompt only (not duplicated in conversation history)
- **Tool result governance** — 50,000 character cap on tool output with truncation notice

#### Learning System Safety (Phase 1)

- **Content-hash dedup** — Rolling 10K entry window prevents duplicate learnings
- **Per-entry length cap** — 2000 characters max per learning entry
- **LEARNINGS.md rotation** — Archives at 100KB to `LEARNINGS-ARCHIVE-{timestamp}.md`
- **Trigger-path tagging** — Every entry tagged with source (e.g., `memory_store`)
- **Filtered content logging** — Rejected entries logged to `FILTERED.jsonl` for human review

#### Dashboard APIs

- `GET /api/memory/search` — Memory search with query and limit params
- `GET /api/governor/status` — Resource governor pressure and metrics
- `GET /api/consciousness/status` — Consciousness daemon state and entropy
- `POST /api/chat` — REST API for sending messages (alternative to WebSocket)

#### CI/CD Pipeline

- `.github/workflows/ci.yml` — Core CI: check, clippy, test, fmt on push/PR
- `.github/workflows/dashboard.yml` — Dashboard CI: tsc, build on dashboard changes
- `.github/workflows/release.yml` — 3-platform release (Linux, Windows, macOS) on tag push

#### Observability

- **Request ID middleware** — UUID per request, `X-Request-Id` header in all responses
- **Structured health check** — `/health` returns `{ status: "healthy", version, timestamp }`
- **OpenTelemetry deps** — `tracing-opentelemetry`, `opentelemetry`, `opentelemetry_sdk` added to CLI

#### Production Hardening

- **WebSocket connection limit** — Max 100 concurrent connections, 429 Too Many Requests on overflow
- **Consciousness daemon wiring** — Spawned as tokio task in `SwarmController::ignite()`
- **Resource governor wiring** — `spawn_agent()` checks pressure, defers agents when no permits
- **ProviderChain runtime** — All providers wrapped in ProviderChain (circuit breaker, timeout, rate limiter active)

#### Skill Chain Execution

- **SkillChainExecutor** — Sequential step execution with conditional execution, output passing, error handling
- **5 tests** — empty chain, missing tool, condition skip, max steps, condition evaluation

#### Grounding Reform

- **GroundingScore** — `OutputFilter::score()` returns weighted environmental (1.0) + introspective (0.6) scores
- **Fabrication blocking** — Hard block returns total=0.0

#### Contributing

- **CONTRIBUTING.md** — Setup, code style, checks, PR process, commit messages, security disclosure

#### Tests

- **1,193 tests** across all crates (up from 770+, recovered 39 previously crashing memory tests)
- **IPC tests** — 10 new tests (GlobalState, AgentEntry, SwarmSharedContext, consensus, blackboard)
- **CLI tests** — 4 new tests (argument parsing)
- **Skill chain tests** — 5 new tests
- **Memory test fix** — `test_lsm_engine_basic_operations` and `test_engine` use 64-dim vectors instead of 2560 (prevents 332MB allocation crash on Windows)

### Changed

- **Mandatory SecurityScanner** — `SovereignShell` scanner field changed from `Option<Arc<SecurityScanner>>` to `Arc<SecurityScanner>`. No more optional bypass.
- **Provider chain** — `ProviderChain` now uses `Arc<dyn LlmProvider>` instead of `Box<dyn LlmProvider>`. `RetryProvider` dead wrapper removed.
- **Ollama embedding service** — Graceful degradation: auto-start, model check, test embed. Returns clear error with setup instructions instead of crashing.
- **Config loading** — Corrupt config falls back to `Config::default()` with warning log instead of crashing.
- **SovereignSynthesizer** — Replaced template-only selection with LLM-driven synthesis. Kani proofs replaced with `cargo check` verification.
- **Tool trait** — Added `when_to_use()` and `when_not_to_use()` methods for LLM guidance.
- **Memory system docs** — `docs/memory.md` updated with Glass House, Reflective Memory, BM25, Procedures, Lessons, Insights, Multimodal, Audit, Notifications sections.
- **Hivemind docs** — `docs/swarm.md` completely rewritten to reflect actual code architecture (replaced 1476 lines of theoretical content).
- **Collective intelligence docs** — `docs/collective_intelligence.md` rewritten with enterprise-quality architecture documentation.
- **Version bumped** — All 23 crate Cargo.toml files bumped from 0.3.1 to 0.3.2

### Fixed

- **Circuit breaker race condition** — `CircuitBreaker` now uses single `RwLock<CircuitBreakerInner>` instead of separate locks for state and failure count
- **Tool panic isolation** — Side-effect tool execution in `HyperCausalEngine` wrapped in `tokio::spawn` to catch panics
- **Mid-stream timeout retry** — Stream errors trigger 2s wait + 1 retry before failing
- **Hardcoded API key** — Removed hardcoded OpenGateway key from `swarm.rs`
- **Local provider routing** — LMStudio/Perplexity/Local now log warning when routing through OpenRouter
- **Memory recall duplication** — Recalled memories no longer appear in both system prompt and conversation history
- **Config immutable fields** — Security-critical config fields blocked from runtime mutation
- **Clippy fixes** — 2 platform-specific unused import fixes (sandbox/agentd.rs, agent/synthesis.rs)
- **Cargo warnings** — Removed Tauri profile.release duplicate, fixed reqwest default-features
- **Memory test allocation** — Fixed 332MB/288MB allocation crash in memory tests by using 64-dim vectors instead of 2560 in test configs (MockEmbeddingProvider, LsmConfig, VectorConfig)

### Documentation (0.3.2)

- **CONVENTIONS.md** — 621 lines covering all 12 undocumented codebase patterns
- **Evolution system guide** — `docs/evolution/evolution-system.md` user guide for the evolution pipeline
- **Config reference** — `docs/config/CONFIG.md` updated with all new config sections
- **API reference** — `docs/api/README.md` updated with new endpoints, auth, immutable fields, size limits
- **47 outdated docs** archived to `docs/archive/`
- **All FID tracking** updated and archived (76 total FIDs, 0 active)

---

## [0.3.1] - 2026-05-17

**Full Workspace Audit Remediation. 11 FIDs closed. 180+ issues fixed across 20 crates. Zero unwrap/expect in production code.**

### Security Audit Remediation (`savant_security`)

- Removed crate-level `#[allow(clippy::disallowed_methods)]` — replaced with per-test-module allows
- Fixed 4x `.expect()` on clock errors (token.rs, enclave.rs) — replaced with checked arithmetic returning `Result`
- Fixed non-ASCII text lowercasing index mismatch panic in `prompt_defense.rs`
- Zero credential bytes on revoke in `continuous/credentials.rs`
- 32/32 tests pass, 0 clippy warnings

### Core Audit Remediation (`savant_core`)

- Fixed infinite recursion in `EmbeddingProvider::dimensions()` and `OllamaEmbeddingService::dimensions()` (trait method called itself instead of inherent impl)
- Fixed batch cache index out of bounds in `embeddings.rs`
- Changed `.expect()` to `Result` return in `secure_client()` + added `secure_client_fallible()` for backward compat
- Added Unix file permissions (`0o600`) for `.env` file writes
- Replaced `DefaultHasher` with `blake3::Hasher` in `session.rs`
- Fixed corrupted `agent.json` silently returning defaults — now returns `Err`
- Removed dead `storage/mod.rs` file
- 53/53 tests pass, 0 clippy warnings

### Agent Audit Remediation (`savant_agent`) — 29 Issues Fixed

- Removed blanket `#[allow(clippy::disallowed_methods)]` from lib.rs — replaced with targeted allow for `serde_json::json!` macro
- Fixed speculative horizon instruction discarded (`react_speculative.rs`)
- Fixed JSON parse failures silently mangling tool args (`reactor.rs`, `stream.rs`) — now propagates errors to LLM for retry
- Replaced `unwrap_or_default()` with proper error propagation (`ald.rs`, `heartbeat.rs`)
- Removed master key fallback on key creation failure (`swarm.rs`) — security hardening
- Added validation logging for missing heartbeat payload fields (`heartbeat.rs`)
- Fixed fallback parser to extract real action names (`stream.rs`)
- Changed `WebSovereign::new()` to return `Result` instead of panicking (`web.rs`)
- 168 tests pass (120 unit + 44 a2a + 3 production + 1 doc-test), 0 clippy warnings

### Stub/Abandoned Implementation Remediation — 38 Issues Across 20 Crates

- `cull_low_entropy_memories()` — implemented entropy-based culling (was returning `Ok(0)`)
- `promote_to_agents()` — writes sanitized content to AGENTS.md (was no-op)
- `HeartbeatTool` — returns structured JSON with evaluation (was uppercasing action)
- `EvaluateNotificationTool` — evaluates urgency, quiet hours, anomalies (was returning static "Acknowledged")
- `run_promotion_cycle()` — archives low-scoring, reinforces high-scoring entries (was scoring but never acting)
- 9 `ChannelAdapter` trait methods — wired to nexus event bus (were returning `Ok(())`)
- `VaultWatcher` — wired enclave for semantic memory storage (was `#[allow(dead_code)]`)
- 5 send-only channels — added inbound `handle_event` routing
- `delta_rx` — connected to DreamScheduler (was discarded)
- `SyncScheduler` — added graceful shutdown via `watch` channel (was infinite loop)
- Echo watcher — replaced 3600s CPU-wasting sleep with channel recv
- `LambdaInvokeRequest/Response` — used in actual invocation (were dead code)
- `MemoryLayer` enum — used in promotion cycle for layer distribution tracking
- Gateway API handlers — return proper `serde_json::Value` responses
- `JsonRpcError` — fields used in error formatting
- `CircuitBreaker::start_time` — used for timeout detection
- `browser_get_tabs` — queries Chrome DevTools Protocol for real tab list
- `ColdStorageManager::_writer` — used to ensure vault structure
- Duplicate `bootstrap_log` call removed
- `voice` module — fully implemented with nexus event bus routing
- Commented-out `// pub mod memory;` removed
- Email `send_event` — logs dropped events
- `fetch_deduped_tail()` helper extracted (deduplication for semantic search + Ollama retry)
- `iter_all_messages()` — added `limit` param to bound memory usage
- FNV-1a hash replaced with `std::hash::DefaultHasher`
- TOCTOU race fixed in `get_or_create_session_state()`
- Entity extraction now includes keyword word itself in capture
- Dead code removed (`find_by_key`, unused watchdog)

### Memory Audit Remediation (`savant_memory`)

- 7/8 issues fixed (1 by-design: reflective in-memory confirmed as Obsidian vault responsibility)
- Replaced FNV-1a hash with `std::hash::DefaultHasher`
- Fixed TOCTOU race in session state creation
- Fixed entity extraction to include keyword in capture
- Deduplicated code with `fetch_deduped_tail()` helper
- Added pagination to `iter_all_messages()`
- Removed dead `find_by_key` function
- 82/82 tests pass

### Gateway Audit Remediation (`savant_gateway`)

- Fixed all `.expect()` panic risks with `fallback_response()`
- Monolithic handler split is tech debt (all code is functional)
- 19/19 tests pass

### Other Crate Remediations

- Cognitive: Removed blanket allow, fixed 5 `.unwrap()` in production code
- Dream: Removed blanket allow (no production unwraps found)
- IPC: No issues found
- Obsidian/Desktop/CLI: No issues found

### Workspace-Wide Verification

- `cargo check --workspace` — 0 errors, 0 warnings
- `cargo test --workspace` — 0 failures (all crates)
- `cargo clippy --all-targets -- -D warnings` — 0 warnings
- `cargo fmt --check` — clean
- Zero `.unwrap()` / `.expect()` / `todo!()` / `unimplemented!()` in non-test code
- Zero `#[allow(dead_code)]` annotations (excluding test-only code)
- Zero functions returning `Ok(0)` or `Ok(())` without side effects

---

## [0.3.0] - 2026-05-15

**Gemma 4 Model System. A2A Communication Layer. Glass House Obsidian Sync. Personality Evolution. Continuous Consciousness. Full Workspace Clippy Cleanup. 265 files changed.**

### Gemma 4 Model System — Default Vision + Embedding Engine

Gemma 4 is the default local model for the entire framework. Even when the user selects a different primary chat model, Gemma 4 automatically handles vision and embeddings if the primary model doesn't support them.

- **Default Model** — `gemma4` configured as default for chat (`ai.model`), vision (`browser.vision_model`), embeddings (`browser.embedding_model`), and manifestation (`ai.manifestation_model`)
- **Vision Fallback** — If the user's selected chat model doesn't support vision (not in the known vision patterns list: gemma4, qwen3-vl, llava, bakllava, moondream, minicpm-v, phi-3-vision, pixtral, internvl, idefics, florence, mistral-3, cogvlm, deepseek-vl), the system automatically falls back to the configured vision model (default: gemma4) for image understanding
- **Embedding Fallback** — Same pattern for text embeddings. If the primary model doesn't support embeddings, Gemma 4 handles it
- **On-Demand Loading** — Vision model loads via Ollama on use, unloads immediately after (`keep_alive: 0`) to minimize CPU/memory consumption
- **Embedding Service** — Ollama is REQUIRED (no silent fallback). Auto-starts Ollama if not running. Auto-pulls the embedding model if missing. Hard error with installation instructions if Ollama is unavailable
- **Free Model Router** — Cloud fallback via `openrouter/free` when no local model is available. `FreeModelRouter` with model validation, dashboard model list, and rotation strategy
- **NLP Model Commands** — Natural language model switching: "switch to gemma4", "use gemma4:e4b", etc. Supports all Gemma 4 variants (e2b, e4b, 26b, 31b)
- **Setup Wizard** — First-launch wizard that:
  - Detects hardware (RAM, CPU cores, GPU via WebGL)
  - Recommends optimal Gemma 4 variant based on available resources
  - Shows all 4 variants with VRAM requirements and descriptions
  - Checks Ollama status and model availability
  - Pulls the selected model via Ollama API
  - Configures vision_model, embedding_model, and vault_path in savant.toml
  - Can be skipped and configured later in Settings
- **Settings UI** — Dashboard settings page with vision_model and embedding_model inputs
- **Config API** — `POST /api/config/set` for runtime config updates (browser, obsidian, ai sections)
- **Setup API** — `GET /api/setup/check` (Ollama + model status), `POST /api/setup/install-model` (pull model)

### A2A Communication Layer — Typed Agent-to-Agent Delegation Protocol

Replaces fragile text-based LLM command parsing (`/subagents spawn`) with a fully typed, structured delegation protocol.

- **A2A Protocol Types** (`savant_ipc::a2a`) — `A2AEnvelope`, `DelegationTask`, `TaskState` (Submitted → Working → InputRequired → Completed → Failed → Canceled), `Artifact`, `ArtifactPart`, `AgentCard`, `ContextPackage`, `ResultRouter` — all `#[repr(C)]`, rkyv-serialized for zero-copy shared memory
- **AgentCard Registry** — Machine-readable capability manifests in iceoryx2 blackboard. Semantic capability matching via keyword scoring. Dynamic pressure tracking for load balancing
- **ContextPackage** — Memory-aware context passing. References CortexaDB collection keys instead of raw text. Fixed-size arrays for `#[repr(C)]` compatibility
- **TaskState WAL Journaling** — All task state transitions persisted to CortexaDB WAL with CRC32 checksums. `recover_interrupted_delegations()` re-hydrates interrupted tasks on restart
- **Delegation Lifecycle** — `delegate_task()` wired into `execute_turn()` via `DELEGATE:` prefix detection. CCT token minting for authorization. Cancellation cascade through nested subagents
- **Cross-Agent Speculative Execution** — `execute_cross_agent_speculative()` delegates to multiple agents, selects best artifact via informational entropy scoring
- **Consensus Integration** — `requires_consensus` flag on `DelegationTask` triggers swarm voting for destructive operations
- **Task Timeout Detection** — `is_task_expired()` checks `deadline_timestamp` during continuation. `TaskExpired` error variant
- **ResultRouter** — Per-agent result channels via iceoryx2 request-response. Parent polls for `ArtifactDelivery`
- **AgentCardCopy Fix** — Uses `size_of::<AgentCard>()` at compile time instead of hardcoded 192, eliminating buffer over-read/overflow UB
- **44 Integration Tests** — Full delegation cycle, AgentCard matching, ContextPackage extraction, WAL journaling, cancellation, consensus, speculative execution

### Glass House — Obsidian Bidirectional Sync System (New Crate: `savant_obsidian`)

Full memory-to-vault projection system with bidirectional sync. The agent's entire memory system is rendered as structured Obsidian markdown, and user edits in Obsidian feed back into agent memory.

- **VaultWriter** (40KB) — Projects all memory graphs into Obsidian vault structure:
  - `Episodic/` — Daily markdown files with wiki-links, frontmatter, Mermaid timeline graphs
  - `Semantic/` — Concept nodes with relation edges, importance scores, decay status
  - `Identity/Evolution/` — SOUL.md (protected), Personality.md (OCEAN model), evolution changelog
  - `Themes/` — Auto-detected themes across episodic memories
  - `Working/` — Active task context, recent tool outputs
  - `Dashboard/` — Real-time stats, agent status, memory metrics
  - `Delegation/` — Structured markdown artifacts from A2A task results
  - Index page with full vault stats and navigation
- **VaultWatcher** (12KB) — File system watcher with edit classification:
  - `Episodic/*.md` → Rejected (past cannot be rewritten). Edits logged as Correction nodes
  - `Semantic/*.md` → Accepted as Ground Truth Override. Parsed into LSM metadata
  - `Identity/SOUL.md` → Blocked. Must go through Evolution system
  - `Identity/Personality.md` → Accepted. OCEAN values parsed and forwarded
  - New files → Quarantine. Extracted entities require user validation
  - File deletions → Tombstone pattern (marked hidden in DB, not recreated)
  - All edits pass through `scan_prompt()` injection defense before affecting agent state
- **OutboxWorker** (6KB) — Cursor-based projection trigger. Polls LSM for state changes, triggers full projection only when memory state differs from last-projected cursor. Atomic file writes via tempfile+rename
- **ColdStorageManager** (7KB) — Enforces Obsidian file ceiling (~15K files, Graph View crashes at ~100K). Migrates episodic content older than configurable days to LSM-only retention. Retains data immutably in database, removes from vault
- **Injection Defense** — All vault edits scanned via `scan_prompt()` before affecting agent state. Vault treated as potentially hostile data source

### Personality Evolution Architecture

- **ALD (Autonomous Learning & Distillation)** — Identity signal processing pipeline. Extracts identity signals from conversation history, proposes SOUL.md mutations
- **Evolution Cooldown** — No proposals targeting same section during cooldown period
- **Immutable Section Locking** — Core Laws protected from mutation
- **EVOLUTION.jsonl** — Append-only audit log of all evolution events
- **Nexus Event Emission** — Real-time dashboard updates on evolution events

### Continuous Consciousness

- **Self-Referential Heartbeat Feedback Loop** — Deterministic stillness detection via content hashing. Skips inference when substrate state is identical
- **Forced Reflection** — Triggers when state is unchanged but reflection interval exceeded
- **DeltaTracker** — Environmental change detection (git diff, filesystem snapshot, message count, tool error count)

### New Crates

- `savant_obsidian` — Glass House bidirectional sync (writer, watcher, outbox, cold storage)
- `savant_toolforge` — Tool creation with quality gates, provenance tracking, registry
- `savant_integrations` — External service connectors (Gmail, Notion) with sync scheduler and state tracking

### Dashboard & UI (0.3.0)

- **Evolution Pages** — `/evolution` and `/evolution/behind-the-curtain` pages for viewing personality evolution
- **Enhanced Settings** — Provider configuration with validation, vision/embedding model inputs
- **Health Monitoring** — `/health` page for system status
- **Browser Panel** — Refactored from monolithic component to modular architecture
- **Setup Wizard** — Redesigned with proper CSS module styling, hardware detection, model selection

### Full Workspace Clippy Cleanup

- Fixed 500+ clippy warnings across 24 crates (100+ files)
- Zero `unwrap()`/`expect()` in production code (enforced by `clippy.toml` `disallowed-methods`)
- All `map_or(false, ...)` → `is_some_and()`, `map_or(true, ...)` → `is_none_or()`
- All manual `Range::contains()` → `(start..end).contains(&value)`
- All manual prefix stripping → `strip_prefix()`
- All redundant closures `|e| Error(e)` → `Error`
- All needless borrows `&format!(...)` → `format!(...)`
- All `impl Default` that could be `#[derive(Default)]`
- All `match_single_binding` → `let` destructuring
- All `assert!(true)` → meaningful assertions or removed
- All unused imports removed
- Added `#[allow(clippy::disallowed_methods)]` to test modules and `json!`-heavy files

### Root Directory Cleanup

- Moved misplaced docs to `docs/plans/`, `docs/research/`, `docs/evolution/`
- Moved stale code to `dev/archive/`
- Moved session transcripts to `dev/archive/`
- Moved audit report to `docs/archive/`
- Removed `firebase-debug.log`
- Archived completed FIDs to `dev/fids/archive/`

### Verification

- `cargo check --workspace` — 0 errors, 0 warnings
- `cargo clippy --all-targets -- -D warnings` — 0 errors, 0 warnings
- `cargo test --workspace` — all tests pass (0 failures)
- `npx tsc --noEmit` — frontend compiles clean
- `cargo fmt --check` — clean (only external cortexadb lib has issues)

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

### Dashboard & UI (0.1.1)

- **Frontend chat fix** — role casing corrected (`'User'`/`'Assistant'` → `'user'`/`'assistant'`) to match Rust serde expectations.
- **Gateway error logging** — WebSocket deserialization failures now logged with error message and payload preview.
- **Fine tuner settings sync** — LLM parameters (temperature, top_p, frequency_penalty, presence_penalty) now sync to agent.json. Dashboard and backend always aligned.
- **Console window fix** — tracing subscriber stderr suppressed in release builds. No blank console window on Windows.
- **Agent logs window** — fixed TypeScript syntax error in logs.html, added logs window to Tauri capabilities.
- **LLM tuning** — companion-first parameters: temperature 0.85, top_p 0.92, frequency_penalty 0.6, presence_penalty 0.2.

### Documentation (0.1.1)

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
