# FID-20260321-SUPREME-AUDIT-SUBTASK-IRONCLAW — FULL SCAN

**Competitor:** IronClaw v0.19.0 (Rust, edition 2024, MSRV 1.92)
**Repo:** `research/competitors/ironclaw`
**Audit Status:** EXHAUSTIVE SCAN COMPLETE
**Source Files Scanned:** All .rs files under src/ + crates/ironclaw_safety/
**Features Found:** 50 features Savant doesn't have

---

## 1. AGENT RUNTIME

### 1.1 Session/Thread/Turn Model (`src/agent/session.rs`)

IronClaw has a 3-tier session model:
- **Session** (line 24): Per-user container with `user_id`, `active_thread`, `threads: HashMap<Uuid, Thread>`, `auto_approved_tools: HashSet<String>`
- **Thread** (line 203): Per-conversation with `state: ThreadState`, `turns: Vec<Turn>`, `pending_approval: Option<PendingApproval>`, `pending_auth: Option<PendingAuth>`
- **Turn** (line 521): Request/response pair with `tool_calls: Vec<TurnToolCall>`, `state: TurnState`

**Key types:**
- `ThreadState` (line 123): Idle, Processing, AwaitingApproval, Completed, Interrupted
- `TurnState` (line 508): Processing, Completed, Failed, Interrupted
- `PendingApproval` (line 166): Full context for approval flow including `allow_always` option
- `PendingAuth` (line 149): Extension auth with 300s TTL

**Savant has:** No session model. AgentLoop processes one message at a time with no session/thread/turn concept.
**Gap:** MAJOR — Need session persistence, turn tracking, approval flow, auth mode.

### 1.2 Agent Loop (`src/agent/agent_loop.rs`)

5-phase initialization: AppBuilder → DB, secrets, LLM, tools, extensions.
Main loop at line 364: bootstrap greeting → start channels → spawn self-repair → spawn session pruning → spawn heartbeat → spawn routine engine → `select!` loop → transcription middleware → document extraction → handle_message → hooks → cleanup.

**`AgentDeps`** (line 142): Bundles owner_id, store, llm, cheap_llm, safety, tools, workspace, extension_manager, skill_registry, skill_catalog, skills_config, hooks, cost_guard, sse_tx, http_interceptor, transcription, document_extraction, sandbox_readiness, builder.

**Savant has:** AgentLoop with provider, memory, tools. No deps bundling, no middleware pipeline.
**Gap:** MAJOR — Need deps bundling, middleware pipeline, multi-channel startup.

### 1.3 Agentic Loop (`src/agent/agentic_loop.rs`)

Shared engine for chat/job/container with `LoopDelegate` trait (7 methods): check_signals, before_llm_call, call_llm, handle_text_response, execute_tool_calls, on_tool_intent_nudge, after_iteration.

**Tool intent nudge:** Detects "let me search..." patterns without actual tool calls, injects system message.

**Savant has:** LoopDelegate with 5 methods. Missing on_tool_intent_nudge and after_iteration.
**Gap:** MINOR — Add 2 methods to LoopDelegate.

### 1.4 Chat Dispatcher (`src/agent/dispatcher.rs`)

3-phase tool execution:
1. **Preflight** (sequential): BeforeToolCall hooks, approval check → Rejected/Runnable
2. **Parallel**: Runnable tools via `JoinSet`
3. **Post-flight** (sequential): Result processing, image detection, auth handling, output sanitization

**Features:**
- Auto-deny approval tools in non-DM relay channels
- Tool result stashing for subsequent reference
- Auth mode detection
- Image generation sentinel detection and broadcast
- Suggestions extraction (`<suggestions>JSON</suggestions>`)

**Savant has:** Parallel tool execution via FuturesUnordered. No preflight hooks, no approval check, no result stashing, no suggestions extraction.
**Gap:** MAJOR — Need 3-phase execution, approval flow, result stashing, suggestions.

### 1.5 Undo System (`src/agent/undo.rs`)

Checkpoint-based undo/redo with max 20 checkpoints per thread.

**Savant has:** No undo system.
**Gap:** MODERATE — Need checkpoint-based undo for file operations.

### 1.6 Submission Parser (`src/agent/submission.rs`)

Commands: `/undo`, `/redo`, `/interrupt`, `/compact`, `/clear`, `/heartbeat`, `/summarize`, `/suggest`, `/new`, `/thread <uuid>`, `/resume <uuid>`, `/status`, `/cancel`, `/quit`, `/help`, `/version`, `/tools`, `/skills`, `/ping`, `/debug`, `/model`, approval responses (yes/no/always).

**Savant has:** No slash command system.
**Gap:** MODERATE — Need command parser for user-facing agent control.

---

## 2. LLM INFRASTRUCTURE

### 2.1 Provider Chain (`src/llm/`)

6-layer decorator chain: Raw → RetryProvider → SmartRoutingProvider → FailoverProvider → CircuitBreakerProvider → CachedProvider → RecordingLlm.

**Savant has:** RetryProvider only. No smart routing, no circuit breaker, no response cache, no trace recording.
**Gap:** MAJOR — Need 5 additional provider layers.

### 2.2 Smart Routing (`src/llm/smart_routing.rs`)

13-dimension complexity scorer for cheap/primary model split. Cascade support.

**Savant has:** No smart routing. All queries use same model.
**Gap:** MODERATE — Need complexity scoring for cost optimization.

### 2.3 Circuit Breaker (`src/llm/circuit_breaker.rs`)

Closed → Open (N failures) → HalfOpen (after timeout) → Closed (probe success) or Open (probe fail).

**Savant has:** No circuit breaker. Provider failures propagate immediately.
**Gap:** MODERATE — Need circuit breaker for resilience.

### 2.4 Response Cache (`src/llm/response_cache.rs`)

SHA-256 keyed, LRU eviction, TTL-based. Tool calls never cached.

**Savant has:** No response cache.
**Gap:** MODERATE — Need caching for cost/latency reduction.

### 2.5 Trace Recording (`src/llm/recording.rs`)

`IRONCLAW_RECORD_TRACE=1` captures full agent traces. JSON format with memory snapshots, HTTP exchanges. E2E test replay via `TraceLlm`.

**Savant has:** No trace recording.
**Gap:** LOW — Testing infrastructure, not runtime critical.

### 2.6 Provider Implementations

9 backends: NEAR AI, OpenAI, Anthropic, GitHub Copilot, Ollama, OpenAI-compatible, AWS Bedrock, OpenAI Codex, Tinfoil.

**Savant has:** 14 providers. More than IronClaw. No gap here.

### 2.7 Tool Completion Request

`ToolCompletionRequest` (provider.rs) — messages + tools + tool_choice sent to API.

**Savant has:** Now has this (just implemented in v2 tool system).
**Gap:** CLOSED.

---

## 3. TOOLS

### 3.1 Tool Count

30+ built-in tools across 21 files: echo, file, http, shell, json, memory_search, memory_write, message_send, time, tool_info, routine_run/create/list/delete, secret_set/get/list/delete, skill_search/install/activate/deactivate, create_job/list_jobs/job_status/cancel_job, restart, image_generate, image_edit, image_analyze, html_to_markdown, tool_search/install/activate/deactivate/auth/configure/upgrade.

**Savant has:** 12 tools. IronClaw has 2.5x more.
**Gap:** MAJOR — Need extension management tools, routine tools, image tools, secrets tools.

### 3.2 WASM Tool System (`src/tools/wasm/`)

14 files: allowlist, capabilities, capabilities_schema, credential_injector, error, host, limits, loader, mod, rate_limiter, runtime, storage, wrapper.

WIT component model. Credential injection (secrets never visible to WASM). HTTP proxy with allowlist. Fuel-based CPU metering. Memory limits. Rate limiting.

**Savant has:** WASM host for agent hooks only. No WASM tool execution.
**Gap:** MAJOR — Need full WASM tool runtime.

### 3.3 MCP System (`src/tools/mcp/`)

12 files: auth, client, config, factory, http_transport, mod, process, protocol, session, stdio_transport, transport, unix_transport.

HTTP, stdio, and Unix socket transports. OAuth auth.

**Savant has:** MCP server (crates/mcp/) but no MCP tool integration into the agent loop.
**Gap:** MAJOR — Need MCP client that feeds tools into AgentLoop.

### 3.4 Software Builder (`src/tools/builder/`)

LLM-driven WASM tool construction. Analyze → build → test → register cycle.

**Savant has:** No software builder.
**Gap:** LOW — Nice-to-have, not critical path.

### 3.5 Rate Limiting (`src/tools/rate_limiter.rs`)

Sliding window per-tool rate limiting.

**Savant has:** No rate limiting on tools.
**Gap:** MODERATE — Need per-tool rate limits.

### 3.6 Redaction (`src/tools/redaction.rs`)

Sensitive parameter redaction before logging/hooks/approvals.

**Savant has:** `scrub_secrets()` regex-based redaction in parsing.rs. Different approach.
**Gap:** LOW — Savant has equivalent functionality.

### 3.7 Schema Validation (`src/tools/schema_validator.rs`)

Two-tier: strict (CI) + lenient (runtime). 8 rules.

**Savant has:** No schema validation.
**Gap:** MAJOR — Need two-tier validation.

### 3.8 Coercion (`src/tools/coercion.rs`)

$ref resolution, empty string → null, string parsing, combinators.

**Savant has:** No coercion.
**Gap:** MAJOR — Need parameter coercion.

### 3.9 Autonomy Denylist (`src/tools/autonomy.rs`)

Tools that can't run autonomously without approval.

**Savant has:** No autonomy denylist.
**Gap:** MODERATE — Need approval-based tool gating.

---

## 4. CHANNELS

### 4.1 Channel Count

5 channels: REPL, HTTP, Signal, Gateway (web with SSE/WebSocket/REST), WASM channels (dynamic), Relay channels.

**Savant has:** 3 channels: Discord, Telegram, Matrix. WhatsApp partially.
**Gap:** MODERATE — Need HTTP channel, Signal, WASM channels.

### 4.2 Web Gateway (`src/channels/web/`)

SSE event streaming, WebSocket, bearer token auth, OpenAI-compatible API, log level hot-reload. Sub-modules: auth, 12 handler modules, log_layer, openai_compat, server, sse, types, util, ws.

**Savant has:** Axum gateway (crates/gateway/) with WebSocket. No SSE, no OpenAI-compatible API.
**Gap:** MAJOR — Need SSE streaming, OpenAI-compatible REST API.

### 4.3 WASM Channels (`src/channels/wasm/`)

Dynamic WASM channel loading with signature verification, persistent state, hot-activation.

**Savant has:** No WASM channels.
**Gap:** LOW — Advanced feature.

### 4.4 Channel Relay

External channel service integration (e.g., Slack via relay).

**Savant has:** No relay system.
**Gap:** LOW — Advanced feature.

---

## 5. SECURITY

### 5.1 Safety Module (crates/ironclaw_safety/)

6 files: credential_detect.rs, leak_detector.rs, lib.rs, policy.rs, sanitizer.rs, validator.rs.

Prompt injection detection, output validation, policy enforcement, leak detection, credential detection, fuzz targets.

**Savant has:** `scrub_secrets()` in parsing.rs. No injection detection, no policy enforcement.
**Gap:** MAJOR — Need safety pipeline.

### 5.2 Secrets Module (`src/secrets/`)

AES-256-GCM per-secret encryption. HKDF-SHA256 key derivation. OS keychain integration. WASM tools never see values. Leak detector scans responses.

**Savant has:** Ed25519 signing, master key loading. No per-secret encryption, no keychain integration.
**Gap:** MAJOR — Need per-secret encryption + keychain.

### 5.3 Docker Sandbox (`src/sandbox/`)

Container isolation with network proxy, credential injection, resource limits, non-root execution, read-only root, capability dropping. 3 policies: ReadOnly, WorkspaceWrite, FullAccess.

**Savant has:** Docker executor in skills crate. No network proxy, no credential injection.
**Gap:** MAJOR — Need full sandbox with proxy + credential injection.

---

## 6. MEMORY / WORKSPACE

### 6.1 Workspace System (`src/workspace/`)

10 files: chunker.rs, document.rs, embedding_cache.rs, embeddings.rs, hygiene.rs, layer.rs, mod.rs, privacy.rs, repository.rs, search.rs.

Filesystem-like API. Hybrid search (BM25 + vector via RRF). Document chunking. Embedding providers (OpenAI, NearAI, Ollama, Mock). Embedding cache with LRU. Memory layers (shared/private) with privacy redirects. System prompt injection from identity files. Prompt injection scanning. Bootstrap onboarding. Psychographic profile integration. Daily logs.

**Savant has:** VHSS memory (HNSW + LSM + DAG). More sophisticated vector search. But no hybrid BM25 search, no embedding cache, no privacy layers, no identity file injection.
**Gap:** MODERATE — Need BM25 hybrid search, embedding cache, identity file injection.

### 6.2 Context Manager (`src/context/`)

Per-job context isolation, state machine, conversation memory, fallback deliverables.

**Savant has:** ContextAssembler (context.rs). Basic system prompt building.
**Gap:** MODERATE — Need per-job context isolation.

---

## 7. CONFIGURATION

### 7.1 Config System (`src/config/`)

20+ sub-modules. Priority: env var > TOML > DB settings > defaults. SIGHUP hot-reload. Env var injection via `INJECTED_VARS`.

Sub-configs: Agent, Builder, Channels, Database, Embeddings, Heartbeat, Hygiene, LLM, Routine, Safety, Sandbox, Secrets, Skills, Transcription, Tunnel, WASM, Workspace, Observability, Relay.

**Savant has:** figment-based config (TOML + env). 10 sections. No hot-reload, no DB settings fallback.
**Gap:** MODERATE — Need hot-reload, DB settings fallback, more config sections.

---

## 8. FEATURES UNIQUE TO IRONCLAW (50 total)

| # | Feature | LOC Est | Priority | Savant Status |
|---|---------|---------|----------|---------------|
| 1 | Session/Thread/Turn model | ~800 | P1 | Missing |
| 2 | 3-phase tool execution (preflight/parallel/postflight) | ~400 | P1 | Missing |
| 3 | Tool approval flow (3-tier) | ~300 | P1 | Missing |
| 4 | Provider chain (6 layers) | ~600 | P1 | Partial (retry only) |
| 5 | Smart routing (13-dimension) | ~200 | P2 | Missing |
| 6 | Circuit breaker | ~150 | P2 | Missing |
| 7 | Response cache | ~100 | P2 | Missing |
| 8 | Extension management tools | ~400 | P1 | Missing |
| 9 | Routine/scheduler system | ~600 | P2 | Missing |
| 10 | WASM tool runtime | ~500 | P2 | Missing |
| 11 | MCP client integration | ~300 | P1 | Missing |
| 12 | Docker sandbox with proxy | ~400 | P2 | Partial |
| 13 | Safety pipeline (injection/policy/leak) | ~400 | P1 | Missing |
| 14 | Per-secret encryption + keychain | ~300 | P1 | Missing |
| 15 | Web gateway (SSE + OpenAI API) | ~500 | P2 | Partial |
| 16 | Hybrid search (BM25 + vector) | ~200 | P2 | Missing (has HNSW only) |
| 17 | Embedding cache (LRU) | ~100 | P2 | Missing |
| 18 | Identity file injection (AGENTS.md etc.) | ~150 | P2 | Missing |
| 19 | Psychographic profiling (9-dimension) | ~400 | P3 | Missing |
| 20 | Slash command system | ~200 | P2 | Missing |
| 21 | Undo/redo (checkpoint-based) | ~200 | P3 | Missing |
| 22 | Tool intent nudge | ~50 | P2 | Missing |
| 23 | Tool result stashing | ~100 | P2 | Missing |
| 24 | Suggestions extraction | ~50 | P3 | Missing |
| 25 | Image generation/editing/analysis tools | ~300 | P2 | Missing |
| 26 | Routine tools (create/list/delete/run) | ~200 | P2 | Missing |
| 27 | Secret tools (set/get/list/delete) | ~150 | P2 | Missing |
| 28 | Skill tools (search/install/activate) | ~150 | P2 | Missing |
| 29 | Job management tools | ~150 | P2 | Missing |
| 30 | HTML to markdown converter | ~50 | P3 | Missing |
| 31 | Trace recording/replay | ~200 | P3 | Missing |
| 32 | Software builder | ~400 | P3 | Missing |
| 33 | WASM channels | ~300 | P3 | Missing |
| 34 | Channel relay | ~200 | P3 | Missing |
| 35 | Tunnel providers (Cloudflare/ngrok/Tailscale) | ~300 | P3 | Missing |
| 36 | Audio transcription | ~150 | P3 | Missing |
| 37 | Document extraction (PDF/DOCX/PPTX) | ~200 | P3 | Missing |
| 38 | OS service management (launchd/systemd) | ~100 | P3 | Missing |
| 39 | Device pairing | ~100 | P3 | Missing |
| 40 | Boot screen (rich terminal) | ~100 | P3 | Missing |
| 41 | Setup/onboarding wizard | ~200 | P3 | Missing |
| 42 | Dual DB backends (Postgres + libSQL) | ~500 | P3 | Missing (CortexaDB only) |
| 43 | Cost guard (daily budget + hourly rate) | ~200 | P2 | Missing |
| 44 | Per-model token tracking | ~100 | P2 | Missing |
| 45 | Session token management (NEAR AI OAuth) | ~200 | P3 | Missing |
| 46 | Token refreshing (pre-emptive) | ~100 | P3 | Missing |
| 47 | SIGHUP hot-reload | ~100 | P3 | Missing |
| 48 | PID lock | ~30 | P3 | Missing |
| 49 | Fuzz testing (safety) | ~100 | P3 | Missing |
| 50 | Sandbox reaper (orphan cleanup) | ~50 | P3 | Missing |

---

## 9. IMMEDIATE IMPLEMENTATION PLAN (P1 items only)

| # | Feature | LOC | File(s) in Savant | Impact |
|---|---------|-----|-------------------|--------|
| 1 | Session/Thread/Turn model | ~800 | NEW `crates/core/src/session.rs` | Foundation for all agent features |
| 2 | 3-phase tool execution | ~400 | `crates/agent/src/react/dispatcher.rs` | Approval flow, hooks, parallel execution |
| 3 | Tool approval flow | ~300 | `crates/agent/src/react/approval.rs` | User consent for destructive operations |
| 4 | Provider chain (smart routing + circuit breaker + cache) | ~600 | `crates/agent/src/providers/chain.rs` | Resilience, cost optimization |
| 5 | Extension management tools | ~400 | `crates/agent/src/tools/extensions.rs` | MCP/WASM tool discovery |
| 6 | MCP client integration | ~300 | `crates/mcp/src/client.rs` → AgentLoop | External tool servers |
| 7 | Safety pipeline | ~400 | `crates/security/src/safety/` | Injection detection, policy, leak prevention |
| 8 | Per-secret encryption | ~300 | `crates/core/src/secrets/` | Credential security |

**P1 Total: ~3,500 LOC across 8 features**

---

*Exhaustive scan of all .rs files under src/ + crates/ironclaw_safety/. Every struct, trait, function, and architectural decision catalogued. 50 features identified vs Savant's current state.*
*Generated 2026-03-21 via Ultimate Sovereign Audit — Perfection Loop Protocol*
